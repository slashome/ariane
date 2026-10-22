//! The JSON API of `ariane serve`, consumed by the embedded front (ADR-0012).
//!
//! Every endpoint reuses what the commands already compute, so the interface and
//! the command line can never disagree: `/api/items` is `ariane tasks --all
//! --archived --dates --json` (`?kind=` and `?body=1` are `--kind` and
//! `--body`), `/api/show?ref=` is `ariane show --json`, `/api/schema` is
//! `ariane schema --json`, `/api/doctor` is `ariane doctor --all --json`. Responses are built under a short read lock on
//! the tree, released before anything is sent.
//!
//! The writing endpoints — `PATCH /api/items`, `PUT /api/dashboard`,
//! `POST /api/commit`, `POST /api/push`, `POST /api/discard` — run on the
//! blocking pool under the state's `busy` lock, one at a time, and never commit
//! unless asked.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::{Query, State as Shared};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use clap::ValueEnum;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_stream::wrappers::{BroadcastStream, WatchStream};
use tokio_stream::{Stream, StreamExt};

use super::State;
use super::write::{self, Patch, Refusal};
use super::{dashboard as pins, git};
use crate::item::Kind;
use crate::tree::Tree;

/// Interval between the keep-alive comments of an event stream.
const KEEP_ALIVE: Duration = Duration::from_secs(15);

/// Runs `build` on the current tree under a read lock and answers its JSON.
///
/// A poisoned lock (a thread panicked while writing) becomes a `500` rather
/// than taking the whole server down.
fn with_tree(state: &State, build: impl FnOnce(&Tree) -> Value) -> Response {
    match state.tree.read() {
        Ok(tree) => Json(build(&tree)).into_response(),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the tree lock is poisoned",
        ),
    }
}

/// A JSON error body, `{"error": "<message>"}`, with its status.
fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

/// The query of `GET /api/items`.
#[derive(Debug, Default, Deserialize)]
pub struct ItemsQuery {
    /// Comma-separated kinds, as `ariane tasks --kind`; every kind when absent.
    pub kind: Option<String>,
    /// `1` or `true` adds each item's body, as `ariane tasks --body`.
    pub body: Option<String>,
}

/// `GET /api/items`: every item of the tree, archived ones included, as the rows
/// of `ariane tasks --all --archived --dates --json`; `400` for an unknown kind.
pub async fn items(Shared(state): Shared<Arc<State>>, Query(query): Query<ItemsQuery>) -> Response {
    let kinds = match parse_kinds(query.kind.as_deref().unwrap_or_default()) {
        Ok(kinds) => kinds,
        Err(message) => return error(StatusCode::BAD_REQUEST, &message),
    };
    let body = matches!(query.body.as_deref(), Some("1" | "true"));
    let Ok(dates) = state.dates.read() else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the date lock is poisoned",
        );
    };
    with_tree(&state, |tree| {
        let selection = crate::tasks::Selection {
            archived: true,
            kinds: &kinds,
            body,
            dates: Some(&dates),
        };
        serde_json::to_value(crate::tasks::rows(tree, None, &selection)).unwrap_or(Value::Null)
    })
}

/// Reads `task,story` into kinds, with the words `ariane tasks --kind` takes.
fn parse_kinds(raw: &str) -> Result<Vec<Kind>, String> {
    raw.split(',')
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(|word| Kind::from_str(word, true).map_err(|_| format!("unknown kind `{word}`")))
        .collect()
}

/// The query of `GET /api/show`.
#[derive(Debug, Deserialize)]
pub struct ShowQuery {
    /// The item, `<node>#<id>`; a bare id is looked up in the whole tree.
    #[serde(rename = "ref")]
    pub reference: String,
}

/// `GET /api/show?ref=<node>%23<id>`: one item with its header, body and
/// relations, as `ariane show --json`; `404` when the reference names no
/// single item.
pub async fn show(Shared(state): Shared<Arc<State>>, Query(query): Query<ShowQuery>) -> Response {
    let Ok(dates) = state.dates.read() else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the date lock is poisoned",
        );
    };
    let Ok(tree) = state.tree.read() else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the tree lock is poisoned",
        );
    };
    match crate::show::find(&tree, &query.reference, None) {
        Ok(item) => Json(crate::show::shown(&tree, item, Some(&dates))).into_response(),
        Err(failure) => error(StatusCode::NOT_FOUND, &format!("{failure:#}")),
    }
}

/// `GET /api/nodes`: every node with its entry point, item count, effective
/// workflow and tracker binding (after `node.toml` inheritance, ADR-0008 and
/// ADR-0011) and the date of its latest commit, computed when the tree was
/// loaded. The tracker is the raw `[tracker]` table, or `null`.
pub async fn nodes(Shared(state): Shared<Arc<State>>) -> Response {
    let dates = match state.last_change.read() {
        Ok(dates) => dates.clone(),
        Err(_) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the date lock is poisoned",
            );
        }
    };
    with_tree(&state, |tree| {
        let nodes: Vec<Value> = tree
            .nodes
            .iter()
            .map(|node| {
                let items = tree
                    .items
                    .iter()
                    .filter(|item| item.node == node.path)
                    .count();
                let effective = tree.configs.effective(&node.path);
                json!({
                    "path": node.path,
                    "has_index": node.has_index,
                    "items": items,
                    "workflow": effective.workflow,
                    "tracker": effective.tracker,
                    "last_change": dates.get(&node.path).cloned().flatten(),
                })
            })
            .collect();
        Value::Array(nodes)
    })
}

/// `GET /api/schema`: the core header schema, as `ariane schema --json`.
pub async fn schema() -> Json<Value> {
    Json(crate::schema::export())
}

/// `GET /api/doctor`: the findings of `ariane doctor --all --json`.
pub async fn doctor(Shared(state): Shared<Arc<State>>) -> Response {
    with_tree(&state, |tree| {
        serde_json::to_value(crate::doctor::diagnose(tree, None, false)).unwrap_or(Value::Null)
    })
}

/// `GET /api/repo`: the repository root, its branch, how many commits it is
/// ahead of its upstream and its uncommitted files, which feed the header's
/// commit indicator.
///
/// `git` is a blocking subprocess, so it runs on tokio's blocking thread pool
/// (`spawn_blocking`) instead of stalling the threads that serve requests.
pub async fn repo(Shared(state): Shared<Arc<State>>) -> Response {
    let root = state.root.clone();
    match tokio::task::spawn_blocking(move || git::status(&root)).await {
        Ok(Ok(status)) => Json(json!({
            "root": state.root.to_string_lossy(),
            "branch": status.branch,
            "ahead": status.ahead,
            "uncommitted": status
                .entries
                .iter()
                .map(|entry| json!({ "code": entry.code, "path": entry.path.to_string_lossy() }))
                .collect::<Vec<_>>(),
        }))
        .into_response(),
        Ok(Err(failure)) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "git status did not finish",
        ),
    }
}

/// `GET /api/events`: a Server-Sent Events stream that emits `changed` with
/// `{"scope":"tree"}` after each reload, and a keep-alive comment every 15 s.
///
/// Each stream subscribes to the state's broadcast channel. A slow client that
/// misses messages gets a lag error from the channel; it is turned into one more
/// `changed` event, since all a client does with it is refetch. The stream ends
/// when the server starts stopping: reloads become `Some`, the stop signal
/// becomes `None`, and `take_while` cuts the merged stream at the first `None`.
pub async fn events(
    Shared(state): Shared<Arc<State>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let reloads = BroadcastStream::new(state.changes.subscribe()).map(|_| Some(()));
    let stop = WatchStream::new(state.stopping.subscribe())
        .filter(|stopping| *stopping)
        .map(|_| None);
    let stream = reloads.merge(stop).take_while(Option::is_some).map(|_| {
        Ok(Event::default()
            .event("changed")
            .data(r#"{"scope":"tree"}"#))
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEP_ALIVE).text("keep-alive"))
}

/// Runs `work` on the blocking pool while holding the state's `busy` lock, so
/// that writes and git actions never interleave, and turns a panic of the task
/// into a `500`.
async fn exclusive(
    state: Arc<State>,
    work: impl FnOnce(&State) -> Response + Send + 'static,
) -> Response {
    let done = tokio::task::spawn_blocking(move || match state.busy.lock() {
        Ok(_guard) => work(&state),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the write lock is poisoned",
        ),
    })
    .await;
    done.unwrap_or_else(|_| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the write did not finish",
        )
    })
}

/// `PATCH /api/items`: changes `status`, `size`, `impact` or `priority` of one
/// item, refusing a stale `from` with `409` and the current values.
///
/// After writing, the tree is reloaded at once so the response carries the
/// updated row; the watcher will reload again and notify the browsers.
pub async fn patch_item(Shared(state): Shared<Arc<State>>, Json(patch): Json<Patch>) -> Response {
    exclusive(state, move |state| {
        let applied = match state.tree.read() {
            Ok(tree) => write::apply(&tree, &patch),
            Err(_) => {
                return error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "the tree lock is poisoned",
                );
            }
        };
        match applied {
            Ok(_) => {}
            Err(Refusal::BadRequest(message)) => return error(StatusCode::BAD_REQUEST, &message),
            Err(Refusal::Invalid(message)) => {
                return error(StatusCode::UNPROCESSABLE_ENTITY, &message);
            }
            Err(Refusal::Failed(message)) => {
                return error(StatusCode::INTERNAL_SERVER_ERROR, &message);
            }
            Err(Refusal::Conflict(current)) => {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({
                        "error": "the item changed since it was read",
                        "current": current,
                    })),
                )
                    .into_response();
            }
        }
        if let Err(failure) = state.reload() {
            return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
        }
        with_tree(state, |tree| {
            tree.items
                .iter()
                .find(|item| item.relative == std::path::Path::new(&patch.path))
                .map(|item| {
                    serde_json::to_value(crate::tasks::row(tree, item)).unwrap_or(Value::Null)
                })
                .unwrap_or(Value::Null)
        })
    })
    .await
}

/// The body of `GET` and `PUT /api/dashboard`.
#[derive(Debug, Deserialize)]
pub struct Pins {
    /// Node paths, in card order.
    pub pinned: Vec<String>,
}

/// `GET /api/dashboard`: the pinned nodes, in card order.
pub async fn dashboard(Shared(state): Shared<Arc<State>>) -> Response {
    match pins::read(&state.config) {
        Ok(pinned) => Json(json!({ "pinned": pinned })).into_response(),
        Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
    }
}

/// `PUT /api/dashboard`: replaces the pinned nodes; every path must be a node.
pub async fn pin(Shared(state): Shared<Arc<State>>, Json(body): Json<Pins>) -> Response {
    exclusive(state, move |state| {
        let unknown: Vec<String> = match state.tree.read() {
            Ok(tree) => body
                .pinned
                .iter()
                .filter(|path| {
                    !tree
                        .nodes
                        .iter()
                        .any(|node| node.path.as_str() == path.as_str())
                })
                .cloned()
                .collect(),
            Err(_) => {
                return error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "the tree lock is poisoned",
                );
            }
        };
        if !unknown.is_empty() {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!("not nodes of the tree: {}", unknown.join(", ")),
            );
        }
        match pins::write(&state.config, &body.pinned) {
            Ok(()) => Json(json!({ "pinned": body.pinned })).into_response(),
            Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
        }
    })
    .await
}

/// The body of `POST /api/commit`.
#[derive(Debug, Deserialize)]
pub struct CommitRequest {
    /// The commit message; it must not be blank.
    pub message: String,
}

/// `POST /api/commit`: stages everything under `tree/` and commits it with the
/// owner's message and the machine's git identity; `409` when nothing changed.
pub async fn commit(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<CommitRequest>,
) -> Response {
    if request.message.trim().is_empty() {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "the commit message is empty",
        );
    }
    exclusive(state, move |state| {
        match git::commit(&state.root, &request.message) {
            Ok(git::Committed::Commit(hash)) => Json(json!({ "commit": hash })).into_response(),
            Ok(git::Committed::NothingToCommit) => {
                error(StatusCode::CONFLICT, "nothing to commit under tree/")
            }
            Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
        }
    })
    .await
}

/// `POST /api/push`: pushes the current branch to its upstream; `502` with
/// git's message when the push fails.
pub async fn push(Shared(state): Shared<Arc<State>>) -> Response {
    exclusive(state, |state| match git::push(&state.root) {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(failure) => error(StatusCode::BAD_GATEWAY, &format!("{failure:#}")),
    })
    .await
}

/// The body of `POST /api/discard`.
#[derive(Debug, Default, Deserialize)]
pub struct DiscardRequest {
    /// Paths under `tree/` to discard; absent means everything under `tree/`.
    #[serde(default)]
    pub paths: Option<Vec<String>>,
}

/// `POST /api/discard`: throws away the uncommitted changes of `paths`, or of
/// everything under `tree/` when `paths` is absent. A tracked file is restored
/// from `HEAD`, an untracked one is deleted. `400` when a path is absolute,
/// holds `..` or is outside `tree/`; `409` when nothing changed there.
///
/// The tree is reloaded at once, so the items fetched right after already
/// reflect the restored files.
pub async fn discard(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<DiscardRequest>,
) -> Response {
    let paths = match request.paths {
        None => Vec::new(),
        Some(paths) if paths.is_empty() => {
            return error(StatusCode::CONFLICT, "no path to discard");
        }
        Some(paths) => match paths
            .iter()
            .map(|path| git::under_tree(path))
            .collect::<anyhow::Result<Vec<_>>>()
        {
            Ok(paths) => paths,
            Err(failure) => return error(StatusCode::BAD_REQUEST, &format!("{failure:#}")),
        },
    };
    exclusive(state, move |state| {
        match git::discard(&state.root, &paths) {
            Ok(git::Discarded::Paths(discarded)) => {
                if let Err(failure) = state.reload() {
                    return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
                }
                Json(json!({
                    "discarded": discarded
                        .iter()
                        .map(|path| path.to_string_lossy())
                        .collect::<Vec<_>>(),
                }))
                .into_response()
            }
            Ok(git::Discarded::NothingToDiscard) => {
                error(StatusCode::CONFLICT, "nothing to discard under tree/")
            }
            Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
        }
    })
    .await
}

/// The query of `GET /api/diff`.
#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    /// The file under `tree/` whose diff is asked.
    pub path: String,
}

/// `GET /api/diff?path=…`: the unified diff of one file under `tree/` against
/// `HEAD`, an untracked file being compared with an empty one; `400` when the
/// path is absolute, holds `..` or is outside `tree/`.
pub async fn diff(Shared(state): Shared<Arc<State>>, Query(query): Query<DiffQuery>) -> Response {
    if let Err(failure) = git::under_tree(&query.path) {
        return error(StatusCode::BAD_REQUEST, &format!("{failure:#}"));
    }
    let root = state.root.clone();
    let path = query.path.clone();
    match tokio::task::spawn_blocking(move || git::diff(&root, &path)).await {
        Ok(Ok(diff)) => Json(json!({ "path": query.path, "diff": diff })).into_response(),
        Ok(Err(failure)) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
        Err(_) => error(StatusCode::INTERNAL_SERVER_ERROR, "git diff did not finish"),
    }
}

/// Any `/api/…` path that is not an endpoint: a JSON `404`.
pub async fn not_found() -> Response {
    error(StatusCode::NOT_FOUND, "no such API endpoint")
}

//! The JSON API of `ariane serve`, consumed by the embedded front (ADR-0012).
//!
//! Every endpoint reuses what the commands already compute, so the interface and
//! the command line can never disagree: `/api/items` is `ariane tasks --all
//! --archived --dates --json` (`?kind=` and `?body=1` are `--kind` and
//! `--body`), `/api/search?q=` is `ariane search --json` (`?kind=`, `?node=` and
//! `?archived=1` are `--kind`, `--node` and `--archived`),
//! `/api/show?ref=` is `ariane show --json`, `/api/schema` is
//! `ariane schema --json`, `/api/doctor` is `ariane doctor --all --json`,
//! `/api/proxy` is the checks of `ariane proxy status`, `/api/calendar` is
//! `ariane calendar --json`, `/api/roadmap?node=` is `ariane roadmap <node>
//! --json`. Responses are built under a short read lock on
//! the tree, released before anything is sent.
//!
//! The writing endpoints — `PATCH /api/items`, `PUT /api/dashboard`,
//! `PUT /api/settings/theme`, `PUT` and `DELETE /api/proxy/routes`,
//! `POST`, `PATCH` and `DELETE /api/releases` (`ariane release`),
//! `PATCH` and `DELETE /api/calendar` (`ariane calendar set|unset`),
//! `PATCH /api/estimates` (`ariane estimates set|unset`),
//! `POST /api/commit`, `POST /api/push`, `POST /api/pull`, `POST /api/discard` — run on the
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
use super::git;
use super::write::{self, Move, Patch, Refusal};
use crate::calendar::write::{
    BlockSpec, Change as CalendarChange, NodeChange, Refusal as CalendarRefusal,
};
use crate::item::Kind;
use crate::release::declare::{self, Place, Refusal as ReleaseRefusal};
use crate::tree::{NodePath, Tree};

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

/// The query of `GET /api/search`.
#[derive(Debug, Default, Deserialize)]
pub struct SearchQuery {
    /// The words to find; no word finds nothing.
    #[serde(default)]
    pub q: String,
    /// Comma-separated kinds, as `ariane search --kind`; every kind when absent.
    pub kind: Option<String>,
    /// Only this node and its descendants, as `--node`; the whole tree when absent.
    pub node: Option<String>,
    /// `1` or `true` includes archived items, as `--archived`.
    pub archived: Option<String>,
}

/// `GET /api/search?q=`: the items `q` finds, best first, as
/// `ariane search --all --json` with the dates of `/api/items`; `400` for an
/// unknown kind.
pub async fn search(
    Shared(state): Shared<Arc<State>>,
    Query(query): Query<SearchQuery>,
) -> Response {
    let kinds = match parse_kinds(query.kind.as_deref().unwrap_or_default()) {
        Ok(kinds) => kinds,
        Err(message) => return error(StatusCode::BAD_REQUEST, &message),
    };
    let scope = query.node.as_deref().map(crate::tree::NodePath::new);
    let Ok(dates) = state.dates.read() else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the date lock is poisoned",
        );
    };
    with_tree(&state, |tree| {
        let options = crate::search::Options {
            archived: matches!(query.archived.as_deref(), Some("1" | "true")),
            kinds: &kinds,
            dates: Some(&dates),
        };
        serde_json::to_value(crate::search::search(
            tree,
            scope.as_ref(),
            &query.q,
            &options,
        ))
        .unwrap_or(Value::Null)
    })
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

/// `GET /api/nodes`: every node with its own `description`, its item count, effective
/// workflow, releases in delivery order (`[[release]]`, inherited, `null` when none applies) and the node that declares them, what sizes cost (`[estimates]`, inherited key by key) and the node each key comes from (`estimates_from`, `null` for the value of ADR-0025), people (`[people]`, inherited), personas (`[personas]`, inherited) and tracker binding (after `node.toml` inheritance, ADR-0008 and
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
                    "description": tree.configs.description(&node.path),
                    "items": items,
                    "workflow": effective.workflow,
                    "releases": effective.releases,
                    "releases_from": effective.releases_from,
                    "estimates": effective.estimates,
                    "estimates_from": crate::estimate::declare::sources(&tree.configs, &node.path),
                    "people": effective.people,
                    "personas": effective.personas,
                    "tracker": effective.tracker,
                    "tracker_from": effective.tracker_from,
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
        if let Err(refusal) = applied {
            return refused(refusal);
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

/// `POST /api/items/rank`: places one item in the manual order of its node
/// and kind, with an optional status change, refusing stale ranks with `409`.
///
/// Responds with the rows of the items rewritten, read from the reloaded tree.
pub async fn rank_item(Shared(state): Shared<Arc<State>>, Json(request): Json<Move>) -> Response {
    exclusive(state, move |state| {
        let applied = match state.tree.read() {
            Ok(tree) => write::rank(&tree, &request),
            Err(_) => {
                return error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "the tree lock is poisoned",
                );
            }
        };
        let written = match applied {
            Ok(written) => written,
            Err(refusal) => return refused(refusal),
        };
        if let Err(failure) = state.reload() {
            return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
        }
        with_tree(state, |tree| {
            let rows: Vec<Value> = tree
                .items
                .iter()
                .filter(|item| written.contains(&item.file))
                .map(|item| {
                    serde_json::to_value(crate::tasks::row(tree, item)).unwrap_or(Value::Null)
                })
                .collect();
            json!({ "items": rows })
        })
    })
    .await
}

/// The response to a refused edit: its message, and for a conflict the current values.
fn refused(refusal: Refusal) -> Response {
    match refusal {
        Refusal::BadRequest(message) => error(StatusCode::BAD_REQUEST, &message),
        Refusal::Invalid(message) => error(StatusCode::UNPROCESSABLE_ENTITY, &message),
        Refusal::Failed(message) => error(StatusCode::INTERNAL_SERVER_ERROR, &message),
        Refusal::Conflict(current) => (
            StatusCode::CONFLICT,
            Json(json!({
                "error": "the item changed since it was read",
                "current": current,
            })),
        )
            .into_response(),
    }
}

/// The body of `GET` and `PUT /api/dashboard`.
#[derive(Debug, Deserialize)]
pub struct Pins {
    /// Node paths, in card order.
    pub pinned: Vec<String>,
}

/// `GET /api/dashboard`: the pinned nodes, in card order, from the content's
/// `settings.toml`.
pub async fn dashboard(Shared(state): Shared<Arc<State>>) -> Response {
    match crate::settings::load(&state.root) {
        Ok(settings) => Json(json!({ "pinned": settings.dashboard.pinned })).into_response(),
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
        match crate::settings::set_pinned(&state.root, &body.pinned) {
            Ok(()) => Json(json!({ "pinned": body.pinned })).into_response(),
            Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
        }
    })
    .await
}

/// `GET /api/settings`: the settings versioned with the content, as
/// `ariane settings --json`.
pub async fn settings(Shared(state): Shared<Arc<State>>) -> Response {
    match crate::settings::load(&state.root) {
        Ok(settings) => Json(settings).into_response(),
        Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
    }
}

/// `GET /api/calendar`: the personal work calendar, as `ariane calendar
/// --json`: the file read, whether it exists, its valid part and its problems.
pub async fn calendar(Shared(state): Shared<Arc<State>>) -> Response {
    with_personal_tree(&state, |personal, tree| {
        overview(crate::calendar::overview(personal, tree))
    })
}

/// Runs `answer` on the personal repository and its tree: the served tree
/// when the server serves the personal repository, else that repository's,
/// loaded for the occasion.
fn with_personal_tree(
    state: &State,
    answer: impl FnOnce(&std::path::Path, &Tree) -> Response,
) -> Response {
    let personal = crate::calendar::personal_of(&state.root, state.config.repo_path().as_deref());
    if crate::calendar::is_personal(&state.root, &personal) {
        let Ok(tree) = state.tree.read() else {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the tree lock is poisoned",
            );
        };
        return answer(&personal, &tree);
    }
    match Tree::load(&personal) {
        Ok(tree) => answer(&personal, &tree),
        Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
    }
}

/// The body of `PATCH /api/calendar`: a node and what changes in its table,
/// or no node and the global keys. An absent key is left as it is.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarRequest {
    /// The node whose table changes; absent for the global keys.
    #[serde(default)]
    pub node: Option<String>,
    /// The node's blocks, replacing the old ones; `[]` removes them.
    #[serde(default)]
    pub blocks: Option<Vec<BlockSpec>>,
    /// The node's focus; `null` removes it.
    #[serde(default, deserialize_with = "present")]
    pub focus: Option<Option<f64>>,
    /// The node's days off, or every node's without a node; `[]` removes them.
    #[serde(default)]
    pub days_off: Option<Vec<String>>,
    /// The time zone of every block; only without a node.
    #[serde(default)]
    pub timezone: Option<String>,
}

impl CalendarRequest {
    /// The change the request asks for; refuses keys of a node table without
    /// a node, and the time zone with one.
    fn change(self) -> Result<CalendarChange, String> {
        match self.node {
            Some(node) => {
                if self.timezone.is_some() {
                    return Err("`timezone` is global: send it without `node`".to_owned());
                }
                Ok(CalendarChange::Node {
                    node: NodePath::new(node),
                    change: NodeChange {
                        blocks: self.blocks,
                        focus: self.focus,
                        days_off: self.days_off,
                    },
                })
            }
            None => {
                if self.blocks.is_some() || self.focus.is_some() {
                    return Err("`blocks` and `focus` belong to a node: send `node`".to_owned());
                }
                Ok(CalendarChange::Globals {
                    timezone: self.timezone,
                    days_off: self.days_off,
                })
            }
        }
    }
}

/// The body of `DELETE /api/calendar`.
#[derive(Debug, Deserialize)]
pub struct CalendarRemoval {
    /// The node whose table goes.
    pub node: String,
}

/// Writes one change of the personal calendar, reloads the tree so that the
/// browsers are told, and answers the calendar as `GET /api/calendar` does.
/// `422` for a refused change: `error`, and for a result with errors the
/// `problems` it would have, each overlap naming its two nodes (`node`,
/// `other`) and the position of each block in its node's list (`block`,
/// `other_block`).
async fn change_calendar(state: Arc<State>, change: CalendarChange) -> Response {
    exclusive(state, move |state| {
        let written = with_personal_tree(
            state,
            |personal, tree| match crate::calendar::write::write(personal, tree, &change) {
                Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
                Ok(Err(refusal)) => refused_calendar(&refusal),
                Ok(Ok(_)) => StatusCode::OK.into_response(),
            },
        );
        if written.status() != StatusCode::OK {
            return written;
        }
        if let Err(failure) = state.reload() {
            return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
        }
        with_personal_tree(state, |personal, tree| {
            overview(crate::calendar::overview(personal, tree))
        })
    })
    .await
}

/// The `422` of a refused change of the calendar.
fn refused_calendar(refusal: &CalendarRefusal) -> Response {
    let mut body = json!({ "error": refusal.to_string() });
    if let CalendarRefusal::Problems(problems) = refusal {
        body["problems"] = serde_json::to_value(problems).unwrap_or(Value::Null);
    }
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
}

/// `PATCH /api/calendar`: sets a node's `blocks`, `focus` (`null` removes it)
/// and `days_off`, or without `node` the `timezone` and the `days_off` of
/// every node, as `ariane calendar set`; `400` for a malformed request.
pub async fn change_calendar_entry(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<CalendarRequest>,
) -> Response {
    match request.change() {
        Ok(change) => change_calendar(state, change).await,
        Err(malformed) => error(StatusCode::BAD_REQUEST, &malformed),
    }
}

/// `DELETE /api/calendar`: removes a node's table, as `ariane calendar unset`.
pub async fn remove_calendar_entry(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<CalendarRemoval>,
) -> Response {
    let change = CalendarChange::Remove {
        node: NodePath::new(request.node),
    };
    change_calendar(state, change).await
}

/// The body of `PATCH /api/estimates`.
#[derive(Debug, Deserialize)]
pub struct EstimatesRequest {
    /// The node whose own `[estimates]` changes.
    pub node: String,
    /// The keys to change: a number or a string sets one, `null` removes it.
    pub estimates: serde_json::Map<String, Value>,
}

impl EstimatesRequest {
    /// The changes as [`crate::estimate::declare::change`] takes them;
    /// refuses a value that is neither a number, a string nor `null`.
    fn changes(&self) -> Result<Vec<(String, Option<toml::Value>)>, String> {
        self.estimates
            .iter()
            .map(|(key, value)| {
                let value = match value {
                    Value::Null => None,
                    Value::String(text) => Some(toml::Value::String(text.clone())),
                    Value::Number(number) => Some(match number.as_i64() {
                        Some(integer) => toml::Value::Integer(integer),
                        None => toml::Value::Float(number.as_f64().unwrap_or(f64::NAN)),
                    }),
                    other => {
                        return Err(format!(
                            "`{key}`: {other} is neither a number, a string nor null"
                        ));
                    }
                };
                Ok((key.clone(), value))
            })
            .collect()
    }
}

/// `PATCH /api/estimates`: sets or removes keys of a node's own `[estimates]`
/// all at once, as `ariane estimates set|unset`, and answers the node's
/// `estimates` and, key by key, the node each comes from (`estimates_from`,
/// `null` for the value of ADR-0025). `400` for a value of the wrong type,
/// `422` for a refused change, nothing written.
pub async fn change_estimates(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<EstimatesRequest>,
) -> Response {
    let changes = match request.changes() {
        Ok(changes) => changes,
        Err(malformed) => return error(StatusCode::BAD_REQUEST, &malformed),
    };
    exclusive(state, move |state| {
        let node = NodePath::new(&request.node);
        let outcome = match state.tree.read() {
            Ok(tree) => crate::estimate::declare::change(&tree, &node, &changes),
            Err(_) => {
                return error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "the tree lock is poisoned",
                );
            }
        };
        match outcome {
            Err(failure) => {
                return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
            }
            Ok(Err(refusal)) => return error(StatusCode::UNPROCESSABLE_ENTITY, &refusal),
            Ok(Ok(())) => {}
        }
        if let Err(failure) = state.reload() {
            return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
        }
        with_tree(state, |tree| {
            serde_json::to_value(crate::estimate::declare::report(tree, &node))
                .unwrap_or(Value::Null)
        })
    })
    .await
}

/// The query of `GET /api/roadmap`.
#[derive(Debug, Default, Deserialize)]
pub struct RoadmapQuery {
    /// The node to forecast.
    pub node: Option<String>,
}

/// `GET /api/roadmap?node=<node>`: the forecast of a node, as `ariane roadmap
/// <node> --json`, on the personal calendar of the configuration the server
/// started with; `400` without a node, `404` for a node not in the tree,
/// `500` when the calendar cannot be read.
pub async fn roadmap(
    Shared(state): Shared<Arc<State>>,
    Query(query): Query<RoadmapQuery>,
) -> Response {
    let Some(node) = query.node.as_deref().filter(|node| !node.is_empty()) else {
        return error(
            StatusCode::BAD_REQUEST,
            "name a node: /api/roadmap?node=<node>",
        );
    };
    let node = crate::tree::NodePath::new(node);
    let personal = crate::calendar::personal_of(&state.root, state.config.repo_path().as_deref());
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
    if !crate::roadmap::is_node(&tree, &node) {
        return error(
            StatusCode::NOT_FOUND,
            &format!("`{node}` is not a node of the tree"),
        );
    }
    match crate::roadmap::forecast(
        &personal,
        &tree,
        Some(&dates),
        &node,
        crate::forecast::now(),
    ) {
        Ok(forecast) => Json(forecast).into_response(),
        Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
    }
}

/// The JSON of a calendar overview, or a `500` when the file cannot be read.
fn overview(read: anyhow::Result<crate::calendar::Overview>) -> Response {
    match read {
        Ok(overview) => Json(overview).into_response(),
        Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
    }
}

/// The body of `PUT /api/settings/theme`.
#[derive(Debug, Deserialize)]
pub struct ThemeRequest {
    pub primary: String,
    pub secondary: String,
}

/// `PUT /api/settings/theme`: sets both theme colors in `settings.toml`, as
/// `ariane settings set`, and answers the settings; `422` for a color that is
/// not `#rrggbb`.
pub async fn theme(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<ThemeRequest>,
) -> Response {
    exclusive(state, move |state| {
        for color in [&request.primary, &request.secondary] {
            if !crate::settings::is_color(color) {
                return error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    &format!("`{color}` is not a color written `#rrggbb`"),
                );
            }
        }
        match crate::settings::set_theme(&state.root, &request.primary, &request.secondary)
            .and_then(|()| crate::settings::load(&state.root))
        {
            Ok(settings) => Json(settings).into_response(),
            Err(failure) => error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
        }
    })
    .await
}

/// The body of `POST /api/sync/monday`.
#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    /// Repository-relative path of the item, as `/api/items` gives it.
    pub path: String,
}

/// `POST /api/sync/monday`: sends one item's status to its monday ticket, as
/// `ariane sync monday --item <ref> --apply`, and answers the plan carried
/// out — a conflict or a refusal is reported, never forced. `502` when monday
/// cannot be reached or a token is missing.
pub async fn sync_monday(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<SyncRequest>,
) -> Response {
    exclusive(state, move |state| {
        let Ok(tree) = state.tree.read() else {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the tree lock is poisoned",
            );
        };
        let Some(item) = tree
            .items
            .iter()
            .find(|item| item.relative == std::path::Path::new(&request.path))
        else {
            return error(StatusCode::BAD_REQUEST, "not an item of the tree");
        };
        match crate::monday::sync(
            &tree,
            None,
            Some(item),
            true,
            &crate::secrets::file(),
            &crate::monday::state_file(),
            &|token| Box::new(crate::monday::Http::new(token)),
        ) {
            Ok(report) => Json(report).into_response(),
            Err(failure) => error(StatusCode::BAD_GATEWAY, &format!("{failure:#}")),
        }
    })
    .await
}

/// `GET /api/people`: the nodes that declare a tracker or people, with their
/// people and who waits for each, as `ariane people --json`.
pub async fn people(Shared(state): Shared<Arc<State>>) -> Response {
    with_tree(&state, |tree| {
        serde_json::to_value(crate::people::list(tree, None)).unwrap_or(Value::Null)
    })
}

/// The body of the writes of `/api/people`.
#[derive(Debug, Deserialize)]
pub struct PersonRequest {
    pub node: String,
    pub handle: String,
    #[serde(default)]
    pub name: Option<String>,
    /// `null` removes the monday id; absent keeps it.
    #[serde(default, deserialize_with = "present")]
    pub monday: Option<Option<i64>>,
    /// A new handle, rewritten in the items that wait for the person.
    #[serde(default)]
    pub rename_to: Option<String>,
}

/// Tells an absent key (`None`) from a `null` one (`Some(None)`).
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Runs one change of the directory, reloads the tree and answers the new
/// directory; `422` for a refused change, `409` with the items for a person
/// still awaited.
async fn change_people(
    state: Arc<State>,
    change: impl FnOnce(&crate::tree::Tree) -> anyhow::Result<Result<(), crate::people::Refusal>>
    + Send
    + 'static,
) -> Response {
    exclusive(state, move |state| {
        let outcome = match state.tree.read() {
            Ok(tree) => change(&tree),
            Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "the tree lock is poisoned"),
        };
        match outcome {
            Err(failure) => return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
            Ok(Err(crate::people::Refusal::Awaited(items))) => {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": crate::people::Refusal::Awaited(items.clone()).to_string(), "items": items })),
                )
                    .into_response();
            }
            Ok(Err(refusal)) => return error(StatusCode::UNPROCESSABLE_ENTITY, &refusal.to_string()),
            Ok(Ok(())) => {}
        }
        if let Err(failure) = state.reload() {
            return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
        }
        with_tree(state, |tree| serde_json::to_value(crate::people::list(tree, None)).unwrap_or(Value::Null))
    })
    .await
}

/// `POST /api/people`: declares a person, as `ariane people add`.
pub async fn add_person(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<PersonRequest>,
) -> Response {
    change_people(state, move |tree| {
        crate::people::add(
            tree,
            &crate::tree::NodePath::new(&request.node),
            &request.handle,
            request.name.as_deref().unwrap_or_default(),
            request.monday.flatten(),
        )
    })
    .await
}

/// `PATCH /api/people`: changes a person's name or monday id, and renames
/// the handle (items rewritten) when `rename_to` is given, as `ariane people
/// set` and `rename`.
pub async fn change_person(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<PersonRequest>,
) -> Response {
    change_people(state, move |tree| {
        let node = crate::tree::NodePath::new(&request.node);
        let change = crate::people::Change {
            name: request.name.clone(),
            monday: request.monday,
        };
        if let Err(refusal) = crate::people::set(tree, &node, &request.handle, &change)? {
            return Ok(Err(refusal));
        }
        match request
            .rename_to
            .as_deref()
            .filter(|to| *to != request.handle)
        {
            Some(to) => crate::people::rename(tree, &node, &request.handle, to)
                .map(|outcome| outcome.map(|_| ())),
            None => Ok(Ok(())),
        }
    })
    .await
}

/// `DELETE /api/people`: removes a person no item waits for, as `ariane
/// people remove`.
pub async fn remove_person(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<PersonRequest>,
) -> Response {
    change_people(state, move |tree| {
        crate::people::remove(
            tree,
            &crate::tree::NodePath::new(&request.node),
            &request.handle,
        )
    })
    .await
}

/// `GET /api/personas`: the nodes that declare a tracker or personas, with
/// their personas and the items written for each.
pub async fn personas(Shared(state): Shared<Arc<State>>) -> Response {
    with_tree(&state, |tree| {
        serde_json::to_value(crate::personas::list(tree)).unwrap_or(Value::Null)
    })
}

/// The body of the writes of `/api/personas`.
#[derive(Debug, Deserialize)]
pub struct PersonaRequest {
    pub node: String,
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// A new id, rewritten in the items that name the persona.
    #[serde(default)]
    pub rename_to: Option<String>,
}

/// Runs one change of the personas, reloads the tree and answers the new
/// directory; `422` for a refused change, `409` with the items for a persona
/// still named.
async fn change_personas(
    state: Arc<State>,
    change: impl FnOnce(&crate::tree::Tree) -> anyhow::Result<Result<(), crate::personas::Refusal>>
    + Send
    + 'static,
) -> Response {
    exclusive(state, move |state| {
        let outcome = match state.tree.read() {
            Ok(tree) => change(&tree),
            Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "the tree lock is poisoned"),
        };
        match outcome {
            Err(failure) => return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}")),
            Ok(Err(crate::personas::Refusal::Used(items))) => {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({ "error": crate::personas::Refusal::Used(items.clone()).to_string(), "items": items })),
                )
                    .into_response();
            }
            Ok(Err(refusal)) => return error(StatusCode::UNPROCESSABLE_ENTITY, &refusal.to_string()),
            Ok(Ok(())) => {}
        }
        if let Err(failure) = state.reload() {
            return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
        }
        with_tree(state, |tree| serde_json::to_value(crate::personas::list(tree)).unwrap_or(Value::Null))
    })
    .await
}

/// `POST /api/personas`: declares a persona.
pub async fn add_persona(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<PersonaRequest>,
) -> Response {
    change_personas(state, move |tree| {
        crate::personas::add(
            tree,
            &crate::tree::NodePath::new(&request.node),
            &request.id,
            request.name.as_deref().unwrap_or_default(),
        )
    })
    .await
}

/// `PATCH /api/personas`: changes a persona's name, and renames the id
/// (items rewritten) when `rename_to` is given.
pub async fn change_persona(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<PersonaRequest>,
) -> Response {
    change_personas(state, move |tree| {
        let node = crate::tree::NodePath::new(&request.node);
        if let Some(name) = request.name.as_deref()
            && let Err(refusal) = crate::personas::set_name(tree, &node, &request.id, name)?
        {
            return Ok(Err(refusal));
        }
        match request.rename_to.as_deref().filter(|to| *to != request.id) {
            Some(to) => crate::personas::rename(tree, &node, &request.id, to)
                .map(|outcome| outcome.map(|_| ())),
            None => Ok(Ok(())),
        }
    })
    .await
}

/// `DELETE /api/personas`: removes a persona no item names.
pub async fn remove_persona(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<PersonaRequest>,
) -> Response {
    change_personas(state, move |tree| {
        crate::personas::remove(
            tree,
            &crate::tree::NodePath::new(&request.node),
            &request.id,
        )
    })
    .await
}

/// Where `POST` and `PATCH /api/releases` place a release.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleasePlace {
    /// Right before the release `other` names.
    Before,
    /// Right after the release `other` names.
    After,
    /// First of the list.
    Top,
    /// Last of the list.
    Bottom,
}

/// The body of the writes of `/api/releases`.
#[derive(Debug, Deserialize)]
pub struct ReleaseRequest {
    /// The node that declares the list.
    pub node: String,
    /// The release to add, move, rename or remove.
    pub name: String,
    /// Where to add or move it: last when absent on an add, unmoved on a change.
    #[serde(default)]
    pub place: Option<ReleasePlace>,
    /// The release it is placed before or after.
    #[serde(default)]
    pub other: Option<String>,
    /// Whether it is a parking release: `false` on an add when absent,
    /// unchanged on a change.
    #[serde(default)]
    pub parking: Option<bool>,
    /// Its target date; on a change, `null` removes it and absent keeps it.
    #[serde(default, deserialize_with = "present")]
    pub target: Option<Option<String>>,
    /// A new name, rewritten in the items that name the release.
    #[serde(default)]
    pub rename_to: Option<String>,
}

impl ReleaseRequest {
    /// The marks a change sets: those the request gives.
    fn marks(&self) -> declare::Marks<'_> {
        declare::Marks {
            parking: self.parking,
            target: self.target.as_ref().map(Option::as_deref),
        }
    }

    /// The place the request asks for, `None` when it asks none; refuses
    /// `before` or `after` without `other`.
    fn place(&self) -> Result<Option<Place<'_>>, String> {
        let other = || {
            self.other
                .as_deref()
                .ok_or_else(|| "`before` and `after` need `other`".to_owned())
        };
        Ok(match self.place {
            None => None,
            Some(ReleasePlace::Top) => Some(Place::Top),
            Some(ReleasePlace::Bottom) => Some(Place::Bottom),
            Some(ReleasePlace::Before) => Some(Place::Before(other()?)),
            Some(ReleasePlace::After) => Some(Place::After(other()?)),
        })
    }
}

/// What a change of releases did: the items it rewrote, or why it did nothing.
type ReleaseOutcome = anyhow::Result<Result<Vec<String>, ReleaseRefusal>>;

/// Runs one change of a release list, reloads the tree and answers the list
/// the node plans against: `node`, `releases_from`, `releases` in delivery
/// order and the items a rename `rewritten`. `400` for a malformed request,
/// `422` for a refused change (with `declared_in` when the node inherits its
/// list), `409` with the `items` that still name a release to remove.
async fn change_releases(
    state: Arc<State>,
    request: ReleaseRequest,
    change: impl FnOnce(&Tree, &ReleaseRequest) -> Result<ReleaseOutcome, String> + Send + 'static,
) -> Response {
    exclusive(state, move |state| {
        let outcome = match state.tree.read() {
            Ok(tree) => change(&tree, &request),
            Err(_) => {
                return error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "the tree lock is poisoned",
                );
            }
        };
        let rewritten = match outcome {
            Err(malformed) => return error(StatusCode::BAD_REQUEST, &malformed),
            Ok(Err(failure)) => {
                return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
            }
            Ok(Ok(Err(refusal))) => return refused_release(&refusal),
            Ok(Ok(Ok(rewritten))) => rewritten,
        };
        if let Err(failure) = state.reload() {
            return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
        }
        let node = NodePath::new(&request.node);
        with_tree(state, |tree| {
            let (from, releases) = declare::effective_list(tree, &node);
            json!({
                "node": node,
                "releases_from": from,
                "releases": releases,
                "rewritten": rewritten,
            })
        })
    })
    .await
}

/// The response to a refused change of releases.
fn refused_release(refusal: &ReleaseRefusal) -> Response {
    let (status, extra) = match refusal {
        ReleaseRefusal::Invalid(_) => (StatusCode::UNPROCESSABLE_ENTITY, json!({})),
        ReleaseRefusal::Inherited { from, .. } => (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "declared_in": from }),
        ),
        ReleaseRefusal::Used { items, .. } => (StatusCode::CONFLICT, json!({ "items": items })),
    };
    let mut body = extra;
    body["error"] = Value::String(refusal.to_string());
    (status, Json(body)).into_response()
}

/// `POST /api/releases`: declares a release at its place, last by default,
/// as `ariane release add`.
pub async fn add_release(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<ReleaseRequest>,
) -> Response {
    change_releases(state, request, |tree, request| {
        let place = request.place()?.unwrap_or(Place::Bottom);
        let declaration = declare::Declaration {
            name: request.name.clone(),
            parking: request.parking.unwrap_or(false),
            target: request.target.clone().flatten(),
        };
        Ok(
            declare::add(tree, &NodePath::new(&request.node), &declaration, place)
                .map(|outcome| outcome.map(|()| Vec::new())),
        )
    })
    .await
}

/// `PATCH /api/releases`: moves a release when `place` is given, sets its
/// `parking` mark and its `target` (`null` removes it) when given, then
/// renames it (items rewritten) when `rename_to` is, as `ariane release move`,
/// `set` and `rename`.
pub async fn change_release(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<ReleaseRequest>,
) -> Response {
    change_releases(state, request, |tree, request| {
        let node = NodePath::new(&request.node);
        let place = request.place()?;
        let marks = request.marks();
        if place.is_none()
            && request.rename_to.is_none()
            && marks.parking.is_none()
            && marks.target.is_none()
        {
            return Err(
                "nothing to change: give `place`, `parking`, `target` or `rename_to`".to_owned(),
            );
        }
        Ok(move_set_then_rename(tree, &node, request, place))
    })
    .await
}

/// Moves the release `request` names to `place`, if any, then sets its marks,
/// if any, then renames it to `rename_to`, if any.
fn move_set_then_rename(
    tree: &Tree,
    node: &NodePath,
    request: &ReleaseRequest,
    place: Option<Place>,
) -> ReleaseOutcome {
    if let Some(place) = place
        && let Err(refusal) = declare::move_to(tree, node, &request.name, place)?
    {
        return Ok(Err(refusal));
    }
    let marks = request.marks();
    if (marks.parking.is_some() || marks.target.is_some())
        && let Err(refusal) = declare::set(tree, node, &request.name, marks)?
    {
        return Ok(Err(refusal));
    }
    match request.rename_to.as_deref() {
        Some(to) => declare::rename(tree, node, &request.name, to),
        None => Ok(Ok(Vec::new())),
    }
}

/// `DELETE /api/releases`: removes a release no item names, as `ariane
/// release remove`.
pub async fn remove_release(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<ReleaseRequest>,
) -> Response {
    change_releases(state, request, |tree, request| {
        Ok(
            declare::remove(tree, &NodePath::new(&request.node), &request.name)
                .map(|outcome| outcome.map(|()| Vec::new())),
        )
    })
    .await
}

/// The query of `GET /api/monday/users`.
#[derive(Debug, Deserialize)]
pub struct UsersQuery {
    pub account: String,
    pub q: String,
}

/// `GET /api/monday/users?account=&q=`: the account's users whose name
/// contains `q`, as `ariane people find`; `502` without a token or a reachable monday.
pub async fn monday_users(Query(query): Query<UsersQuery>) -> Response {
    let found = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<crate::monday::User>> {
        let Some(token) = crate::secrets::token(&crate::secrets::file(), "monday", &query.account)?
        else {
            anyhow::bail!(
                "no monday token for `{}`: run `ariane connect monday --account {}`",
                query.account,
                query.account
            );
        };
        crate::monday::Http::new(token).users(&query.q)
    })
    .await;
    match found {
        Ok(Ok(users)) => Json(users).into_response(),
        Ok(Err(failure)) => error(StatusCode::BAD_GATEWAY, &format!("{failure:#}")),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the search did not finish",
        ),
    }
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
            Ok(git::Committed::NothingToCommit) => error(
                StatusCode::CONFLICT,
                "nothing to commit in tree/, settings.toml or calendar.toml",
            ),
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

/// `POST /api/pull`: fast-forwards the current branch to its upstream and
/// answers how many commits came in, `0` when it was up to date; `502` with
/// git's message when the pull is refused or fails.
///
/// The tree is reloaded at once, so the items fetched right after already
/// reflect the pulled files.
pub async fn pull(Shared(state): Shared<Arc<State>>) -> Response {
    exclusive(state, |state| match git::pull(&state.root) {
        Ok(git::Pulled::UpToDate) => Json(json!({ "pulled": 0 })).into_response(),
        Ok(git::Pulled::Forward(count)) => {
            if let Err(failure) = state.reload() {
                return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
            }
            Json(json!({ "pulled": count })).into_response()
        }
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
            .map(|path| git::in_content(path))
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
            Ok(git::Discarded::NothingToDiscard) => error(
                StatusCode::CONFLICT,
                "nothing to discard in tree/, settings.toml or calendar.toml",
            ),
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
    if let Err(failure) = git::in_content(&query.path) {
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

/// `GET /api/proxy`: every check of `ariane proxy status` with its verdict,
/// each route with whether its name is declared and its port listens, the
/// ports of the front door and of the interface, and the `[proxy]` tables
/// left out.
///
/// The probes are blocking connections, so they run on the blocking pool.
pub async fn proxy(Shared(state): Shared<Arc<State>>) -> Response {
    let table = match state.tree.read() {
        Ok(tree) => crate::proxy::Table::of(&tree, &state.config.server),
        Err(_) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the tree lock is poisoned",
            );
        }
    };
    let checked = tokio::task::spawn_blocking(move || {
        crate::proxy::machine::check(&state.machine, &state.config, &table)
    })
    .await;
    match checked {
        Ok(report) => Json(report).into_response(),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the checks did not finish",
        ),
    }
}

/// The body of `PUT /api/proxy/routes`.
#[derive(Debug, Deserialize)]
pub struct RouteRequest {
    /// The node whose `[proxy]` table or `[[proxy]]` entry is written.
    pub node: String,
    pub host: String,
    /// Read wide, so that a port out of range is refused with a reason.
    pub port: i64,
    /// The host of the node's route being edited; absent to add a route.
    #[serde(default)]
    pub replaces: Option<String>,
}

/// The body of `DELETE /api/proxy/routes`.
#[derive(Debug, Deserialize)]
pub struct RouteRemoval {
    pub node: String,
    /// The host of the route to remove; absent to remove every route of the
    /// node.
    #[serde(default)]
    pub host: Option<String>,
}

/// Runs one write of a node's routes, reloads the tree and answers the new
/// route table; `422` with the reason for a refused write.
async fn change_route(
    state: Arc<State>,
    change: impl FnOnce(&Tree, &crate::config::Config) -> anyhow::Result<Result<(), String>>
    + Send
    + 'static,
) -> Response {
    exclusive(state, move |state| {
        let outcome = match state.tree.read() {
            Ok(tree) => change(&tree, &state.config),
            Err(_) => {
                return error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "the tree lock is poisoned",
                );
            }
        };
        match outcome {
            Err(failure) => {
                return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
            }
            Ok(Err(refusal)) => return error(StatusCode::UNPROCESSABLE_ENTITY, &refusal),
            Ok(Ok(())) => {}
        }
        if let Err(failure) = state.reload() {
            return error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{failure:#}"));
        }
        with_tree(state, |tree| {
            serde_json::to_value(crate::proxy::Table::of(tree, &state.config.server))
                .unwrap_or(Value::Null)
        })
    })
    .await
}

/// `PUT /api/proxy/routes`: gives a node the route `host` → `port` in its
/// `node.toml`, rewriting the route `replaces` when given, and answers the
/// route table.
pub async fn set_route(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<RouteRequest>,
) -> Response {
    change_route(state, move |tree, config| {
        crate::proxy::declare::set(
            tree,
            config,
            &crate::tree::NodePath::new(&request.node),
            &request.host,
            request.port,
            request.replaces.as_deref(),
        )
    })
    .await
}

/// `DELETE /api/proxy/routes`: takes a node's route `host` away, or all of
/// them without `host`, and answers the route table.
pub async fn remove_route(
    Shared(state): Shared<Arc<State>>,
    Json(request): Json<RouteRemoval>,
) -> Response {
    change_route(state, move |tree, _| {
        crate::proxy::declare::remove(
            tree,
            &crate::tree::NodePath::new(&request.node),
            request.host.as_deref(),
        )
    })
    .await
}

/// Any `/api/…` path that is not an endpoint: a JSON `404`.
pub async fn not_found() -> Response {
    error(StatusCode::NOT_FOUND, "no such API endpoint")
}

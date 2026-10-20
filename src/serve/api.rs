//! The JSON API of `ariane serve`, consumed by the embedded front (ADR-0012).
//!
//! Every endpoint reuses what the commands already compute, so the interface and
//! the command line can never disagree: `/api/items` is `ariane tasks --all
//! --archived --json`, `/api/schema` is `ariane schema --json`, `/api/doctor` is
//! `ariane doctor --all --json`. Responses are built under a short read lock on
//! the tree, released before anything is sent.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::State as Shared;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use super::State;
use super::git;
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

/// `GET /api/items`: every item of the tree, archived ones included, as the rows
/// of `ariane tasks --json`.
pub async fn items(Shared(state): Shared<Arc<State>>) -> Response {
    with_tree(&state, |tree| {
        let rows: Vec<_> = tree
            .items
            .iter()
            .map(|item| crate::tasks::row(tree, item))
            .collect();
        serde_json::to_value(rows).unwrap_or(Value::Null)
    })
}

/// `GET /api/nodes`: every node with its entry point, item count and effective
/// workflow (after `node.toml` inheritance, ADR-0008).
pub async fn nodes(Shared(state): Shared<Arc<State>>) -> Response {
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
                json!({
                    "path": node.path,
                    "has_index": node.has_index,
                    "items": items,
                    "workflow": tree.configs.effective(&node.path).workflow,
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

/// `GET /api/repo`: the repository root, its branch and its uncommitted files,
/// which feed the header's commit indicator.
///
/// `git` is a blocking subprocess, so it runs on tokio's blocking thread pool
/// (`spawn_blocking`) instead of stalling the threads that serve requests.
pub async fn repo(Shared(state): Shared<Arc<State>>) -> Response {
    let root = state.root.clone();
    match tokio::task::spawn_blocking(move || git::status(&root)).await {
        Ok(Ok(status)) => Json(json!({
            "root": state.root.to_string_lossy(),
            "branch": status.branch,
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
/// `changed` event, since all a client does with it is refetch.
pub async fn events(
    Shared(state): Shared<Arc<State>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(state.changes.subscribe()).map(|_| {
        Ok(Event::default()
            .event("changed")
            .data(r#"{"scope":"tree"}"#))
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEP_ALIVE).text("keep-alive"))
}

/// Any `/api/…` path that is not an endpoint: a JSON `404`.
pub async fn not_found() -> Response {
    error(StatusCode::NOT_FOUND, "no such API endpoint")
}

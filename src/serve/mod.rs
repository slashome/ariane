//! `ariane serve`: the web interface, served by the binary itself (ADR-0012).
//!
//! One process: an `axum` server bound to `127.0.0.1` only, holding the loaded
//! [`Tree`] in memory. The front is a React application embedded at build time
//! ([`assets`]); the JSON API it calls lives in [`api`]. A file watcher
//! ([`watch`]) reloads the tree when anything under `tree/` or `.git/` changes and
//! tells the browsers through Server-Sent Events, so there is no Rescan button.
//!
//! The interface writes too, within limits: it edits four header keys of an
//! item ([`mod@write`]), the theme and the dashboard pins in the content's
//! `settings.toml` ([`crate::settings`]), and commits or pushes only when the
//! owner asks ([`git`]). Writes and git actions
//! are serialized, one at a time.

mod api;
mod assets;
mod git;
mod watch;
mod write;

use std::collections::{BTreeMap, HashMap};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{Context, Result};
use axum::Router;
use axum::routing::{get, post, put};
use tokio::sync::{broadcast, watch as signal};

use crate::cli::ServeArgs;
use crate::tree::{NodePath, Tree};

/// What every request handler shares: the repository root, the tree loaded from
/// it with the date of each node's latest commit and of each file, and the
/// channel that announces each reload to the open event streams.
///
/// The tree sits behind a [`RwLock`]: handlers take short read locks to build
/// their response, the watcher takes a write lock only to swap in a freshly
/// loaded tree. No lock is held across an `.await`.
pub struct State {
    /// Root of the content repository.
    pub root: PathBuf,
    /// The tree as last loaded from disk.
    pub tree: RwLock<Tree>,
    /// Date of the latest commit under each node, recomputed with every load.
    pub last_change: RwLock<BTreeMap<NodePath, Option<String>>>,
    /// Date of the latest commit of each item file, by path relative to the
    /// root, recomputed with every load.
    pub dates: RwLock<HashMap<PathBuf, String>>,
    /// Held for the whole of a write or a git action, so that two of them never
    /// interleave.
    pub busy: Mutex<()>,
    /// Sends one message per reload; each `/api/events` stream subscribes to it.
    pub changes: broadcast::Sender<()>,
    /// Turns `true` when the server stops, which ends every `/api/events`
    /// stream so the graceful shutdown is not held open by a browser tab.
    pub stopping: signal::Sender<bool>,
}

impl State {
    /// Wraps a loaded tree into shared state, ready to be cloned into handlers.
    ///
    /// `Arc` is a reference-counted pointer: cloning it is cheap and every clone
    /// points to the same state, which is how axum hands it to each request.
    pub fn new(root: PathBuf, tree: Tree) -> Arc<State> {
        let (changes, _) = broadcast::channel(16);
        let (stopping, _) = signal::channel(false);
        let last_change = RwLock::new(last_changes(&root, &tree));
        let dates = RwLock::new(crate::history::file_dates(&root));
        Arc::new(State {
            root,
            tree: RwLock::new(tree),
            last_change,
            dates,
            busy: Mutex::new(()),
            changes,
            stopping,
        })
    }

    /// Reloads the tree from disk, swaps it in and announces the change.
    ///
    /// A tree that fails to load leaves the previous one in place: the interface
    /// keeps showing the last good state rather than going blank.
    ///
    /// # Errors
    ///
    /// Fails when the tree cannot be read; the shared state is then unchanged.
    pub fn reload(&self) -> Result<()> {
        let fresh = Tree::load(&self.root)?;
        let nodes = last_changes(&self.root, &fresh);
        let files = crate::history::file_dates(&self.root);
        *self
            .tree
            .write()
            .map_err(|_| anyhow::anyhow!("the tree lock is poisoned"))? = fresh;
        *self
            .last_change
            .write()
            .map_err(|_| anyhow::anyhow!("the date lock is poisoned"))? = nodes;
        *self
            .dates
            .write()
            .map_err(|_| anyhow::anyhow!("the date lock is poisoned"))? = files;
        let _ = self.changes.send(());
        Ok(())
    }
}

/// Dates every node of `tree` by its latest commit, with one `git log` per node.
fn last_changes(root: &Path, tree: &Tree) -> BTreeMap<NodePath, Option<String>> {
    tree.nodes
        .iter()
        .map(|node| {
            let mut directory = PathBuf::from("tree");
            directory.extend(
                node.path
                    .as_str()
                    .split('/')
                    .filter(|segment| !segment.is_empty()),
            );
            (node.path.clone(), git::last_change(root, &directory))
        })
        .collect()
}

/// Builds the router: the JSON API under `/api`, everything else from the
/// embedded front with a fallback to its `index.html`.
pub fn router(state: Arc<State>) -> Router {
    Router::new()
        .route("/api/items", get(api::items).patch(api::patch_item))
        .route("/api/show", get(api::show))
        .route("/api/dashboard", get(api::dashboard).put(api::pin))
        .route("/api/settings", get(api::settings))
        .route("/api/settings/theme", put(api::theme))
        .route("/api/commit", post(api::commit))
        .route("/api/sync/monday", post(api::sync_monday))
        .route(
            "/api/people",
            get(api::people)
                .post(api::add_person)
                .patch(api::change_person)
                .delete(api::remove_person),
        )
        .route("/api/monday/users", get(api::monday_users))
        .route("/api/push", post(api::push))
        .route("/api/discard", post(api::discard))
        .route("/api/diff", get(api::diff))
        .route("/api/nodes", get(api::nodes))
        .route("/api/schema", get(api::schema))
        .route("/api/doctor", get(api::doctor))
        .route("/api/repo", get(api::repo))
        .route("/api/events", get(api::events))
        .route("/api", get(api::not_found))
        .route("/api/{*rest}", get(api::not_found))
        .fallback(get(assets::serve))
        .with_state(state)
}

/// Runs `ariane serve` until Ctrl-C.
///
/// The server listens on `127.0.0.1` only, never on another interface, which is
/// why it has no authentication (ADR-0012). `main` is synchronous, so this
/// function builds the tokio runtime itself and blocks on the server.
///
/// # Errors
///
/// Fails when the runtime cannot start, the port is taken, or the watcher
/// cannot be installed.
pub fn run(root: PathBuf, tree: Tree, args: &ServeArgs) -> Result<ExitCode> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("starting the async runtime")?;
    let port = match args.port {
        Some(port) => port,
        None => {
            crate::config::load(&crate::config::file())?
                .config
                .server
                .port
        }
    };
    runtime.block_on(serve(root, tree, port, args.open))?;
    Ok(ExitCode::SUCCESS)
}

/// Binds the port, starts the watcher and serves until Ctrl-C.
///
/// # Errors
///
/// Fails when the port cannot be bound or the watcher cannot be installed.
async fn serve(root: PathBuf, tree: Tree, port: u16, open: bool) -> Result<()> {
    let state = State::new(root.clone(), tree);
    let _watcher = watch::start(state.clone()).context("watching the content repository")?;
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("listening on {address}"))?;
    let url = format!("http://{address}");
    eprintln!("ariane serve: {} on {url} (Ctrl-C to stop)", root.display());
    if open {
        open_browser(&url);
    }
    let stopping = state.stopping.clone();
    axum::serve(listener, router(state))
        .with_graceful_shutdown(stop_on_ctrl_c(stopping))
        .await
        .context("serving")
}

/// Waits for Ctrl-C, then asks every event stream to end so the graceful
/// shutdown can complete.
///
/// A connection that still lingers must not keep the process alive: a second
/// Ctrl-C, or [`FORCE_AFTER`] without one, exits the process with code 130,
/// the conventional status of a process interrupted by `SIGINT`.
async fn stop_on_ctrl_c(stopping: signal::Sender<bool>) {
    let _ = tokio::signal::ctrl_c().await;
    eprintln!("ariane serve: stopping");
    let _ = stopping.send(true);
    tokio::spawn(async {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            () = tokio::time::sleep(FORCE_AFTER) => {}
        }
        std::process::exit(130);
    });
}

/// How long the graceful shutdown may take before the process exits anyway.
const FORCE_AFTER: std::time::Duration = std::time::Duration::from_secs(3);

/// Opens `url` in the default browser: `open` on macOS, `xdg-open` elsewhere.
///
/// A failure is reported on stderr and ignored: the server is up either way.
fn open_browser(url: &str) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(error) = std::process::Command::new(program).arg(url).spawn() {
        eprintln!("ariane serve: could not open the browser ({error}); open {url}");
    }
}

/// Requests sent to the router without a network, against fixture repositories.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// A repository with one node, a workflow, one task and one invalid item.
    fn sample() -> Repo {
        let repo = Repo::new();
        repo.write("p/INDEX.md", "# p\n")
            .write(
                "p/node.toml",
                "[[workflow]]\nid = \"a-faire\"\nlabel = \"À faire\"\ncategory = \"todo\"\n\n[tracker]\nkind = \"monday\"\nboard = 7\n",
            )
            .write(
                "p/tasks/T001-a.md",
                "---\nariane: 1\nnode: p\nid: T001\ntitle: A\nstatus: a-faire\nexternal: [monday:42]\n---\n# A\n",
            )
            .write("p/tasks/legacy.md", "# Legacy\n");
        repo
    }

    /// Sends a GET to a router over `repo` and returns the status and body.
    async fn get(repo: &Repo, path: &str) -> (StatusCode, Vec<u8>, Option<String>) {
        let state = State::new(repo.root().to_path_buf(), Tree::load(repo.root()).unwrap());
        let response = router(state)
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .map(|value| value.to_str().unwrap().to_owned());
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, body.to_vec(), content_type)
    }

    /// Parses a response body as JSON.
    fn json(body: &[u8]) -> serde_json::Value {
        serde_json::from_slice(body).unwrap()
    }

    #[tokio::test]
    async fn items_lists_every_item_with_its_resolved_category() {
        let repo = sample();
        let (status, body, _) = get(&repo, "/api/items").await;
        assert_eq!(status, StatusCode::OK);
        let items = json(&body);
        assert_eq!(items.as_array().unwrap().len(), 2);
        let task = items
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "T001")
            .unwrap();
        assert_eq!(task["category"], "todo");
        assert_eq!(task["node"], "p");
        assert_eq!(task["external"], serde_json::json!(["monday:42"]));
    }

    #[tokio::test]
    async fn items_keep_the_kinds_asked_and_add_bodies_on_request() {
        let repo = sample();
        repo.write(
            "p/stories/S001-s.md",
            "---\nariane: 1\nnode: p\nid: S001\ntitle: S\nstatus: a-faire\n---\n# S\n",
        );
        let (status, body, _) = get(&repo, "/api/items?kind=story&body=1").await;
        assert_eq!(status, StatusCode::OK);
        let items = json(&body);
        assert_eq!(items.as_array().unwrap().len(), 1);
        assert_eq!(items[0]["reference"], "p#S001");
        assert_eq!(items[0]["body"], "# S\n");
        let (_, body, _) = get(&repo, "/api/items?kind=task,story").await;
        assert_eq!(json(&body).as_array().unwrap().len(), 3);
        assert!(json(&body)[0].get("body").is_none());
        let (status, body, _) = get(&repo, "/api/items?kind=backlog").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(json(&body)["error"].as_str().unwrap().contains("backlog"));
    }

    #[tokio::test]
    async fn show_is_the_same_json_as_the_show_command() {
        let repo = sample();
        repo.write(
            "p/tasks/T002-b.md",
            "---\nariane: 1\nnode: p\nid: T002\ntitle: B\nstatus: a-faire\ndepends_on: [T001]\n---\n# B\n",
        );
        let (status, body, _) = get(&repo, "/api/show?ref=p%23T001").await;
        assert_eq!(status, StatusCode::OK);
        let tree = Tree::load(repo.root()).unwrap();
        let item = crate::show::find(&tree, "p#T001", None).unwrap();
        let dates = crate::history::file_dates(repo.root());
        let expected = serde_json::to_value(crate::show::shown(&tree, item, Some(&dates))).unwrap();
        assert_eq!(json(&body), expected);
        assert_eq!(
            expected["relations"]["dependents"][0]["reference"],
            "p#T002"
        );
        assert_eq!(expected["header"]["external"][0], "monday:42");
        let (status, body, _) = get(&repo, "/api/show?ref=p%23T404").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(json(&body)["error"].is_string());
        let (status, _, _) = get(&repo, "/api/show").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn nodes_carry_their_item_count_and_effective_workflow() {
        let repo = sample();
        let (status, body, _) = get(&repo, "/api/nodes").await;
        assert_eq!(status, StatusCode::OK);
        let nodes = json(&body);
        let node = nodes
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["path"] == "p")
            .unwrap();
        assert_eq!(node["items"], 2);
        assert_eq!(node["has_index"], true);
        assert_eq!(node["workflow"][0]["id"], "a-faire");
        assert_eq!(node["workflow"][0]["category"], "todo");
        assert_eq!(node["tracker"]["kind"], "monday");
        assert_eq!(node["tracker"]["board"], 7);
        let root = nodes
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["path"] == "")
            .unwrap();
        assert!(root["tracker"].is_null());
    }

    #[tokio::test]
    async fn schema_is_the_same_json_as_the_schema_command() {
        let repo = sample();
        let (status, body, _) = get(&repo, "/api/schema").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json(&body), crate::schema::export());
    }

    #[tokio::test]
    async fn doctor_reports_the_same_findings_as_the_doctor_command() {
        let repo = sample();
        let (status, body, _) = get(&repo, "/api/doctor").await;
        assert_eq!(status, StatusCode::OK);
        let tree = Tree::load(repo.root()).unwrap();
        let expected = serde_json::to_value(crate::doctor::diagnose(&tree, None, false)).unwrap();
        assert_eq!(json(&body), expected);
        assert!(!expected.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_unknown_api_path_is_a_json_404() {
        let repo = sample();
        let (status, body, content_type) = get(&repo, "/api/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(json(&body)["error"].is_string());
        assert!(content_type.unwrap().starts_with("application/json"));
    }

    #[tokio::test]
    async fn any_other_path_falls_back_to_the_front_entry_page() {
        let repo = sample();
        let (status, body, content_type) = get(&repo, "/tasks/some/deep/link").await;
        assert_eq!(status, StatusCode::OK);
        assert!(content_type.unwrap().starts_with("text/html"));
        assert!(String::from_utf8(body).unwrap().contains("<html"));
    }

    /// Sends a request with a JSON body to a router over `repo` and returns the
    /// status and the parsed body.
    async fn send(
        repo: &Repo,
        method: &str,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let state = State::new(repo.root().to_path_buf(), Tree::load(repo.root()).unwrap());
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = router(state).oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap_or_default())
    }

    #[tokio::test]
    async fn patching_an_item_answers_its_updated_row_or_the_reason_it_was_refused() {
        let repo = sample();
        let path = "tree/p/tasks/T001-a.md";
        let edit = |from: &str, to: &str| serde_json::json!({ "path": path, "changes": { "status": { "from": from, "to": to } } });
        let (status, body) = send(&repo, "PATCH", "/api/items", edit("a-faire", "zzz")).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(body["error"].is_string());
        let (status, body) = send(&repo, "PATCH", "/api/items", edit("other", "a-faire")).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["current"]["status"], "a-faire");
        let outside = serde_json::json!({ "path": "Cargo.toml", "changes": { "size": { "from": null, "to": "S" } } });
        let (status, _) = send(&repo, "PATCH", "/api/items", outside).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let sized =
            serde_json::json!({ "path": path, "changes": { "size": { "from": null, "to": "M" } } });
        let (status, body) = send(&repo, "PATCH", "/api/items", sized).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["size"], "M");
        assert_eq!(body["id"], "T001");
    }

    #[tokio::test]
    async fn the_dashboard_pins_round_trip_through_the_settings_file() {
        let repo = sample();
        let settings = repo.root().join("settings.toml");
        std::fs::write(&settings, "# keep me\nversion = 1\n").unwrap();
        let (status, body) = send(&repo, "GET", "/api/dashboard", serde_json::Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["pinned"], serde_json::json!([]));
        let (status, _) = send(
            &repo,
            "PUT",
            "/api/dashboard",
            serde_json::json!({ "pinned": ["p", ""] }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (_, body) = send(&repo, "GET", "/api/dashboard", serde_json::Value::Null).await;
        assert_eq!(body["pinned"], serde_json::json!(["p", ""]));
        assert!(
            std::fs::read_to_string(&settings)
                .unwrap()
                .starts_with("# keep me\nversion = 1\n")
        );
        let (status, _) = send(
            &repo,
            "PUT",
            "/api/dashboard",
            serde_json::json!({ "pinned": ["nope"] }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn the_theme_is_read_and_set_through_the_settings_file() {
        let repo = sample();
        let (status, body, _) = get(&repo, "/api/settings").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            json(&body)["theme"]["primary"],
            crate::settings::DEFAULT_COLOR
        );
        let theme = serde_json::json!({ "primary": "#AA0000", "secondary": "#00aa00" });
        let (status, body) = send(&repo, "PUT", "/api/settings/theme", theme).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["theme"]["primary"], "#aa0000");
        let (_, body, _) = get(&repo, "/api/settings").await;
        assert_eq!(
            json(&body),
            serde_json::to_value(crate::settings::load(repo.root()).unwrap()).unwrap()
        );
        let bad = serde_json::json!({ "primary": "red", "secondary": "#00aa00" });
        let (status, body) = send(&repo, "PUT", "/api/settings/theme", bad).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(body["error"].as_str().unwrap().contains("red"));
    }

    #[tokio::test]
    async fn syncing_an_item_without_a_monday_binding_sends_nothing() {
        let repo = sample();
        repo.write(
            "q/tasks/T1-a.md",
            "---\nariane: 1\nnode: q\nid: T1\ntitle: A\nstatus: todo\n---\n",
        );
        let body = serde_json::json!({ "path": "tree/q/tasks/T1-a.md" });
        let (status, answer) = send(&repo, "POST", "/api/sync/monday", body).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(answer["steps"], serde_json::json!([]));
        let (status, _) = send(
            &repo,
            "POST",
            "/api/sync/monday",
            serde_json::json!({ "path": "Cargo.toml" }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn the_people_are_listed_added_renamed_and_kept_while_awaited() {
        let repo = sample();
        repo.write(
            "p/tasks/T002-b.md",
            "---\nariane: 1\nnode: p\nid: T002\ntitle: B\nstatus: a-faire\n---\n",
        );
        let person = |extra: serde_json::Value| {
            let mut body = serde_json::json!({ "node": "p", "handle": "keyvan" });
            body.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            body
        };
        let (status, body) = send(
            &repo,
            "POST",
            "/api/people",
            person(serde_json::json!({ "name": "Keyvan", "monday": 42 })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body[0]["node"], "p");
        assert_eq!(body[0]["people"][0]["monday"], 42);
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/people",
            person(serde_json::json!({ "monday": null, "rename_to": "kcoen" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body[0]["people"][0]["handle"], "kcoen");
        assert!(body[0]["people"][0]["monday"].is_null());
        let (status, _) = send(
            &repo,
            "POST",
            "/api/people",
            person(serde_json::json!({ "handle": "Bad", "name": "X" })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, body, _) = get(&repo, "/api/people").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json(&body)[0]["people"].as_array().unwrap().len(), 1);
        let (status, _) = send(
            &repo,
            "DELETE",
            "/api/people",
            serde_json::json!({ "node": "p", "handle": "kcoen" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    /// Replaces the fixture's repository by a fresh one with its own identity,
    /// so commits run as in production without touching the machine's config.
    fn reinit_git(repo: &Repo) {
        std::fs::remove_dir_all(repo.root().join(".git")).unwrap();
        for args in [
            &["init", "-q"][..],
            &["config", "user.name", "test"],
            &["config", "user.email", "test@example.com"],
        ] {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(repo.root())
                .args(args)
                .status()
                .unwrap()
                .success();
            assert!(ok);
        }
    }

    #[tokio::test]
    async fn committing_needs_a_message_and_something_to_commit() {
        let repo = sample();
        reinit_git(&repo);
        let (status, _) = send(
            &repo,
            "POST",
            "/api/commit",
            serde_json::json!({ "message": "  " }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, body) = send(
            &repo,
            "POST",
            "/api/commit",
            serde_json::json!({ "message": "import" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["commit"].is_string());
        let (status, _) = send(
            &repo,
            "POST",
            "/api/commit",
            serde_json::json!({ "message": "again" }),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn a_changed_file_shows_its_diff_and_can_be_discarded() {
        let repo = sample();
        reinit_git(&repo);
        send(
            &repo,
            "POST",
            "/api/commit",
            serde_json::json!({ "message": "import" }),
        )
        .await;
        let path = repo.root().join("tree/p/tasks/T001-a.md");
        let original = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, original.replace("title: A", "title: Changed")).unwrap();

        let (status, body, _) = get(&repo, "/api/diff?path=tree/p/tasks/T001-a.md").await;
        assert_eq!(status, StatusCode::OK);
        let diff = json(&body)["diff"].as_str().unwrap().to_owned();
        assert!(diff.contains("-title: A\n+title: Changed"), "{diff}");
        let (status, _, _) = get(&repo, "/api/diff?path=tree/../config.toml").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        for refused in ["/etc/passwd", "tree/../config.toml", "config.toml"] {
            let (status, _) = send(
                &repo,
                "POST",
                "/api/discard",
                serde_json::json!({ "paths": [refused] }),
            )
            .await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        }
        let (status, body) = send(
            &repo,
            "POST",
            "/api/discard",
            serde_json::json!({ "paths": ["tree/p/tasks/T001-a.md"] }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body["discarded"],
            serde_json::json!(["tree/p/tasks/T001-a.md"])
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        let (status, _) = send(&repo, "POST", "/api/discard", serde_json::json!({})).await;
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn the_event_stream_ends_when_the_server_stops() {
        let repo = sample();
        let state = State::new(repo.root().to_path_buf(), Tree::load(repo.root()).unwrap());
        let response = router(state.clone())
            .oneshot(Request::get("/api/events").body(Body::empty()).unwrap())
            .await
            .unwrap();
        state.stopping.send(true).unwrap();
        let body = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            response.into_body().collect(),
        )
        .await;
        assert!(
            body.is_ok(),
            "the event stream stayed open after the stop signal"
        );
    }

    #[tokio::test]
    async fn a_reload_swaps_the_tree_and_notifies_subscribers() {
        let repo = sample();
        let state = State::new(repo.root().to_path_buf(), Tree::load(repo.root()).unwrap());
        let mut changes = state.changes.subscribe();
        repo.write(
            "p/tasks/T002-b.md",
            "---\nariane: 1\nnode: p\nid: T002\ntitle: B\nstatus: a-faire\n---\n# B\n",
        );
        state.reload().unwrap();
        assert_eq!(state.tree.read().unwrap().items.len(), 3);
        assert!(changes.try_recv().is_ok());
    }
}

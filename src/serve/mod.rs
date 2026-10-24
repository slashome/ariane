//! `ariane serve`: the web interface, served by the binary itself (ADR-0012).
//!
//! One process: an `axum` server bound to `127.0.0.1` only, holding the loaded
//! [`Tree`] in memory. The front is a React application embedded at build time
//! ([`assets`]); the JSON API it calls lives in [`api`]. A file watcher
//! ([`watch`]) reloads the tree when anything under `tree/` or `.git/` changes and
//! tells the browsers through Server-Sent Events, so there is no Rescan button.
//!
//! Next to the interface, the server opens the front door of `ariane proxy`
//! ([`crate::proxy`], ADR-0016), whose routes follow the tree as it reloads.
//!
//! The interface writes too, within limits: it edits four header keys of an
//! item ([`mod@write`]), the theme and the dashboard pins in the content's
//! `settings.toml` ([`crate::settings`]), the `[proxy]` table of a node's
//! `node.toml` ([`crate::proxy::declare`]), and commits or pushes only when the
//! owner asks ([`git`]). Writes and git actions
//! are serialized, one at a time.
//!
//! As it starts, the server pulls the content once, fast-forward only, in the
//! background: the interface opens at once on what is on disk, and the watcher
//! shows what the pull brings.

mod api;
mod assets;
pub mod git;
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
use crate::config::Config;
use crate::proxy::machine::Machine;
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
    /// The configuration the server started with, whose ports and
    /// `server.host` the front door routes by.
    pub config: Config,
    /// The machine `/api/proxy` checks the chain of `ariane proxy` on.
    pub machine: Machine,
}

impl State {
    /// Wraps a loaded tree into shared state, ready to be cloned into handlers,
    /// with the configuration it serves under and the machine it runs on.
    ///
    /// `Arc` is a reference-counted pointer: cloning it is cheap and every clone
    /// points to the same state, which is how axum hands it to each request.
    pub fn new(root: PathBuf, tree: Tree, config: Config, machine: Machine) -> Arc<State> {
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
            config,
            machine,
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
        .route("/api/search", get(api::search))
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
        .route("/api/pull", post(api::pull))
        .route("/api/discard", post(api::discard))
        .route("/api/diff", get(api::diff))
        .route("/api/nodes", get(api::nodes))
        .route("/api/schema", get(api::schema))
        .route("/api/doctor", get(api::doctor))
        .route("/api/repo", get(api::repo))
        .route("/api/proxy", get(api::proxy))
        .route(
            "/api/proxy/routes",
            put(api::set_route).delete(api::remove_route),
        )
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
/// Fails when the runtime cannot start, the configuration is invalid, the port
/// is taken, or the watcher cannot be installed.
pub fn run(root: PathBuf, tree: Tree, args: &ServeArgs) -> Result<ExitCode> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("starting the async runtime")?;
    let mut config = crate::config::load(&crate::config::file())?.config;
    if let Some(port) = args.port {
        config.server.port = port;
    }
    runtime.block_on(serve(root, tree, &config, args.open))?;
    Ok(ExitCode::SUCCESS)
}

/// Binds the port, starts the watcher and the first pull, opens the front door
/// and serves until Ctrl-C.
///
/// # Errors
///
/// Fails when the port cannot be bound or the watcher cannot be installed. A
/// front door that cannot open is reported, not an error: the interface stays
/// reachable on its port.
async fn serve(root: PathBuf, tree: Tree, config: &Config, open: bool) -> Result<()> {
    let state = State::new(root.clone(), tree, config.clone(), Machine::current());
    let _watcher = watch::start(state.clone()).context("watching the content repository")?;
    pull_at_start(state.clone());
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, config.server.port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("listening on {address}"))?;
    let url = format!("http://{address}");
    eprintln!("ariane serve: {} on {url} (Ctrl-C to stop)", root.display());
    open_front_door(&state, config).await;
    if open {
        open_browser(&url_to_open(&state, config, url.clone()).await);
    }
    let stopping = state.stopping.clone();
    axum::serve(listener, router(state))
        .with_graceful_shutdown(stop_on_ctrl_c(stopping))
        .await
        .context("serving")
}

/// Pulls the content once, on the blocking pool and under the `busy` lock, so
/// that no write of the interface interleaves with it.
///
/// The outcome is printed and never stops the server: offline, without an
/// upstream or on a diverged branch, the interface serves what is on disk.
fn pull_at_start(state: Arc<State>) {
    tokio::task::spawn_blocking(move || {
        let Ok(_guard) = state.busy.lock() else {
            return;
        };
        match git::pull(&state.root) {
            Ok(git::Pulled::UpToDate) => eprintln!("ariane serve: the content is up to date"),
            Ok(git::Pulled::Forward(count)) => {
                eprintln!("ariane serve: pulled {count} commit(s)");
                if let Err(error) = state.reload() {
                    eprintln!("ariane serve: reloading after the pull: {error:#}");
                }
            }
            Err(error) => eprintln!("ariane serve: no pull: {error:#}"),
        }
    });
}

/// Opens the front door of `ariane proxy` on `proxy.port`, routing by the
/// tree as it is when each request arrives, so that a `[proxy]` table added to
/// a `node.toml` is served without a restart.
///
/// When the port is taken — usually by `ariane proxy` run alone — the reason
/// is printed and the interface keeps serving without it.
async fn open_front_door(state: &Arc<State>, config: &Config) {
    let listeners = match crate::proxy::door::bind(config.proxy.port).await {
        Ok(listeners) => listeners,
        Err(error) => {
            eprintln!("ariane serve: no front door: {error:#}");
            return;
        }
    };
    let state = state.clone();
    let lookup: crate::proxy::door::Lookup = Arc::new(move |host| {
        let tree = state.tree.read().ok()?;
        crate::proxy::Table::of(&tree, &state.config.server)
            .find(host)
            .cloned()
    });
    eprintln!(
        "ariane serve: front door on 127.0.0.1:{port} and [::1]:{port}, http://{host} once `ariane proxy setup` ran",
        port = config.proxy.port,
        host = config.server.host
    );
    tokio::spawn(async move {
        if let Err(error) = crate::proxy::door::serve(listeners, lookup).await {
            eprintln!("ariane serve: the front door stopped: {error:#}");
        }
    });
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

/// The address `--open` uses: the interface's name, `http://<server.host>`,
/// when it reaches the front door, else `url` on its loopback port.
async fn url_to_open(state: &Arc<State>, config: &Config, url: String) -> String {
    let machine = state.machine.clone();
    let host = config.server.host.clone();
    let named = tokio::task::spawn_blocking(move || {
        crate::proxy::machine::reaches_by_name(&machine, &host)
    })
    .await
    .unwrap_or(false);
    if named {
        format!("http://{}", config.server.host)
    } else {
        url
    }
}

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
        repo.write(
                "p/node.toml",
                "description = \"The *p* project.\"\n\n[[workflow]]\nid = \"a-faire\"\nlabel = \"À faire\"\ncategory = \"todo\"\n\n[tracker]\nkind = \"monday\"\nboard = 7\n",
            )
            .write(
                "p/tasks/T001-a.md",
                "---\nariane: 1\nnode: p\nid: T001\ntitle: A\nstatus: a-faire\nexternal: [monday:42]\n---\n# A\n",
            )
            .write("p/tasks/legacy.md", "# Legacy\n");
        repo
    }

    /// A runner that answers every command with empty output.
    struct Quiet;

    impl crate::service::Runner for Quiet {
        fn run(&self, _: &str, _: &[String]) -> anyhow::Result<crate::service::Outcome> {
            Ok(crate::service::Outcome::default())
        }
    }

    /// A port of `127.0.0.1` that nothing listens on.
    fn closed_port() -> u16 {
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.local_addr().unwrap().port()
    }

    /// The state of a server over `repo` whose ports nothing listens on, on a
    /// machine with an empty hosts file and no alias.
    fn state(repo: &Repo) -> Arc<State> {
        let mut config = Config::default();
        config.server.port = closed_port();
        config.proxy.port = closed_port();
        let machine = Machine {
            macos: true,
            hosts: repo.root().join("hosts"),
            staging: repo.root().join("staging"),
            redirected: [SocketAddr::from((Ipv4Addr::LOCALHOST, closed_port())); 2],
            runner: Arc::new(Quiet),
        };
        State::new(
            repo.root().to_path_buf(),
            Tree::load(repo.root()).unwrap(),
            config,
            machine,
        )
    }

    /// Sends a GET to a router over `repo` and returns the status and body.
    async fn get(repo: &Repo, path: &str) -> (StatusCode, Vec<u8>, Option<String>) {
        let state = state(repo);
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
    async fn search_is_the_same_json_as_the_search_command() {
        let repo = sample();
        repo.write(
            "p/archives/tasks/T009-z.md",
            "---\nariane: 1\nnode: p\nid: T009\ntitle: Été\nstatus: a-faire\n---\nT001 again\n",
        );
        let (status, body, _) = get(&repo, "/api/search?q=t001").await;
        assert_eq!(status, StatusCode::OK);
        let tree = Tree::load(repo.root()).unwrap();
        let dates = crate::history::file_dates(repo.root());
        let options = crate::search::Options {
            dates: Some(&dates),
            ..crate::search::Options::default()
        };
        let expected =
            serde_json::to_value(crate::search::search(&tree, None, "t001", &options)).unwrap();
        assert_eq!(json(&body), expected);
        assert_eq!(expected[0]["reference"], "p#T001");
        assert_eq!(expected[0]["field"], "reference");
        assert_eq!(expected.as_array().unwrap().len(), 1);
        let (_, body, _) = get(&repo, "/api/search?q=T001&archived=1").await;
        assert_eq!(json(&body)[1]["reference"], "p#T009");
        assert_eq!(json(&body)[1]["field"], "body");
        let (_, body, _) = get(&repo, "/api/search?q=ete&archived=true&kind=task&node=p").await;
        assert_eq!(json(&body)[0]["highlights"], serde_json::json!([[0, 3]]));
        let (_, body, _) = get(&repo, "/api/search?q=T001&node=q").await;
        assert_eq!(json(&body), serde_json::json!([]));
        let (_, body, _) = get(&repo, "/api/search").await;
        assert_eq!(json(&body), serde_json::json!([]));
        let (status, _, _) = get(&repo, "/api/search?q=a&kind=nope").await;
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
        assert!(node.get("has_index").is_none());
        assert_eq!(node["description"], "The *p* project.");
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
        assert!(root["description"].is_null());
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
        let state = state(repo);
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

    #[tokio::test]
    async fn the_proxy_report_checks_each_link_and_each_route() {
        let repo = sample();
        let upstream = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = upstream.local_addr().unwrap().port();
        repo.write(
            "p/node.toml",
            &format!("[proxy]\nhost = \"p.local\"\nport = {port}\n"),
        )
        .write(
            "q/node.toml",
            "[proxy]\nhost = \"ariane.local\"\nport = 9\n",
        );
        let (status, body, _) = get(&repo, "/api/proxy").await;
        assert_eq!(status, StatusCode::OK);
        let report = json(&body);
        assert_eq!(report["ok"], false);
        let checks = report["checks"].as_array().unwrap();
        assert_eq!(checks.len(), 11);
        assert_eq!(checks[0]["what"], "alias 127.0.0.2 on lo0");
        assert_eq!(checks[0]["ok"], false);
        assert_eq!(checks[0]["hint"], "missing: run `ariane proxy setup`");
        assert_eq!(checks[1]["what"], "alias fd61:7269:616e:6500::2 on lo0");
        assert_eq!(checks[10]["ok"], true);
        let routes = report["routes"].as_array().unwrap();
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0]["host"], "ariane.local");
        assert!(routes[0]["node"].is_null());
        assert_eq!(routes[0]["listening"], false);
        assert_eq!(
            routes[1],
            serde_json::json!({ "host": "p.local", "port": port, "node": "p", "declared": false, "listening": true })
        );
        assert!(report["proxy_port"].is_u64());
        assert!(report["server_port"].is_u64());
        assert!(
            report["problems"][0]
                .as_str()
                .unwrap()
                .contains("already the name of the interface")
        );
    }

    #[tokio::test]
    async fn a_route_is_written_to_node_toml_refused_with_its_reason_and_removed() {
        let repo = sample();
        let file = repo.root().join("tree/p/node.toml");
        let route =
            |host: &str, port: i64| serde_json::json!({ "node": "p", "host": host, "port": port });
        let (status, body) = send(&repo, "PUT", "/api/proxy/routes", route("p.local", 3000)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["routes"][1]["host"], "p.local");
        assert_eq!(body["routes"][1]["node"], "p");
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("description = \"The *p* project.\"\n"));
        assert!(text.contains("[tracker]"));
        assert!(text.ends_with("[proxy]\nhost = \"p.local\"\nport = 3000\n"));
        for (host, port, reason) in [
            ("P.local", 3000, "not a host name"),
            ("ariane.local", 3000, "the interface"),
            ("p.local", 70000, "between 1 and 65535"),
        ] {
            let (status, body) = send(&repo, "PUT", "/api/proxy/routes", route(host, port)).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{host}:{port}");
            assert!(body["error"].as_str().unwrap().contains(reason), "{body}");
        }
        let (status, _) = send(
            &repo,
            "PUT",
            "/api/proxy/routes",
            serde_json::json!({ "node": "nowhere", "host": "n.local", "port": 1 }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, body) = send(
            &repo,
            "DELETE",
            "/api/proxy/routes",
            serde_json::json!({ "node": "p" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["routes"].as_array().unwrap().len(), 1);
        assert!(!std::fs::read_to_string(&file).unwrap().contains("proxy"));
        let (status, _) = send(
            &repo,
            "DELETE",
            "/api/proxy/routes",
            serde_json::json!({ "node": "p" }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn a_route_write_reloads_the_tree_and_notifies_subscribers() {
        let repo = sample();
        let state = state(&repo);
        let mut changes = state.changes.subscribe();
        let request = Request::builder()
            .method("PUT")
            .uri("/api/proxy/routes")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "node": "p", "host": "p.local", "port": 3000 }).to_string(),
            ))
            .unwrap();
        let response = router(state.clone()).oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(changes.try_recv().is_ok());
        let tree = state.tree.read().unwrap();
        assert!(
            crate::proxy::Table::of(&tree, &state.config.server)
                .find("p.local")
                .is_some()
        );
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
    async fn pulling_without_an_upstream_is_a_bad_gateway_and_with_one_says_how_many_came() {
        let repo = sample();
        reinit_git(&repo);
        send(
            &repo,
            "POST",
            "/api/commit",
            serde_json::json!({ "message": "import" }),
        )
        .await;
        let (status, body) = send(&repo, "POST", "/api/pull", serde_json::json!({})).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert!(body["error"].is_string());
        let remote = tempfile::tempdir().unwrap();
        for (root, args) in [
            (remote.path(), &["init", "-q", "--bare"][..]),
            (
                repo.root(),
                &["remote", "add", "origin", remote.path().to_str().unwrap()],
            ),
            (repo.root(), &["push", "-q", "-u", "origin", "HEAD"]),
        ] {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .status()
                .unwrap()
                .success();
            assert!(ok);
        }
        let (status, body) = send(&repo, "POST", "/api/pull", serde_json::json!({})).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["pulled"], 0);
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
        let state = state(&repo);
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
        let state = state(&repo);
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

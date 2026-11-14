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
//! `node.toml` ([`crate::proxy::declare`]), its releases
//! ([`crate::release::declare`]) and its `[estimates]`
//! ([`crate::estimate::declare`]), the personal `calendar.toml`
//! ([`crate::calendar::write`]), and commits or pushes only when the owner
//! asks ([`git`]). Writes and git actions
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

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{Context, Result};
use axum::Router;
use axum::routing::{get, patch, post, put};
use tokio::sync::{broadcast, watch as signal};

use crate::cli::ServeArgs;
use crate::config::Config;
use crate::history::History;
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
    /// What git remembers of each item file — its latest commit, the statuses
    /// it went through — by path relative to the root, recomputed with every
    /// load.
    pub dates: RwLock<History>,
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
        let dates = RwLock::new(History::read(&root));
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
        let files = History::read(&self.root);
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
        .route("/api/items/rank", post(api::rank_item))
        .route("/api/search", get(api::search))
        .route("/api/show", get(api::show))
        .route("/api/dashboard", get(api::dashboard).put(api::pin))
        .route("/api/settings", get(api::settings))
        .route("/api/settings/theme", put(api::theme))
        .route(
            "/api/calendar",
            get(api::calendar)
                .patch(api::change_calendar_entry)
                .delete(api::remove_calendar_entry),
        )
        .route("/api/estimates", patch(api::change_estimates))
        .route("/api/roadmap", get(api::roadmap))
        .route("/api/commit", post(api::commit))
        .route("/api/sync/monday", post(api::sync_monday))
        .route(
            "/api/people",
            get(api::people)
                .post(api::add_person)
                .patch(api::change_person)
                .delete(api::remove_person),
        )
        .route(
            "/api/personas",
            get(api::personas)
                .post(api::add_persona)
                .patch(api::change_persona)
                .delete(api::remove_persona),
        )
        .route(
            "/api/releases",
            post(api::add_release)
                .patch(api::change_release)
                .delete(api::remove_release),
        )
        .route("/api/releases/close", post(api::close_release))
        .route("/api/releases/changelog", get(api::changelog))
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
                "description = \"The *p* project.\"\n\n[[workflow]]\nid = \"a-faire\"\nlabel = \"À faire\"\ncategory = \"todo\"\n\n[[release]]\nname = \"1.0\"\ntarget = 2027-03-01\n\n[[release]]\nname = \"plus-tard\"\nparking = true\n\n[tracker]\nkind = \"monday\"\nboard = 7\n",
            )
            .write(
                "p/tasks/T001-a.md",
                "---\nariane: 2\nnode: p\nid: T001\ntitle: A\nstatus: a-faire\nexternal: [monday:42]\n---\n# A\n",
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
        assert_eq!(task["estimate"]["hours"], 12.0);
        assert_eq!(task["estimate"]["basis"], "default");
    }

    #[tokio::test]
    async fn items_keep_the_kinds_asked_and_add_bodies_on_request() {
        let repo = sample();
        repo.write(
            "p/stories/S001-s.md",
            "---\nariane: 2\nnode: p\nid: S001\ntitle: S\nstatus: a-faire\n---\n# S\n",
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
            "---\nariane: 2\nnode: p\nid: T002\ntitle: B\nstatus: a-faire\ndepends_on: [T001]\n---\n# B\n",
        );
        let (status, body, _) = get(&repo, "/api/show?ref=p%23T001").await;
        assert_eq!(status, StatusCode::OK);
        let tree = Tree::load(repo.root()).unwrap();
        let item = crate::show::find(&tree, "p#T001", None).unwrap();
        let dates = History::read(repo.root());
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
            "---\nariane: 2\nnode: p\nid: T009\ntitle: Été\nstatus: a-faire\n---\nT001 again\n",
        );
        let (status, body, _) = get(&repo, "/api/search?q=t001").await;
        assert_eq!(status, StatusCode::OK);
        let tree = Tree::load(repo.root()).unwrap();
        let dates = History::read(repo.root());
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
    async fn nodes_carry_their_item_count_effective_workflow_releases_and_estimates() {
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
        assert_eq!(
            node["releases"],
            serde_json::json!([
                {"name": "1.0", "parking": false, "target": "2027-03-01", "closed": null},
                {"name": "plus-tard", "parking": true, "target": null, "closed": null},
            ])
        );
        assert_eq!(node["releases_from"], "p");
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
        assert!(root["releases"].is_null());
        assert_eq!(root["estimates_from"]["M"], serde_json::Value::Null);
        assert_eq!(
            root["estimates"],
            serde_json::json!({
                "XS": 2.0, "S": 4.0, "M": 12.0, "L": 24.0, "XL": 48.0,
                "default": "M", "test_ratio": 0.25, "tester": "owner",
            })
        );
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
    async fn the_personal_calendar_is_served() {
        let repo = sample();
        let (status, body, _) = get(&repo, "/api/calendar").await;
        assert_eq!(status, StatusCode::OK);
        let body = json(&body);
        assert_eq!(body["present"], false);
        assert_eq!(body["calendar"], serde_json::Value::Null);
        std::fs::write(
            repo.root().join("calendar.toml"),
            "version = 1\ntimezone = \"Europe/Paris\"\n[node.\"p\"]\nfocus = 0.5\nblocks = [{ days = \"mon-fri\", from = \"09:00\", to = \"17:00\" }]\n[node.\"ghost\"]\nblocks = [{ days = \"sat\", from = \"09:00\", to = \"10:00\" }]\n",
        )
        .unwrap();
        let (status, body, _) = get(&repo, "/api/calendar").await;
        assert_eq!(status, StatusCode::OK);
        let body = json(&body);
        assert_eq!(body["present"], true);
        assert_eq!(body["calendar"]["timezone"], "Europe/Paris");
        let block = &body["calendar"]["nodes"]["p"]["blocks"][0];
        assert_eq!(block["from"], "09:00");
        assert_eq!(block["hours"], 40.0);
        assert_eq!(
            block["weekdays"],
            serde_json::json!(["mon", "tue", "wed", "thu", "fri"])
        );
        assert_eq!(body["problems"][0]["kind"], "unknown_node");
    }

    #[tokio::test]
    async fn the_calendar_is_written_through_the_api_and_an_overlap_is_refused_with_its_blocks() {
        let repo = sample();
        repo.write("q/node.toml", "");
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/calendar",
            serde_json::json!({ "timezone": "Europe/Paris", "days_off": ["2026-12-25"] }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["present"], true);
        assert_eq!(body["calendar"]["timezone"], "Europe/Paris");
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/calendar",
            serde_json::json!({
                "node": "p",
                "focus": 0.5,
                "blocks": [
                    { "days": "sat", "from": "08:00", "to": "09:00" },
                    { "days": "mon-fri", "from": "09:00", "to": "12:00" },
                ],
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["calendar"]["nodes"]["p"]["focus"], 0.5);
        let written = std::fs::read_to_string(repo.root().join("calendar.toml")).unwrap();
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/calendar",
            serde_json::json!({ "node": "q", "blocks": [{ "days": "wed", "from": "11:00", "to": "13:00" }] }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let overlap = &body["problems"][0];
        assert_eq!(overlap["kind"], "overlap");
        assert_eq!(
            (&overlap["node"], &overlap["block"]),
            (&serde_json::json!("p"), &serde_json::json!(1))
        );
        assert_eq!(
            (&overlap["other"], &overlap["other_block"]),
            (&serde_json::json!("q"), &serde_json::json!(0))
        );
        assert!(body["error"].as_str().unwrap().contains("not written"));
        assert_eq!(
            std::fs::read_to_string(repo.root().join("calendar.toml")).unwrap(),
            written
        );
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/calendar",
            serde_json::json!({ "node": "p", "focus": null }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["calendar"]["nodes"]["p"]["focus"], 1.0);
        let (status, body) = send(
            &repo,
            "DELETE",
            "/api/calendar",
            serde_json::json!({ "node": "p" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["calendar"]["nodes"], serde_json::json!({}));
    }

    #[tokio::test]
    async fn a_malformed_calendar_change_is_a_400_and_a_refused_one_a_422() {
        let repo = sample();
        for malformed in [
            serde_json::json!({ "focus": 0.5 }),
            serde_json::json!({ "node": "p", "timezone": "UTC" }),
        ] {
            let (status, _) = send(&repo, "PATCH", "/api/calendar", malformed.clone()).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{malformed}");
        }
        for refused in [
            serde_json::json!({ "node": "nowhere", "focus": 0.5 }),
            serde_json::json!({ "node": "p" }),
            serde_json::json!({ "node": "p", "focus": 2 }),
        ] {
            let (status, body) = send(&repo, "PATCH", "/api/calendar", refused.clone()).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
            assert!(body["error"].is_string(), "{body}");
        }
        let (status, _) = send(
            &repo,
            "DELETE",
            "/api/calendar",
            serde_json::json!({ "node": "p" }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!repo.root().join("calendar.toml").exists());
    }

    #[tokio::test]
    async fn estimates_are_set_on_the_node_s_own_table_and_each_key_says_where_it_comes_from() {
        let repo = sample();
        repo.write("p/c/node.toml", "");
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/estimates",
            serde_json::json!({ "node": "p", "estimates": { "M": 10, "test_ratio": 0.5 } }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["estimates"]["M"], 10.0);
        assert_eq!(body["estimates_from"]["M"], "p");
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/estimates",
            serde_json::json!({ "node": "p/c", "estimates": { "default": "S" } }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["estimates"]["M"], 10.0);
        assert_eq!(
            (
                &body["estimates_from"]["M"],
                &body["estimates_from"]["default"],
                &body["estimates_from"]["XS"]
            ),
            (
                &serde_json::json!("p"),
                &serde_json::json!("p/c"),
                &serde_json::Value::Null
            )
        );
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/estimates",
            serde_json::json!({ "node": "p", "estimates": { "M": null } }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["estimates"]["M"], 12.0);
        assert_eq!(body["estimates_from"]["M"], serde_json::Value::Null);
        let before = std::fs::read_to_string(repo.root().join("tree/p/node.toml")).unwrap();
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/estimates",
            serde_json::json!({ "node": "p", "estimates": { "S": 3, "test_ratio": -1 } }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert!(
            body["error"].as_str().unwrap().contains("negative"),
            "{body}"
        );
        let (status, _) = send(
            &repo,
            "PATCH",
            "/api/estimates",
            serde_json::json!({ "node": "p", "estimates": { "S": [3] } }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            std::fs::read_to_string(repo.root().join("tree/p/node.toml")).unwrap(),
            before
        );
    }

    /// What `ariane roadmap <node> --json` prints for `repo`, from the moment
    /// the response `served` starts from.
    fn roadmap_command(repo: &Repo, node: &str, served: &serde_json::Value) -> serde_json::Value {
        let tree = Tree::load(repo.root()).unwrap();
        let now: jiff::Timestamp = served["now"].as_str().unwrap().parse().unwrap();
        let forecast = crate::roadmap::forecast(
            repo.root(),
            &tree,
            Some(&History::read(repo.root())),
            &NodePath::new(node),
            now,
        )
        .unwrap();
        serde_json::to_value(forecast).unwrap()
    }

    #[tokio::test]
    async fn the_roadmap_is_the_same_json_as_the_roadmap_command() {
        let repo = sample();
        repo.write(
            "p/q/tasks/T001-b.md",
            "---\nariane: 2\nnode: p/q\nid: T001\ntitle: B\nstatus: a-faire\nrelease: '1.0'\nsize: S\n---\n# B\n",
        );
        let (status, body, _) = get(&repo, "/api/roadmap?node=p").await;
        assert_eq!(status, StatusCode::OK);
        let body = json(&body);
        assert_eq!(body, roadmap_command(&repo, "p", &body));
        assert_eq!(
            (body["node"].as_str(), body["dated"].as_bool()),
            (Some("p"), Some(false))
        );
        assert_eq!(body["carrier"], serde_json::Value::Null);
        assert_eq!(body["releases"][0]["name"], "1.0");
        assert_eq!(body["unscheduled"][0]["reference"], "p#T001");
        assert_eq!(body["unscheduled"][0]["reason"], "no_release");
        std::fs::write(
            repo.root().join("calendar.toml"),
            "version = 1\ntimezone = \"Europe/Paris\"\n[node.\"p\"]\nblocks = [{ days = \"mon-sun\", from = \"00:00\", to = \"24:00\" }]\n",
        )
        .unwrap();
        let (status, body, _) = get(&repo, "/api/roadmap?node=p%2Fq").await;
        assert_eq!(status, StatusCode::OK);
        let body = json(&body);
        assert_eq!(body, roadmap_command(&repo, "p/q", &body));
        assert_eq!(
            (body["node"].as_str(), body["carrier"].as_str()),
            (Some("p/q"), Some("p"))
        );
        assert_eq!(body["dated"], true);
        assert_eq!(body["timezone"], "Europe/Paris");
        let work = &body["releases"][0]["epics"][0]["stories"][0]["work"][0];
        assert_eq!(work["reference"], "p/q#T001");
        assert!(work["start"]["at"].is_string());
        assert_eq!(body["unscheduled"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn the_roadmap_needs_a_node_of_the_tree() {
        let repo = sample();
        let (status, body, _) = get(&repo, "/api/roadmap").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(json(&body)["error"].as_str().unwrap().contains("node"));
        let (status, body, _) = get(&repo, "/api/roadmap?node=nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(json(&body)["error"].as_str().unwrap().contains("nope"));
    }

    #[tokio::test]
    async fn syncing_an_item_without_a_monday_binding_sends_nothing() {
        let repo = sample();
        repo.write(
            "q/tasks/T1-a.md",
            "---\nariane: 2\nnode: q\nid: T1\ntitle: A\nstatus: todo\n---\n",
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
    async fn the_personas_are_listed_added_renamed_and_kept_while_named() {
        let repo = sample();
        repo.write(
            "p/stories/S001-a.md",
            "---\nariane: 2\nnode: p\nid: S001\ntitle: A\nstatus: a-faire\npersonas: [\"user:rh\"]\n---\n",
        );
        let (status, body) = send(
            &repo,
            "POST",
            "/api/personas",
            serde_json::json!({ "node": "p", "id": "user:rh", "name": "RH" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body[0]["personas"][0]["used_by"][0], "p#S001");
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/personas",
            serde_json::json!({ "node": "p", "id": "user:rh", "name": "Responsable RH", "rename_to": "user:hr" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body[0]["personas"][0]["id"], "user:hr");
        assert_eq!(body[0]["personas"][0]["name"], "Responsable RH");
        assert_eq!(body[0]["personas"][0]["used_by"][0], "p#S001");
        let (status, body) = send(
            &repo,
            "DELETE",
            "/api/personas",
            serde_json::json!({ "node": "p", "id": "user:hr" }),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["items"][0], "p#S001");
        let (status, _) = send(
            &repo,
            "POST",
            "/api/personas",
            serde_json::json!({ "node": "p", "id": "rh", "name": "X" }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn the_people_are_listed_added_renamed_and_kept_while_awaited() {
        let repo = sample();
        repo.write(
            "p/tasks/T002-b.md",
            "---\nariane: 2\nnode: p\nid: T002\ntitle: B\nstatus: a-faire\n---\n",
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
    async fn releases_are_added_moved_renamed_and_kept_while_named() {
        let repo = sample();
        repo.write(
            "p/tasks/T002-b.md",
            "---\nariane: 2\nnode: p\nid: T002\ntitle: B\nstatus: a-faire\nrelease: '1.0'\n---\n",
        );
        let names = |body: &serde_json::Value| -> Vec<String> {
            body["releases"]
                .as_array()
                .unwrap()
                .iter()
                .map(|release| release["name"].as_str().unwrap().to_owned())
                .collect()
        };
        let (status, body) = send(
            &repo,
            "POST",
            "/api/releases",
            serde_json::json!({ "node": "p", "name": "0.9", "place": "top", "target": "2027-01-15" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["releases_from"], "p");
        assert_eq!(names(&body), ["0.9", "1.0", "plus-tard"]);
        assert_eq!(body["releases"][0]["target"], "2027-01-15");
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/releases",
            serde_json::json!({ "node": "p", "name": "1.0", "place": "before", "other": "0.9", "rename_to": "mango" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(names(&body), ["mango", "0.9", "plus-tard"]);
        assert_eq!(body["rewritten"], serde_json::json!(["p#T002"]));
        let task = std::fs::read_to_string(repo.root().join("tree/p/tasks/T002-b.md")).unwrap();
        assert!(task.contains("release: mango\n"), "{task}");
        let (status, body) = send(
            &repo,
            "DELETE",
            "/api/releases",
            serde_json::json!({ "node": "p", "name": "mango" }),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["items"], serde_json::json!(["p#T002"]));
        let (status, body) = send(
            &repo,
            "DELETE",
            "/api/releases",
            serde_json::json!({ "node": "p", "name": "0.9" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(names(&body), ["mango", "plus-tard"]);
        let text = std::fs::read_to_string(repo.root().join("tree/p/node.toml")).unwrap();
        assert!(text.starts_with("description = \"The *p* project.\"\n\n[[workflow]]"));
        assert!(
            text.ends_with("[tracker]\nkind = \"monday\"\nboard = 7\n"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn a_release_is_planned_refused_carried_over_and_closed_and_its_changelog_given() {
        let repo = sample();
        repo.write(
            "p/node.toml",
            "[[release]]\nname = \"1.0\"\n\n[[release]]\nname = \"2.0\"\n\n[[release]]\nname = \"plus-tard\"\nparking = true\n",
        )
        .write(
            "p/tasks/T001-a.md",
            "---\nariane: 2\nnode: p\nid: T001\ntitle: Open\nstatus: todo\nrelease: '1.0'\n---\n# A\n",
        )
        .write(
            "p/tasks/T002-b.md",
            "---\nariane: 2\nnode: p\nid: T002\ntitle: Shipped\nstatus: done\nrelease: '1.0'\n---\n# B\n\nSee [T001](T001-a.md).\n",
        );
        let close = |carry_over: bool, dry_run: bool| serde_json::json!({ "node": "p", "name": "1.0", "carry_over": carry_over, "dry_run": dry_run });
        let (status, body) = send(&repo, "POST", "/api/releases/close", close(false, true)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["blocked"], true);
        assert_eq!(body["carry_to"], "2.0");
        assert_eq!(body["open"][0]["id"], "T001");
        assert_eq!(body["archived"][0]["to"], "tree/p/archives/tasks/T002-b.md");
        assert_eq!(
            body["changelog"]["markdown"],
            "## 1.0\n\n### Added\n\n- **Shipped**. (T002)\n"
        );
        let (status, body) = send(&repo, "POST", "/api/releases/close", close(false, false)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["items"], serde_json::json!(["T001"]));
        assert!(repo.root().join("tree/p/tasks/T002-b.md").exists());
        let (status, body) = send(
            &repo,
            "POST",
            "/api/releases/close",
            serde_json::json!({ "node": "p", "name": "plus-tard" }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let (status, body) = send(&repo, "POST", "/api/releases/close", close(true, false)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["dry_run"], false);
        let archived =
            std::fs::read_to_string(repo.root().join("tree/p/archives/tasks/T002-b.md")).unwrap();
        assert!(
            archived.contains("See [T001](../../tasks/T001-a.md)."),
            "{archived}"
        );
        let open = std::fs::read_to_string(repo.root().join("tree/p/tasks/T001-a.md")).unwrap();
        assert!(open.contains("release: '2.0'\n"), "{open}");
        let text = std::fs::read_to_string(repo.root().join("tree/p/node.toml")).unwrap();
        assert!(text.contains("name = \"1.0\"\nclosed = "), "{text}");
        let (status, body, _) = get(&repo, "/api/releases/changelog?node=p&name=1.0").await;
        assert_eq!(status, StatusCode::OK);
        let entry: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            entry["sections"][0]["lines"][0]["changes"][0]["archived"],
            true
        );
        let (status, _, _) = get(&repo, "/api/releases/changelog?node=p&name=9.9").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/items",
            serde_json::json!({ "path": "tree/p/tasks/T001-a.md", "changes": { "release": { "from": "2.0", "to": "1.0" } } }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }

    #[tokio::test]
    async fn a_release_change_sets_its_parking_mark_and_clears_or_keeps_its_target() {
        let repo = sample();
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/releases",
            serde_json::json!({ "node": "p", "name": "plus-tard", "parking": false, "target": "2027-06-01" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["releases"][1]["parking"], false);
        assert_eq!(body["releases"][1]["target"], "2027-06-01");
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/releases",
            serde_json::json!({ "node": "p", "name": "1.0", "parking": true }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["releases"][0]["parking"], true);
        assert_eq!(body["releases"][0]["target"], "2027-03-01");
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/releases",
            serde_json::json!({ "node": "p", "name": "1.0", "target": null }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["releases"][0]["target"].is_null(), "{body}");
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/releases",
            serde_json::json!({ "node": "p", "name": "1.0", "target": "someday" }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let text = std::fs::read_to_string(repo.root().join("tree/p/node.toml")).unwrap();
        assert!(
            text.contains("[[release]]\nname = \"1.0\"\nparking = true\n\n[[release]]\nname = \"plus-tard\"\ntarget = 2027-06-01\n"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn a_release_write_is_refused_with_its_reason_and_an_inherited_list_names_its_node() {
        let repo = sample();
        repo.write(
            "p/c/tasks/T001-c.md",
            "---\nariane: 2\nnode: p/c\nid: T001\ntitle: C\nstatus: a-faire\n---\n",
        );
        let (status, body) = send(
            &repo,
            "POST",
            "/api/releases",
            serde_json::json!({ "node": "p/c", "name": "2.0" }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["declared_in"], "p");
        assert!(body["error"].as_str().unwrap().contains("`p`"), "{body}");
        for (method, body) in [
            ("POST", serde_json::json!({ "node": "p", "name": "1.0" })),
            ("POST", serde_json::json!({ "node": "p", "name": " " })),
            (
                "POST",
                serde_json::json!({ "node": "p", "name": "2.0", "target": "soon" }),
            ),
            (
                "PATCH",
                serde_json::json!({ "node": "p", "name": "9.9", "place": "top" }),
            ),
        ] {
            let (status, answer) = send(&repo, method, "/api/releases", body.clone()).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body} {answer}");
            assert!(answer["error"].is_string());
        }
        for body in [
            serde_json::json!({ "node": "p", "name": "1.0" }),
            serde_json::json!({ "node": "p", "name": "1.0", "place": "after" }),
        ] {
            let (status, _) = send(&repo, "PATCH", "/api/releases", body.clone()).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        }
    }

    #[tokio::test]
    async fn an_item_is_planned_in_a_release_through_the_item_patch_and_browsers_are_told() {
        let repo = sample();
        let path = "tree/p/tasks/T001-a.md";
        let plan = |from: serde_json::Value, to: serde_json::Value| serde_json::json!({ "path": path, "changes": { "release": { "from": from, "to": to } } });
        let none = serde_json::Value::Null;
        let state = state(&repo);
        let mut changes = state.changes.subscribe();
        let request = Request::builder()
            .method("PATCH")
            .uri("/api/items")
            .header("content-type", "application/json")
            .body(Body::from(plan(none.clone(), "1.0".into()).to_string()))
            .unwrap();
        let response = router(state.clone()).oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(changes.try_recv().is_ok());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let row: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(row["release"], "1.0");
        assert_eq!(row["effective_release"]["position"], 0);
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/items",
            plan("1.0".into(), "9.9".into()),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            body["error"].as_str().unwrap().contains("1.0, plus-tard"),
            "{body}"
        );
        let (status, body) = send(
            &repo,
            "PATCH",
            "/api/items",
            plan(none.clone(), "plus-tard".into()),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["current"]["release"], "1.0");
        let (status, _) = send(&repo, "PATCH", "/api/items", plan("1.0".into(), none)).await;
        assert_eq!(status, StatusCode::OK);
        let text = std::fs::read_to_string(repo.root().join(path)).unwrap();
        assert!(!text.contains("release"), "{text}");
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
            "---\nariane: 2\nnode: p\nid: T002\ntitle: B\nstatus: a-faire\n---\n# B\n",
        );
        state.reload().unwrap();
        assert_eq!(state.tree.read().unwrap().items.len(), 3);
        assert!(changes.try_recv().is_ok());
    }
}

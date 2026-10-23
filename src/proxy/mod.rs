//! `ariane proxy`: local names for the interface and the projects' development
//! servers (ADR-0016).
//!
//! Three pieces, from the browser inward:
//!
//! 1. **The names.** `/etc/hosts` sends every routed name to `127.0.0.2`, an
//!    alias of the loopback interface that only Ariane uses.
//! 2. **The redirect.** A `pf` rule sends port 80 of `127.0.0.2` to the front
//!    door. Port 80 is out of reach of a process that is not root (only a bind
//!    on every interface is exempt, and Docker holds that one), so the alias,
//!    the rule and the names are set up once with `sudo` by [`machine`], and
//!    last until the next reboot.
//! 3. **The front door** ([`door`]): an HTTP server on `127.0.0.1:<proxy.port>`
//!    that reads the `Host` of each request and forwards it to the port of the
//!    matching route. `ariane serve` opens it next to the interface; `ariane
//!    proxy` opens it alone when the server does not run.
//!
//! The routes ([`Table`]) are the interface's own, `server.host` of the
//! configuration, and one per node whose `node.toml` has a `[proxy]` table.

pub mod door;
mod machine;

use std::net::Ipv4Addr;
use std::process::ExitCode;

use anyhow::{Result, bail};

use crate::cli::{ProxyArgs, ProxyCommand};
use crate::config::{self, Config};
use crate::tree::{NodePath, Tree};

/// The loopback alias the routed names point to, kept apart from `127.0.0.1`
/// so that its port 80 can be redirected without touching anyone else's.
pub const ALIAS: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 2);

/// One name the front door answers to, and where it forwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// The host name typed in the browser, such as `dreamcatchr.local`.
    pub host: String,
    /// The port of `127.0.0.1` the requests are forwarded to.
    pub port: u16,
    /// The node that declares the route; `None` for the interface itself.
    pub node: Option<NodePath>,
}

impl Route {
    /// Who declares the route, for messages: the node, or the configuration.
    pub fn owner(&self) -> String {
        match &self.node {
            Some(node) => format!("node {node}"),
            None => "the interface (`server.host`)".to_owned(),
        }
    }
}

/// Every route, and the `[proxy]` tables that could not become one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    /// The routes, the interface's first, then the nodes' in path order.
    pub routes: Vec<Route>,
    /// Why a declared route was left out: an invalid or already taken name.
    pub problems: Vec<String>,
}

impl Table {
    /// The routes of the tree: the interface's on `server`, then one per
    /// `[proxy]` table. A name already taken keeps its first owner.
    pub fn of(tree: &Tree, server: &config::Server) -> Table {
        let mut table = Table {
            routes: vec![Route {
                host: server.host.clone(),
                port: server.port,
                node: None,
            }],
            problems: Vec::new(),
        };
        for (node, file) in &tree.configs.files {
            let Some(proxy) = &file.proxy else {
                continue;
            };
            if !config::is_host(&proxy.host) {
                table.problems.push(format!(
                    "node {node}: `{}` is not a host name such as `{}.local`",
                    proxy.host,
                    node.as_str().rsplit('/').next().unwrap_or("project")
                ));
                continue;
            }
            if proxy.port == 0 {
                table
                    .problems
                    .push(format!("node {node}: port 0 cannot be forwarded to"));
                continue;
            }
            if let Some(taken) = table.find(&proxy.host) {
                table.problems.push(format!(
                    "node {node}: `{}` is already the name of {}",
                    proxy.host,
                    taken.owner()
                ));
                continue;
            }
            table.routes.push(Route {
                host: proxy.host.clone(),
                port: proxy.port,
                node: Some(node.clone()),
            });
        }
        table
    }

    /// The route named `host`, compared without case, port or final dot.
    pub fn find(&self, host: &str) -> Option<&Route> {
        let host = normalize(host);
        self.routes.iter().find(|route| route.host == host)
    }

    /// Only the routes of the nodes `names`, each a node path or its last
    /// segment. With no name, the whole table.
    ///
    /// # Errors
    ///
    /// Fails when a name matches no route, or the last segments of several.
    pub fn select(self, names: &[String]) -> Result<Table> {
        if names.is_empty() {
            return Ok(self);
        }
        let mut routes = Vec::new();
        for name in names {
            let matching: Vec<&Route> = self
                .routes
                .iter()
                .filter(|route| {
                    route.node.as_ref().is_some_and(|node| {
                        node.as_str() == name || node.as_str().rsplit('/').next() == Some(name)
                    })
                })
                .collect();
            match matching.as_slice() {
                [route] => routes.push((*route).clone()),
                [] => bail!(
                    "no node `{name}` declares a route; add `[proxy]` with `host` and `port` to its node.toml"
                ),
                several => {
                    let nodes: Vec<String> = several.iter().map(|route| route.owner()).collect();
                    bail!("`{name}` names several nodes: {}", nodes.join(", "));
                }
            }
        }
        Ok(Table {
            routes,
            problems: self.problems,
        })
    }
}

/// A `Host` header as a route name: lowercase, without port or final dot.
pub fn normalize(host: &str) -> String {
    let name = match host.rsplit_once(':') {
        Some((name, port)) if port.bytes().all(|byte| byte.is_ascii_digit()) => name,
        _ => host,
    };
    name.trim_end_matches('.').to_ascii_lowercase()
}

/// Runs `ariane proxy`: the front door alone, or one of its subcommands.
///
/// # Errors
///
/// Fails when the configuration is invalid, a named node declares no route,
/// the front door's port is taken, or a system command fails.
pub fn run(tree: &Tree, args: &ProxyArgs) -> Result<ExitCode> {
    let config = config::load(&config::file())?.config;
    let table = Table::of(tree, &config.server);
    for problem in &table.problems {
        eprintln!("ariane proxy: {problem}");
    }
    let machine = machine::Machine::current();
    match args.command {
        Some(ProxyCommand::Setup) => machine::setup(&machine, &table, config.proxy.port),
        Some(ProxyCommand::Status) => Ok(machine::status(&machine, &config, &table)),
        Some(ProxyCommand::Teardown) => teardown(),
        None => alone(&config, table.select(&args.nodes)?),
    }
}

/// Runs `ariane proxy teardown`, which needs no content repository.
///
/// # Errors
///
/// Fails on another system than macOS, or when the hosts file cannot be read.
pub fn teardown() -> Result<ExitCode> {
    machine::teardown(&machine::Machine::current())
}

/// Opens the front door without the interface, until Ctrl-C.
///
/// # Errors
///
/// Fails when the runtime cannot start or the front door's port is taken,
/// which usually means `ariane serve` already holds it.
fn alone(config: &Config, table: Table) -> Result<ExitCode> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let listener = door::bind(config.proxy.port).await.map_err(|error| {
            anyhow::anyhow!(
                "{error:#}; `ariane serve` may already hold the front door, and it forwards every route"
            )
        })?;
        for route in &table.routes {
            eprintln!(
                "ariane proxy: http://{} → 127.0.0.1:{} ({})",
                route.host,
                route.port,
                route.owner()
            );
        }
        eprintln!("ariane proxy: front door on 127.0.0.1:{} (Ctrl-C to stop)", config.proxy.port);
        let lookup: door::Lookup = std::sync::Arc::new(move |host| table.find(host).cloned());
        tokio::select! {
            served = door::serve(listener, lookup) => served,
            _ = tokio::signal::ctrl_c() => Ok(()),
        }
    })?;
    Ok(ExitCode::SUCCESS)
}

/// Unit tests of the route table.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// A tree with two projects declaring routes, and a server on 27317.
    fn sample() -> (Repo, config::Server) {
        let repo = Repo::new();
        repo.write("slashome/dreamcatchr/INDEX.md", "# d\n")
            .write(
                "slashome/dreamcatchr/node.toml",
                "[proxy]\nhost = \"dreamcatchr.local\"\nport = 3000\n",
            )
            .write("cmc/lib/INDEX.md", "# l\n")
            .write(
                "cmc/lib/node.toml",
                "[proxy]\nhost = \"lib.local\"\nport = 6006\n",
            );
        (repo, config::Server::default())
    }

    #[test]
    fn the_interface_comes_first_then_each_declared_node() {
        let (repo, server) = sample();
        let table = Table::of(&Tree::load(repo.root()).unwrap(), &server);
        let hosts: Vec<_> = table
            .routes
            .iter()
            .map(|route| route.host.as_str())
            .collect();
        assert_eq!(hosts, ["ariane.local", "lib.local", "dreamcatchr.local"]);
        assert_eq!(table.routes[0].port, config::DEFAULT_PORT);
        assert_eq!(table.routes[0].node, None);
        assert!(table.problems.is_empty());
    }

    #[test]
    fn a_host_is_found_whatever_its_case_port_or_final_dot() {
        let (repo, server) = sample();
        let table = Table::of(&Tree::load(repo.root()).unwrap(), &server);
        assert_eq!(table.find("Dreamcatchr.Local:80").unwrap().port, 3000);
        assert_eq!(table.find("lib.local.").unwrap().port, 6006);
        assert!(table.find("other.local").is_none());
    }

    #[test]
    fn a_taken_or_invalid_name_is_left_out_and_reported() {
        let (repo, server) = sample();
        repo.write("p/INDEX.md", "# p\n")
            .write(
                "p/node.toml",
                "[proxy]\nhost = \"ariane.local\"\nport = 1\n",
            )
            .write("q/INDEX.md", "# q\n")
            .write("q/node.toml", "[proxy]\nhost = \"Q Local\"\nport = 2\n");
        let table = Table::of(&Tree::load(repo.root()).unwrap(), &server);
        assert_eq!(table.routes.len(), 3);
        assert_eq!(table.problems.len(), 2);
        assert!(table.problems[0].contains("already the name of the interface"));
        assert!(table.problems[1].contains("not a host name"));
    }

    #[test]
    fn nodes_are_selected_by_path_or_last_segment() {
        let (repo, server) = sample();
        let table = Table::of(&Tree::load(repo.root()).unwrap(), &server);
        let selected = table.clone().select(&["dreamcatchr".to_owned()]).unwrap();
        assert_eq!(selected.routes.len(), 1);
        assert_eq!(selected.routes[0].host, "dreamcatchr.local");
        let selected = table.clone().select(&["cmc/lib".to_owned()]).unwrap();
        assert_eq!(selected.routes[0].host, "lib.local");
        assert_eq!(table.clone().select(&[]).unwrap(), table);
        assert!(table.select(&["nowhere".to_owned()]).is_err());
    }

    #[test]
    fn host_headers_are_normalized() {
        assert_eq!(normalize("Ariane.Local:80"), "ariane.local");
        assert_eq!(normalize("ariane.local."), "ariane.local");
        assert_eq!(normalize("ariane.local"), "ariane.local");
    }
}

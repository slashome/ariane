//! Writing a node's route: the `[proxy]` table of its `node.toml`
//! (ADR-0016), as the interface edits it.
//!
//! The file is edited in place, comments and other tables kept; a node
//! without a `node.toml` gets one holding only `[proxy]`, and a file left
//! empty by a removal is deleted. Nothing is committed: the content repository
//! is left dirty for the owner to commit.

use anyhow::{Context, Result};
use toml_edit::{Item, Table as TomlTable, TableLike, Value};

use super::Table;
use crate::config::{self, Config};
use crate::node::FILE as NODE_FILE;
use crate::tree::{NodePath, Tree};

/// Gives `node` the route `host` → `port`, replacing the one it has.
///
/// Refuses, with the reason, a node that is not in the tree, a host that is
/// not a lowercase host name or that another route already has, and a port
/// outside 1–65535 or taken by the front door or the interface.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read, parsed or written.
pub fn set(
    tree: &Tree,
    config: &Config,
    node: &NodePath,
    host: &str,
    port: i64,
) -> Result<Result<(), String>> {
    if let Err(refusal) = known(tree, node) {
        return Ok(Err(refusal));
    }
    if !config::is_host(host) {
        return Ok(Err(format!(
            "`{host}` is not a host name: lowercase letters, digits and inner hyphens, separated by dots, such as `{}.local`",
            node.as_str()
                .rsplit('/')
                .next()
                .filter(|last| !last.is_empty())
                .unwrap_or("project")
        )));
    }
    if let Some(taken) = Table::of(tree, &config.server).find(host)
        && taken.node.as_ref() != Some(node)
    {
        return Ok(Err(format!(
            "`{host}` is already the name of {}",
            taken.owner()
        )));
    }
    let Some(port) = u16::try_from(port).ok().filter(|port| *port != 0) else {
        return Ok(Err(format!("port {port} is not between 1 and 65535")));
    };
    if port == config.proxy.port {
        return Ok(Err(format!(
            "port {port} is the front door's (`proxy.port`)"
        )));
    }
    if port == config.server.port {
        return Ok(Err(format!(
            "port {port} is the interface's (`server.port`)"
        )));
    }
    crate::node::edit(&tree.node_dir(node).join(NODE_FILE), |document| {
        if document
            .get("proxy")
            .and_then(Item::as_table_like)
            .is_none()
        {
            document.insert("proxy", Item::Table(TomlTable::new()));
        }
        if let Some(proxy) = document.get_mut("proxy").and_then(Item::as_table_like_mut) {
            put(proxy, "host", Value::from(host));
            put(proxy, "port", Value::from(i64::from(port)));
        }
        Ok(())
    })
}

/// Sets `key` of `table` to `new`, keeping the spacing and the comment
/// around the value it replaces.
fn put(table: &mut dyn TableLike, key: &str, new: Value) {
    match table.get_mut(key).and_then(Item::as_value_mut) {
        Some(old) => {
            let decor = old.decor().clone();
            *old = new;
            *old.decor_mut() = decor;
        }
        None => {
            table.insert(key, Item::Value(new));
        }
    }
}

/// Takes the route of `node` away: its `[proxy]` table, and its `node.toml`
/// when nothing else is left in it.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read, parsed, written or deleted.
pub fn remove(tree: &Tree, node: &NodePath) -> Result<Result<(), String>> {
    if let Err(refusal) = known(tree, node) {
        return Ok(Err(refusal));
    }
    let file = tree.node_dir(node).join(NODE_FILE);
    let removed = crate::node::edit(&file, |document| match document.remove("proxy") {
        Some(_) => Ok(()),
        None => Err(format!("node {node} declares no route")),
    })?;
    if removed.is_ok() {
        let left = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        if left.trim().is_empty() {
            std::fs::remove_file(&file).with_context(|| format!("deleting {}", file.display()))?;
        }
    }
    Ok(removed)
}

/// Refuses a node the tree does not have.
fn known(tree: &Tree, node: &NodePath) -> Result<(), String> {
    if tree.nodes.iter().any(|known| known.path == *node) {
        Ok(())
    } else {
        Err(format!("`{node}` is not a node of the tree"))
    }
}

/// Unit tests of the route writes, against fixture repositories.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// A tree with a node holding a commented `node.toml` and a node without one.
    fn sample() -> Repo {
        let repo = Repo::new();
        repo.write(
            "p/node.toml",
            "# the p project\ndescription = \"P.\"\n\n[proxy]\nhost = \"p.local\" # its dev server\nport = 3000\n\n[tracker]\nkind = \"monday\"\n",
        )
        .write(
            "q/tasks/T1-a.md",
            "---\nariane: 1\nnode: q\nid: T1\ntitle: A\nstatus: todo\n---\n",
        );
        repo
    }

    /// The text of the `node.toml` of `node`.
    fn read(repo: &Repo, node: &str) -> Option<String> {
        std::fs::read_to_string(repo.root().join("tree").join(node).join(NODE_FILE)).ok()
    }

    #[test]
    fn a_route_is_changed_in_place_keeping_comments_and_other_tables() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let node = NodePath::new("p");
        set(&tree, &Config::default(), &node, "pp.local", 3001)
            .unwrap()
            .unwrap();
        assert_eq!(
            read(&repo, "p").unwrap(),
            "# the p project\ndescription = \"P.\"\n\n[proxy]\nhost = \"pp.local\" # its dev server\nport = 3001\n\n[tracker]\nkind = \"monday\"\n"
        );
        let tree = Tree::load(repo.root()).unwrap();
        assert_eq!(
            Table::of(&tree, &config::Server::default())
                .find("pp.local")
                .unwrap()
                .port,
            3001
        );
    }

    #[test]
    fn a_node_without_a_node_toml_gets_one_holding_only_its_route() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        set(
            &tree,
            &Config::default(),
            &NodePath::new("q"),
            "q.local",
            5173,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            read(&repo, "q").unwrap(),
            "[proxy]\nhost = \"q.local\"\nport = 5173\n"
        );
        let tree = Tree::load(repo.root()).unwrap();
        remove(&tree, &NodePath::new("q")).unwrap().unwrap();
        assert_eq!(read(&repo, "q"), None);
    }

    #[test]
    fn removing_a_route_keeps_the_rest_of_the_file() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        remove(&tree, &NodePath::new("p")).unwrap().unwrap();
        let left = read(&repo, "p").unwrap();
        assert!(left.starts_with("# the p project\ndescription = \"P.\"\n"));
        assert!(left.contains("[tracker]"));
        assert!(!left.contains("proxy"));
        let tree = Tree::load(repo.root()).unwrap();
        assert!(
            remove(&tree, &NodePath::new("p"))
                .unwrap()
                .unwrap_err()
                .contains("declares no route")
        );
    }

    #[test]
    fn invalid_or_taken_names_and_ports_are_refused_and_nothing_is_written() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let config = Config::default();
        let q = NodePath::new("q");
        let refused = |node: &NodePath, host: &str, port: i64| {
            set(&tree, &config, node, host, port).unwrap().unwrap_err()
        };
        assert!(refused(&q, "Q.local", 1).contains("not a host name"));
        assert!(refused(&q, "q.local:80", 1).contains("not a host name"));
        assert!(refused(&q, "p.local", 1).contains("already the name of node p"));
        assert!(refused(&q, "ariane.local", 1).contains("the interface"));
        assert!(refused(&q, "q.local", 0).contains("not between 1 and 65535"));
        assert!(refused(&q, "q.local", 65536).contains("not between 1 and 65535"));
        assert!(refused(&q, "q.local", i64::from(config.proxy.port)).contains("front door"));
        assert!(refused(&q, "q.local", i64::from(config.server.port)).contains("interface's"));
        assert!(refused(&NodePath::new("nowhere"), "n.local", 1).contains("not a node"));
        assert_eq!(read(&repo, "q"), None);
        set(&tree, &config, &NodePath::new("p"), "p.local", 4000)
            .unwrap()
            .unwrap();
    }
}

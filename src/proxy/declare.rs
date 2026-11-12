//! Writing a node's routes: the `[proxy]` table, or the `[[proxy]]` entries,
//! of its `node.toml` (ADR-0016), as the interface edits them.
//!
//! The file is edited in place, comments and other tables kept. A node keeps
//! a single `[proxy]` table while it has one route and switches to `[[proxy]]`
//! entries from the second one; a node without a `node.toml` gets one holding
//! only `[proxy]`, and a file left empty by a removal is deleted. Nothing is
//! committed: the content repository is left dirty for the owner to commit.

use anyhow::{Context, Result};
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table as TomlTable, TableLike, Value};

use super::{Table, normalize};
use crate::config::{self, Config};
use crate::node::FILE as NODE_FILE;
use crate::tree::{NodePath, Tree};

/// Gives `node` the route `host` → `port`. With `replaces`, that route of the
/// node is rewritten; without, the node's route named `host` gets the port, or
/// a new route is added next to the node's others.
///
/// Refuses, with the reason, a node that is not in the tree, a `replaces` the
/// node does not have, a host that is not a lowercase host name or that another
/// route already has, and a port outside 1–65535 or taken by the front door or
/// the interface.
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
    replaces: Option<&str>,
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
        && !(taken.node.as_ref() == Some(node)
            && replaces.is_none_or(|old| normalize(old) == taken.host))
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
        write(
            document,
            replaces.unwrap_or(host),
            replaces.is_some(),
            host,
            port,
        )
        .map_err(|()| format!("node {node} has no route `{}`", replaces.unwrap_or(host)))
    })
}

/// Writes `host` → `port` over the entry named `key`, or, unless `replacing`,
/// as a new entry; `Err` when `replacing` finds no entry named `key`.
fn write(
    document: &mut DocumentMut,
    key: &str,
    replacing: bool,
    host: &str,
    port: u16,
) -> Result<(), ()> {
    let Some(item) = document.get_mut("proxy") else {
        if replacing {
            return Err(());
        }
        document.insert("proxy", Item::Table(route(host, port)));
        return Ok(());
    };
    if let Some(table) = item.as_table_like_mut()
        && named(table, key)
    {
        put(table, "host", Value::from(host));
        put(table, "port", Value::from(i64::from(port)));
        return Ok(());
    }
    if let Some(entries) = item.as_array_of_tables_mut()
        && let Some(table) = entries.iter_mut().find(|table| named(*table, key))
    {
        put(table, "host", Value::from(host));
        put(table, "port", Value::from(i64::from(port)));
        return Ok(());
    }
    if replacing {
        return Err(());
    }
    let mut entries = match std::mem::take(item).into_array_of_tables() {
        Ok(entries) => entries,
        Err(single) => {
            let mut entries = ArrayOfTables::new();
            if let Ok(table) = single.into_table() {
                entries.push(table);
            }
            entries
        }
    };
    entries.push(route(host, port));
    *item = Item::ArrayOfTables(entries);
    Ok(())
}

/// A new route table, `host` then `port`.
fn route(host: &str, port: u16) -> TomlTable {
    let mut table = TomlTable::new();
    table.insert("host", Item::Value(Value::from(host)));
    table.insert("port", Item::Value(Value::from(i64::from(port))));
    table
}

/// Whether the route `table` is named `host`.
fn named(table: &dyn TableLike, host: &str) -> bool {
    table
        .get("host")
        .and_then(Item::as_str)
        .is_some_and(|name| normalize(name) == normalize(host))
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

/// Takes routes of `node` away: the one named `host`, or every one without
/// `host`. A node left with one route gets back a single `[proxy]` table, and
/// its `node.toml` is deleted when nothing else is left in it.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read, parsed, written or deleted.
pub fn remove(tree: &Tree, node: &NodePath, host: Option<&str>) -> Result<Result<(), String>> {
    if let Err(refusal) = known(tree, node) {
        return Ok(Err(refusal));
    }
    let file = tree.node_dir(node).join(NODE_FILE);
    let removed = crate::node::edit(&file, |document| {
        let taken = match host {
            None => document.remove("proxy").is_some(),
            Some(host) => take(document, host),
        };
        if taken {
            Ok(())
        } else {
            Err(match host {
                None => format!("node {node} declares no route"),
                Some(host) => format!("node {node} has no route `{host}`"),
            })
        }
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

/// Removes the route named `host` from `document`; whether there was one.
fn take(document: &mut DocumentMut, host: &str) -> bool {
    let Some(item) = document.get_mut("proxy") else {
        return false;
    };
    if item.as_table_like().is_some_and(|table| named(table, host)) {
        document.remove("proxy");
        return true;
    }
    let Some(entries) = item.as_array_of_tables_mut() else {
        return false;
    };
    let before = entries.len();
    entries.retain(|table| !named(table, host));
    if entries.len() == before {
        return false;
    }
    match entries.len() {
        0 => {
            document.remove("proxy");
        }
        1 => {
            if let Some(last) = entries.get(0).cloned() {
                *item = Item::Table(last);
            }
        }
        _ => {}
    }
    true
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
            "---\nariane: 2\nnode: q\nid: T1\ntitle: A\nstatus: todo\n---\n",
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
        set(
            &tree,
            &Config::default(),
            &node,
            "pp.local",
            3001,
            Some("p.local"),
        )
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
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            read(&repo, "q").unwrap(),
            "[proxy]\nhost = \"q.local\"\nport = 5173\n"
        );
        let tree = Tree::load(repo.root()).unwrap();
        remove(&tree, &NodePath::new("q"), None).unwrap().unwrap();
        assert_eq!(read(&repo, "q"), None);
    }

    #[test]
    fn removing_a_route_keeps_the_rest_of_the_file() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        remove(&tree, &NodePath::new("p"), None).unwrap().unwrap();
        let left = read(&repo, "p").unwrap();
        assert!(left.starts_with("# the p project\ndescription = \"P.\"\n"));
        assert!(left.contains("[tracker]"));
        assert!(!left.contains("proxy"));
        let tree = Tree::load(repo.root()).unwrap();
        assert!(
            remove(&tree, &NodePath::new("p"), None)
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
            set(&tree, &config, node, host, port, None)
                .unwrap()
                .unwrap_err()
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
        set(&tree, &config, &NodePath::new("p"), "p.local", 4000, None)
            .unwrap()
            .unwrap();
    }

    #[test]
    fn a_second_route_turns_the_table_into_entries_and_a_removal_turns_it_back() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let p = NodePath::new("p");
        set(&tree, &Config::default(), &p, "p-b.local", 3001, None)
            .unwrap()
            .unwrap();
        assert_eq!(
            read(&repo, "p").unwrap(),
            "# the p project\ndescription = \"P.\"\n\n[[proxy]]\nhost = \"p.local\" # its dev server\nport = 3000\n\n[[proxy]]\nhost = \"p-b.local\"\nport = 3001\n\n[tracker]\nkind = \"monday\"\n"
        );
        let tree = Tree::load(repo.root()).unwrap();
        set(
            &tree,
            &Config::default(),
            &p,
            "p-c.local",
            3002,
            Some("p-b.local"),
        )
        .unwrap()
        .unwrap();
        let tree = Tree::load(repo.root()).unwrap();
        let table = Table::of(&tree, &config::Server::default());
        assert_eq!(table.find("p-c.local").unwrap().port, 3002);
        assert!(table.find("p-b.local").is_none());
        assert_eq!(table.find("p.local").unwrap().port, 3000);
        remove(&tree, &p, Some("p.local")).unwrap().unwrap();
        assert_eq!(
            read(&repo, "p").unwrap(),
            "# the p project\ndescription = \"P.\"\n\n[proxy]\nhost = \"p-c.local\"\nport = 3002\n\n[tracker]\nkind = \"monday\"\n"
        );
    }

    #[test]
    fn a_route_the_node_does_not_have_is_neither_replaced_nor_removed() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let p = NodePath::new("p");
        let config = Config::default();
        set(&tree, &config, &p, "p-b.local", 3001, None)
            .unwrap()
            .unwrap();
        let tree = Tree::load(repo.root()).unwrap();
        assert!(
            set(&tree, &config, &p, "p-c.local", 3002, Some("nowhere.local"))
                .unwrap()
                .unwrap_err()
                .contains("has no route `nowhere.local`")
        );
        assert!(
            set(&tree, &config, &p, "p.local", 3002, Some("p-b.local"))
                .unwrap()
                .unwrap_err()
                .contains("already the name of node p")
        );
        assert!(
            remove(&tree, &p, Some("nowhere.local"))
                .unwrap()
                .unwrap_err()
                .contains("has no route")
        );
    }
}

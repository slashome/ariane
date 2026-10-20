//! `ariane tasks`: lists the items of the tree.
//!
//! Prints one line per item (node, id, status, title) or, with `--json`, the full
//! rows for scripts and agents. ADR-0010 makes this the on-demand aggregation
//! that `INDEX.md` files no longer carry: an agent without the interface asks
//! `ariane tasks --node <node> --json`.

use std::process::ExitCode;

use anyhow::Result;
use serde::Serialize;

use crate::cli::TasksArgs;
use crate::item::Kind;
use crate::node::Category;
use crate::tree::{Item, NodePath, Tree};

/// One item as listed by `ariane tasks`, and as serialized with `--json`.
///
/// The `'a` lifetime says that the string fields borrow from the loaded tree
/// instead of copying it: a `Row` cannot outlive the [`Tree`] it was built from.
#[derive(Debug, Serialize)]
pub struct Row<'a> {
    /// Logical path of the node holding the item.
    pub node: &'a NodePath,
    pub id: Option<&'a str>,
    pub title: Option<&'a str>,
    pub kind: Kind,
    pub archived: bool,
    /// Status id as written in the item, if any.
    pub status: Option<&'a str>,
    /// Category of that status in the node's effective workflow, if it is known there.
    pub category: Option<Category>,
    pub size: Option<&'a str>,
    pub impact: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub created: Option<&'a str>,
    /// Schema version of the file; `None` for a file not migrated yet.
    pub version: Option<u64>,
    /// Path of the file relative to the repository root.
    pub path: String,
}

/// Runs `ariane tasks`: filters the items by node and archive state, then prints
/// them as a table or as JSON.
///
/// # Errors
///
/// Fails only if the rows cannot be serialized to JSON.
pub fn run(tree: &Tree, args: &TasksArgs, scope: Option<&NodePath>) -> Result<ExitCode> {
    let rows: Vec<Row> = tree
        .items_within(scope)
        .filter(|item| args.archived || !item.archived)
        .map(|item| row(tree, item))
        .collect();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        print_table(&rows);
    }
    Ok(ExitCode::SUCCESS)
}

/// Builds the row of one item, resolving its status category through the
/// node's effective workflow (inherited `node.toml` files, SPEC.md §1.5).
pub fn row<'a>(tree: &Tree, item: &'a Item) -> Row<'a> {
    let document = &item.document;
    let status = document.text("status");
    let category = status.and_then(|status| {
        tree.configs
            .effective(&item.node)
            .status(status)
            .map(|status| status.category)
    });
    Row {
        node: &item.node,
        id: document.text("id"),
        title: document.text("title"),
        kind: item.kind,
        archived: item.archived,
        status,
        category,
        size: document.text("size"),
        impact: document.text("impact"),
        priority: document.text("priority"),
        created: document.text("created"),
        version: document.version(),
        path: item.relative.to_string_lossy().into_owned(),
    }
}

/// Prints the rows as aligned columns, followed by a count of unversioned items.
fn print_table(rows: &[Row]) {
    let node_width = rows
        .iter()
        .map(|row| row.node.to_string().len())
        .max()
        .unwrap_or(0);
    let id_width = rows
        .iter()
        .map(|row| row.id.unwrap_or("-").len())
        .max()
        .unwrap_or(0);
    let status_width = rows
        .iter()
        .map(|row| row.status.unwrap_or("-").len())
        .max()
        .unwrap_or(0);
    for row in rows {
        println!(
            "{:node_width$}  {:id_width$}  {:status_width$}  {}",
            row.node.to_string(),
            row.id.unwrap_or("-"),
            row.status.unwrap_or("-"),
            row.title.unwrap_or(&row.path),
        );
    }
    let unversioned = rows.iter().filter(|row| row.version.is_none()).count();
    println!(
        "\n{} items, {unversioned} without a versioned header",
        rows.len()
    );
}

/// Unit tests of row building.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    #[test]
    fn a_row_resolves_the_status_category_through_the_node_workflow() {
        let repo = Repo::new();
        repo.write(
            "slashome/pharmakon/node.toml",
            "[[workflow]]\nid = \"valide\"\nlabel = \"Validé\"\ncategory = \"done\"\n",
        )
        .write(
            "slashome/pharmakon/tasks/T038-x.md",
            "---\nariane: 1\nnode: slashome/pharmakon\nid: T038\ntitle: X\nstatus: valide\n---\n# X\n",
        )
        .write("slashome/me/tasks/legacy.md", "# Legacy\n\n- **État** : ouvert\n");
        let tree = Tree::load(repo.root()).unwrap();
        let rows: Vec<_> = tree.items.iter().map(|item| row(&tree, item)).collect();
        let versioned = rows.iter().find(|row| row.id == Some("T038")).unwrap();
        assert_eq!(versioned.category, Some(Category::Done));
        let legacy = rows.iter().find(|row| row.id.is_none()).unwrap();
        assert_eq!(legacy.version, None);
        assert_eq!(legacy.category, None);
    }
}

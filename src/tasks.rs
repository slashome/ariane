use std::process::ExitCode;

use anyhow::Result;
use serde::Serialize;

use crate::cli::TasksArgs;
use crate::item::Kind;
use crate::node::Category;
use crate::tree::{Item, NodePath, Tree};

#[derive(Debug, Serialize)]
pub struct Row<'a> {
    pub node: &'a NodePath,
    pub id: Option<&'a str>,
    pub title: Option<&'a str>,
    pub kind: Kind,
    pub archived: bool,
    pub status: Option<&'a str>,
    pub category: Option<Category>,
    pub size: Option<&'a str>,
    pub impact: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub created: Option<&'a str>,
    pub version: Option<u64>,
    pub path: String,
}

pub fn run(tree: &Tree, args: &TasksArgs) -> Result<ExitCode> {
    let scope = args.node.as_deref().map(NodePath::new);
    let rows: Vec<Row> = tree
        .items_within(scope.as_ref())
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

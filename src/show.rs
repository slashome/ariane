//! `ariane show`: one item, whole — its header, its body and the items it is
//! linked to.
//!
//! `ariane tasks` lists rows; `show` is the page of one of them. With `--json`
//! it prints the item's `ariane tasks` row, its full header (custom keys
//! included, in file order), its Markdown body and its [`Relations`]: parent,
//! children, dependencies and dependents, each as a row, so an agent or the
//! interface can follow them without a second command. An item with children,
//! such as an epic and its stories, also carries their [`Progress`].

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::cli::ShowArgs;
use crate::node::Category;
use crate::relations::{self, Relations};
use crate::tasks::{self, Row};
use crate::tree::{Item, NodePath, Tree, parse_reference};

/// One item as printed by `ariane show --json`.
#[derive(Debug, Serialize)]
pub struct Shown<'a> {
    /// The item's `ariane tasks` row, its fields at the top level.
    #[serde(flatten)]
    pub row: Row<'a>,
    /// Every key of the front-matter, in file order; `null` without one.
    pub header: serde_json::Value,
    pub body: &'a str,
    pub relations: Relations<'a>,
    /// How far its children have gone; `null` without children.
    pub progress: Option<Progress>,
}

/// How far the children of an item have gone, by status category.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// Every child.
    pub total: usize,
    /// The children whose status is in the `done` category.
    pub done: usize,
    /// The children whose status is in the `dropped` category, out of the count.
    pub dropped: usize,
}

impl Progress {
    /// Counts the categories of `children`; `None` when there are none.
    fn of(children: &[Row]) -> Option<Progress> {
        let count = |category| {
            children
                .iter()
                .filter(|child| child.category == Some(category))
                .count()
        };
        (!children.is_empty()).then(|| Progress {
            total: children.len(),
            done: count(Category::Done),
            dropped: count(Category::Dropped),
        })
    }
}

/// Builds what `show` prints for `item`, dated by `dates` when given.
pub fn shown<'a>(
    tree: &'a Tree,
    item: &'a Item,
    dates: Option<&HashMap<PathBuf, String>>,
) -> Shown<'a> {
    let mut row = tasks::row(tree, item);
    row.updated = dates.and_then(|dates| dates.get(&item.relative).cloned());
    let relations = relations::of(tree, item);
    Shown {
        progress: Progress::of(&relations.children),
        row,
        header: item
            .document
            .mapping()
            .and_then(|mapping| serde_json::to_value(mapping).ok())
            .unwrap_or(serde_json::Value::Null),
        body: &item.document.body,
        relations,
    }
}

/// Finds the item `raw` names: `<node>#<id>` anywhere, a bare id within
/// `here`, or, outside any node, the one item of the tree with that id.
///
/// # Errors
///
/// Fails, naming the candidates, when the reference is malformed, points to
/// nothing, or is a bare id shared by several nodes.
pub fn find<'a>(tree: &'a Tree, raw: &str, here: Option<&NodePath>) -> Result<&'a Item> {
    let home = NodePath::default();
    let Some((node, id)) = parse_reference(raw, here.unwrap_or(&home)) else {
        bail!("`{raw}` is not a reference: write `<node>#<id>` or a bare id");
    };
    let bare = !raw.contains('#') && here.is_none();
    let found: Vec<&Item> = tree
        .items
        .iter()
        .filter(|item| item.id() == Some(id.as_str()) && (bare || item.node == node))
        .collect();
    match found.as_slice() {
        [item] => Ok(item),
        [] => bail!("no item `{raw}`"),
        several => {
            let references: Vec<String> =
                several.iter().filter_map(|item| item.reference()).collect();
            bail!("`{raw}` is ambiguous: {}", references.join(", "))
        }
    }
}

/// Runs `ariane show`: finds the item and prints it, as text or as JSON.
///
/// # Errors
///
/// Fails when the reference names no single item, or the JSON cannot be
/// serialized.
pub fn run(tree: &Tree, args: &ShowArgs, here: Option<&NodePath>) -> Result<ExitCode> {
    let item = find(tree, &args.reference, here)?;
    let dates = crate::history::file_dates(&tree.root);
    let shown = shown(tree, item, Some(&dates));
    if args.json {
        println!("{}", serde_json::to_string_pretty(&shown)?);
    } else {
        print_text(&shown);
    }
    Ok(ExitCode::SUCCESS)
}

/// Prints the item as a short header, its relations, then its body.
fn print_text(shown: &Shown) {
    let row = &shown.row;
    let reference = row.reference.as_deref().unwrap_or(&row.path);
    println!("{reference}  {}", row.title.unwrap_or("(untitled)"));
    let facts: Vec<String> = [
        Some(kind_word(row)),
        row.status.map(|status| format!("status {status}")),
        row.priority.map(|priority| format!("priority {priority}")),
        row.size.map(|size| format!("size {size}")),
        row.impact.map(|impact| format!("impact {impact}")),
        row.archived.then(|| "archived".to_owned()),
    ]
    .into_iter()
    .flatten()
    .collect();
    println!("{}", facts.join(" · "));
    println!("{}", row.path);
    let relations = &shown.relations;
    let list = |rows: &[Row]| {
        rows.iter()
            .map(|row| row.reference.clone().unwrap_or_else(|| row.path.clone()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !row.personas.is_empty() {
        println!("personas:   {}", row.personas.join(", "));
    }
    if let Some(parent) = &relations.parent {
        println!("parent:     {}", list(std::slice::from_ref(parent)));
    }
    if !relations.children.is_empty() {
        let children: Vec<String> = relations
            .children
            .iter()
            .map(|child| {
                let reference = child
                    .reference
                    .clone()
                    .unwrap_or_else(|| child.path.clone());
                match child.status {
                    Some(status) => format!("{reference} ({status})"),
                    None => reference,
                }
            })
            .collect();
        println!("children:   {}", children.join(", "));
    }
    if let Some(progress) = &shown.progress {
        let dropped = match progress.dropped {
            0 => String::new(),
            dropped => format!(", {dropped} dropped"),
        };
        println!(
            "progress:   {}/{} done{dropped}",
            progress.done,
            progress.total - progress.dropped
        );
    }
    for (label, rows) in [
        ("depends on:", &relations.depends_on),
        ("dependents:", &relations.dependents),
    ] {
        if !rows.is_empty() {
            println!("{label} {}", list(rows));
        }
    }
    if !relations.unresolved.is_empty() {
        println!("unresolved: {}", relations.unresolved.join(", "));
    }
    println!("\n{}", shown.body.trim_end());
}

/// The item's kind as its JSON word.
fn kind_word(row: &Row) -> String {
    serde_json::to_value(row.kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Unit tests of reference lookup and of the JSON shape.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    fn sample() -> Tree {
        let repo = Repo::new();
        repo.write(
            "p/stories/S1-s.md",
            "---\nariane: 1\nnode: p\nid: S1\ntitle: S\nstatus: todo\npersonas: [\"cmc:dev\"]\nmod: Kept\n---\n# S\n\nBody.\n",
        )
        .write(
            "p/tasks/T1-a.md",
            "---\nariane: 1\nnode: p\nid: T1\ntitle: A\nstatus: todo\nparent: S1\n---\n",
        )
        .write(
            "q/tasks/T1-b.md",
            "---\nariane: 1\nnode: q\nid: T1\ntitle: B\nstatus: todo\n---\n",
        );
        Tree::load(repo.root()).unwrap()
    }

    #[test]
    fn a_reference_is_found_across_nodes_or_within_the_current_one() {
        let tree = sample();
        assert_eq!(find(&tree, "q#T1", None).unwrap().node.as_str(), "q");
        let here = NodePath::new("p");
        assert_eq!(find(&tree, "T1", Some(&here)).unwrap().node.as_str(), "p");
        assert_eq!(find(&tree, "S1", None).unwrap().node.as_str(), "p");
        let ambiguous = find(&tree, "T1", None).unwrap_err().to_string();
        assert!(
            ambiguous.contains("p#T1") && ambiguous.contains("q#T1"),
            "{ambiguous}"
        );
        assert!(find(&tree, "p#T9", None).is_err());
        assert!(find(&tree, "p#9", None).is_err());
    }

    #[test]
    fn an_epic_lists_its_stories_and_how_far_they_have_gone() {
        let repo = Repo::new();
        repo.write(
            "p/epics/E1-e.md",
            "---\nariane: 1\nnode: p\nid: E1\ntitle: E\nstatus: todo\n---\n",
        );
        for (id, status) in [("S1", "done"), ("S2", "todo"), ("S3", "dropped")] {
            repo.write(
                &format!("p/stories/{id}-s.md"),
                &format!("---\nariane: 1\nnode: p\nid: {id}\ntitle: S\nstatus: {status}\nparent: E1\n---\n"),
            );
        }
        let tree = Tree::load(repo.root()).unwrap();
        let epic = shown(&tree, find(&tree, "p#E1", None).unwrap(), None);
        assert_eq!(epic.row.kind, crate::item::Kind::Epic);
        assert_eq!(epic.relations.children.len(), 3);
        assert_eq!(
            epic.progress,
            Some(Progress {
                total: 3,
                done: 1,
                dropped: 1
            })
        );
        let story = shown(&tree, find(&tree, "p#S2", None).unwrap(), None);
        assert_eq!(story.progress, None);
    }

    #[test]
    fn the_json_carries_the_row_the_whole_header_the_body_and_the_relations() {
        let tree = sample();
        let story = find(&tree, "p#S1", None).unwrap();
        let json = serde_json::to_value(shown(&tree, story, None)).unwrap();
        assert_eq!(json["reference"], "p#S1");
        assert_eq!(json["kind"], "story");
        assert_eq!(json["header"]["mod"], "Kept");
        assert_eq!(json["personas"], serde_json::json!(["cmc:dev"]));
        assert_eq!(json["body"], "# S\n\nBody.\n");
        assert_eq!(json["relations"]["children"][0]["reference"], "p#T1");
        assert_eq!(json["relations"]["dependents"], serde_json::json!([]));
    }
}

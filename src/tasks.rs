//! `ariane tasks`: lists the items of the tree.
//!
//! Prints one line per item (node, id, status, title) or, with `--json`, the full
//! rows for scripts and agents. A node has no index file (ADR-0017): an agent
//! without the interface reads a node's state with
//! `ariane tasks --node <node> --json`. `--kind` keeps some kinds only,
//! `--body` adds each item's Markdown body and `--dates` the date of its
//! latest commit, with, for a finished item without `closed`, the closing date
//! approximated from git (ADR-0027).

use std::process::ExitCode;

use anyhow::Result;
use serde::Serialize;

use crate::cli::TasksArgs;
use crate::closed;
use crate::estimate::{self, Estimate};
use crate::history::History;
use crate::item::Kind;
use crate::node::Category;
use crate::rank;
use crate::relations;
use crate::release::{self, EffectiveRelease};
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
    /// `<node>#<id>`, the reference that reads the same from any node.
    pub reference: Option<String>,
    pub title: Option<&'a str>,
    pub kind: Kind,
    pub archived: bool,
    /// Status id as written in the item, if any.
    pub status: Option<&'a str>,
    /// Category of that status in the node's effective workflow, if it is known there.
    pub category: Option<Category>,
    pub size: Option<&'a str>,
    /// The hours of focused work the item weighs, with its testing and where
    /// they come from (ADR-0025); `None` for a reflexion or a ressource.
    pub estimate: Option<Estimate>,
    pub impact: Option<&'a str>,
    pub priority: Option<&'a str>,
    /// The manual order within the node and kind (ADR-0022); `None` when unranked.
    pub rank: Option<i64>,
    pub created: Option<&'a str>,
    /// When the item entered a status of the `done` category, as written in
    /// its header (ADR-0027).
    pub closed: Option<&'a str>,
    /// The node's classification of the item (`type` in the header).
    pub r#type: Option<&'a str>,
    /// Ids of the personas a story is written for, from the node's `[personas]`.
    pub personas: Vec<&'a str>,
    /// `release` as written in the item's own header.
    pub release: Option<&'a str>,
    /// The release the item is planned in: its own, else its story's, else its
    /// epic's, with where it comes from and its place in the node's declared
    /// order (ADR-0023); `None` when none of them carries one.
    pub effective_release: Option<EffectiveRelease<'a>>,
    /// `parent` as `<node>#<id>`.
    pub parent: Option<String>,
    /// The epic of a story, or of a task's story, as `<node>#<id>` (ADR-0019).
    pub epic: Option<String>,
    /// `depends_on` as `<node>#<id>` references, in header order.
    pub depends_on: Vec<String>,
    /// Links to external trackers, as `<connector>:<id>` (ADR-0011); empty when unbound.
    pub external: Vec<&'a str>,
    /// Branches of the work, as `<owner>/<repo>:<branch>`.
    pub branches: Vec<&'a str>,
    /// Pull requests of the work, as URLs.
    pub pull_requests: Vec<&'a str>,
    /// Handles of the people the item waits for, from the node's `[people]`.
    pub waiting_for: Vec<&'a str>,
    /// Why it waits: `review`, `answer` or `other`.
    pub waiting_on: Option<&'a str>,
    /// Since when it waits.
    pub waiting_since: Option<&'a str>,
    /// Schema version of the file; `None` for a file not migrated yet.
    pub version: Option<u64>,
    /// Path of the file relative to the repository root.
    pub path: String,
    /// Date of the latest commit touching the file, with `--dates`; `None`
    /// for a file never committed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    /// With `--dates`, for an item in the `done` category without `closed`:
    /// the date git first saw it enter that category for the last time, an
    /// approximation never written to the file (ADR-0027); `None` otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_approx: Option<String>,
    /// The Markdown body, with `--body`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<&'a str>,
}

/// Which items `ariane tasks` lists, and what each row carries beyond the header.
#[derive(Debug, Default)]
pub struct Selection<'a> {
    /// Include the items under `archives/`.
    pub archived: bool,
    /// Keep only these kinds; every kind when empty.
    pub kinds: &'a [Kind],
    /// Add the Markdown body to each row.
    pub body: bool,
    /// What git remembers of each file; `None` leaves `updated` and
    /// `closed_approx` out.
    pub dates: Option<&'a History>,
}

/// The rows of the items of `scope` that `selection` keeps: nodes and kinds in
/// tree order, the items of each in their manual order ([`rank::sort`]).
/// The command and `GET /api/items` both list items through it.
pub fn rows<'a>(
    tree: &'a Tree,
    scope: Option<&'a NodePath>,
    selection: &Selection,
) -> Vec<Row<'a>> {
    let mut rows: Vec<Row> = tree
        .items_within(scope)
        .filter(|item| selection.archived || !item.archived)
        .filter(|item| selection.kinds.is_empty() || selection.kinds.contains(&item.kind))
        .map(|item| {
            let mut row = row(tree, item);
            if selection.body {
                row.body = Some(&item.document.body);
            }
            if let Some(history) = selection.dates {
                date(tree, item, &mut row, history);
            }
            row
        })
        .collect();
    rank::sort(&mut rows);
    rows
}

/// Runs `ariane tasks`: filters the items by node and archive state, then prints
/// them as a table or as JSON.
///
/// # Errors
///
/// Fails only if the rows cannot be serialized to JSON.
pub fn run(tree: &Tree, args: &TasksArgs, scope: Option<&NodePath>) -> Result<ExitCode> {
    let dates = args.dates.then(|| History::read(&tree.root));
    let selection = Selection {
        archived: args.archived,
        kinds: &args.kind,
        body: args.body,
        dates: dates.as_ref(),
    };
    let rows = rows(tree, scope, &selection);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        print_table(&rows);
    }
    Ok(ExitCode::SUCCESS)
}

/// Builds the row of one item, resolving its status category through the
/// node's effective workflow and its release through its story, its epic and
/// the node's declared releases (inherited `node.toml` files, SPEC.md §1.5).
pub fn row<'a>(tree: &'a Tree, item: &'a Item) -> Row<'a> {
    let document = &item.document;
    let config = tree.configs.effective(&item.node);
    let status = document.text("status");
    let category = status.and_then(|status| config.status(status).map(|status| status.category));
    Row {
        node: &item.node,
        id: document.text("id"),
        reference: item.reference(),
        title: document.text("title"),
        kind: item.kind,
        archived: item.archived,
        status,
        category,
        size: document.text("size"),
        estimate: estimate::effective(tree, item),
        impact: document.text("impact"),
        priority: document.text("priority"),
        rank: document.integer("rank"),
        created: document.text("created"),
        closed: document.text(closed::KEY),
        r#type: document.text("type"),
        personas: document.texts("personas"),
        release: document.text("release"),
        effective_release: release::effective(tree, item, &config),
        parent: relations::parent(item),
        epic: relations::epic(tree, item),
        depends_on: relations::depends_on(item),
        external: document.texts("external"),
        branches: document.texts("branches"),
        pull_requests: document.texts("pull_requests"),
        waiting_for: document.texts("waiting_for"),
        waiting_on: document.text("waiting_on"),
        waiting_since: document.text("waiting_since"),
        version: document.version(),
        path: item.relative.to_string_lossy().into_owned(),
        updated: None,
        closed_approx: None,
        body: None,
    }
}

/// Dates `row`, the row of `item`, from `history`: its latest commit and, when
/// it is finished without `closed`, its approximate closing date.
pub fn date(tree: &Tree, item: &Item, row: &mut Row, history: &History) {
    row.updated = history.updated.get(&item.relative).cloned();
    if row.closed.is_some()
        || row.category != Some(Category::Done)
        || !closed::applies_to(item.kind)
    {
        return;
    }
    let config = tree.configs.effective(&item.node);
    let is_done = |status: &str| {
        config
            .status(status)
            .is_some_and(|status| status.category == Category::Done)
    };
    row.closed_approx =
        closed::approximate(history.transitions(&item.relative), is_done).map(str::to_owned);
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
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn a_row_resolves_the_status_category_through_the_node_workflow() {
        let repo = Repo::new();
        repo.write(
            "slashome/pharmakon/node.toml",
            "[[workflow]]\nid = \"valide\"\nlabel = \"Validé\"\ncategory = \"done\"\n",
        )
        .write(
            "slashome/pharmakon/tasks/T038-x.md",
            "---\nariane: 2\nnode: slashome/pharmakon\nid: T038\ntitle: X\nstatus: valide\n---\n# X\n",
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

    #[test]
    fn a_selection_keeps_some_kinds_and_adds_bodies_and_dates() {
        let repo = Repo::new();
        repo.write(
            "p/tasks/T1-a.md",
            "---\nariane: 2\nnode: p\nid: T1\ntitle: A\nstatus: todo\nparent: S1\nrelease: \"1.0\"\n---\n# A\n",
        )
        .write(
            "p/stories/S1-s.md",
            "---\nariane: 2\nnode: p\nid: S1\ntitle: S\nstatus: todo\n---\n# S\n",
        )
        .write(
            "p/archives/stories/S2-old.md",
            "---\nariane: 2\nnode: p\nid: S2\ntitle: Old\nstatus: done\n---\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        let dates = History {
            updated: HashMap::from([(
                PathBuf::from("tree/p/stories/S1-s.md"),
                "2026-10-22".to_owned(),
            )]),
            ..History::default()
        };
        let stories = rows(
            &tree,
            None,
            &Selection {
                kinds: &[Kind::Story],
                body: true,
                dates: Some(&dates),
                ..Selection::default()
            },
        );
        assert_eq!(stories.len(), 1);
        assert_eq!(stories[0].reference.as_deref(), Some("p#S1"));
        assert_eq!(stories[0].body, Some("# S\n"));
        assert_eq!(stories[0].updated.as_deref(), Some("2026-10-22"));
        let all = rows(
            &tree,
            None,
            &Selection {
                archived: true,
                ..Selection::default()
            },
        );
        assert_eq!(all.len(), 3);
        let task = all.iter().find(|row| row.id == Some("T1")).unwrap();
        assert_eq!(task.parent.as_deref(), Some("p#S1"));
        assert_eq!(task.release, Some("1.0"));
        assert!(task.body.is_none());
        let json = serde_json::to_value(task).unwrap();
        assert_eq!(
            json["effective_release"],
            serde_json::json!({
                "name": "1.0",
                "source": "own",
                "from": "p#T1",
                "position": null,
                "parking": false,
            })
        );
        assert_eq!(json["estimate"]["basis"], "default");
        assert_eq!(json["estimate"]["hours"], 12.0);
        assert_eq!(json["estimate"]["test_hours"], 3.0);
        assert_eq!(json["estimate"]["tester"], "owner");
        assert!(json.get("body").is_none());
        assert!(json.get("closed_approx").is_none());
        assert_eq!(json["type"], serde_json::Value::Null);
        assert_eq!(json["closed"], serde_json::Value::Null);
    }

    #[test]
    fn a_finished_item_shows_its_closed_date_or_else_the_one_git_approaches() {
        let repo = Repo::new();
        repo.write(
            "p/tasks/T1-a.md",
            "---\nariane: 2\nnode: p\nid: T1\ntitle: A\nstatus: done\nclosed: 2026-11-12\n---\n",
        )
        .write(
            "p/archives/tasks/T2-b.md",
            "---\nariane: 2\nnode: p\nid: T2\ntitle: B\nstatus: done\n---\n",
        )
        .write(
            "p/tasks/T3-c.md",
            "---\nariane: 2\nnode: p\nid: T3\ntitle: C\nstatus: todo\n---\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        let steps = |statuses: &[(&str, &str)]| {
            statuses
                .iter()
                .map(|(date, status)| crate::history::Transition {
                    date: (*date).to_owned(),
                    status: Some((*status).to_owned()),
                })
                .collect::<Vec<_>>()
        };
        let finished = steps(&[("2026-10-01", "todo"), ("2026-10-09", "done")]);
        let history = History {
            statuses: HashMap::from([
                (PathBuf::from("tree/p/tasks/T1-a.md"), finished.clone()),
                (
                    PathBuf::from("tree/p/archives/tasks/T2-b.md"),
                    finished.clone(),
                ),
                (PathBuf::from("tree/p/tasks/T3-c.md"), finished),
            ]),
            ..History::default()
        };
        let all = rows(
            &tree,
            None,
            &Selection {
                archived: true,
                dates: Some(&history),
                ..Selection::default()
            },
        );
        let by_id = |id: &str| all.iter().find(|row| row.id == Some(id)).unwrap();
        assert_eq!(by_id("T1").closed, Some("2026-11-12"));
        assert_eq!(by_id("T1").closed_approx, None);
        assert_eq!(by_id("T2").closed, None);
        assert_eq!(by_id("T2").closed_approx.as_deref(), Some("2026-10-09"));
        assert_eq!(by_id("T3").closed_approx, None);
        let json = serde_json::to_value(by_id("T2")).unwrap();
        assert_eq!(json["closed_approx"], "2026-10-09");
    }
}

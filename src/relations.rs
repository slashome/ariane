//! The links between items: `parent` and `depends_on` as their headers write
//! them (SPEC.md §1.3, §1.4), and their inverses, the children of an item and
//! the items that depend on it.
//!
//! A reference is written `T022` inside its node and `<node>#T022` across
//! nodes. Here every reference is resolved against the whole tree and given
//! back in the form that reads the same from anywhere, `<node>#<id>`, so that
//! the interface and agents never have to know which node a bare id meant.

use std::collections::HashMap;

use serde::Serialize;

use crate::item::Kind;
use crate::tasks::{self, Row};
use crate::tree::{Item, NodePath, Tree, full_reference, parse_reference};

/// Every item of the tree with a valid id, by `(node, id)`.
pub struct Index<'a> {
    items: HashMap<(NodePath, &'a str), &'a Item>,
}

impl<'a> Index<'a> {
    /// Indexes the items of `tree`. Two items sharing an id in a node (a
    /// `doctor` error) resolve to the first one.
    pub fn new(tree: &'a Tree) -> Index<'a> {
        let mut items = HashMap::new();
        for item in &tree.items {
            if let Some(id) = item.id() {
                items.entry((item.node.clone(), id)).or_insert(item);
            }
        }
        Index { items }
    }

    /// The item a reference written in an item of `from` points to.
    pub fn resolve(&self, raw: &str, from: &NodePath) -> Option<&'a Item> {
        let (node, id) = parse_reference(raw, from)?;
        self.items.get(&(node, id.as_str())).copied()
    }
}

/// The `parent` of `item` as `<node>#<id>`; a value that is not a reference
/// is given back as written, for the reader to see it.
pub fn parent(item: &Item) -> Option<String> {
    let raw = item.document.text("parent")?;
    Some(canonical(raw, &item.node))
}

/// The `depends_on` entries of `item` as `<node>#<id>`, in header order.
pub fn depends_on(item: &Item) -> Vec<String> {
    item.document
        .texts("depends_on")
        .into_iter()
        .map(|raw| canonical(raw, &item.node))
        .collect()
}

/// The epic `item` belongs to, as `<node>#<id>`: its `parent` for a story,
/// its story's `parent` for a task (ADR-0019); `None` for an epic itself, or
/// when no ancestor within two levels is an epic.
pub fn epic(tree: &Tree, item: &Item) -> Option<String> {
    let resolve = |child: &Item| {
        let (node, id) = parse_reference(child.document.text("parent")?, &child.node)?;
        tree.items
            .iter()
            .find(|candidate| candidate.node == node && candidate.id() == Some(id.as_str()))
    };
    let mut current = item;
    for _ in 0..2 {
        let parent = resolve(current)?;
        if parent.kind == Kind::Epic {
            return parent.reference();
        }
        current = parent;
    }
    None
}

/// `raw` as `<node>#<id>` when it parses, unchanged otherwise.
fn canonical(raw: &str, from: &NodePath) -> String {
    match parse_reference(raw, from) {
        Some((node, id)) => full_reference(&node, &id),
        None => raw.to_owned(),
    }
}

/// The items one item is linked to, each as its `ariane tasks` row.
#[derive(Debug, Serialize)]
pub struct Relations<'a> {
    /// The item named by its `parent`.
    pub parent: Option<Row<'a>>,
    /// The items whose `parent` is this one.
    pub children: Vec<Row<'a>>,
    /// The items named by its `depends_on`, in header order.
    pub depends_on: Vec<Row<'a>>,
    /// The items whose `depends_on` names this one.
    pub dependents: Vec<Row<'a>>,
    /// The references of its `parent` and `depends_on` that point to no item.
    pub unresolved: Vec<String>,
}

/// Resolves the relations of `item` against the whole tree, archived items
/// included.
pub fn of<'a>(tree: &'a Tree, item: &'a Item) -> Relations<'a> {
    let index = Index::new(tree);
    let mut unresolved = Vec::new();
    let mut resolve = |raw: &str| {
        let found = index.resolve(raw, &item.node);
        if found.is_none() {
            unresolved.push(canonical(raw, &item.node));
        }
        found
    };
    let parent = item
        .document
        .text("parent")
        .and_then(&mut resolve)
        .map(|parent| tasks::row(tree, parent));
    let depends_on = item
        .document
        .texts("depends_on")
        .into_iter()
        .filter_map(&mut resolve)
        .map(|target| tasks::row(tree, target))
        .collect();
    let points_here = |raw: &str, from: &NodePath| {
        index
            .resolve(raw, from)
            .is_some_and(|target| std::ptr::eq(target, item))
    };
    let children = tree
        .items
        .iter()
        .filter(|other| {
            other
                .document
                .text("parent")
                .is_some_and(|raw| points_here(raw, &other.node))
        })
        .map(|child| tasks::row(tree, child))
        .collect();
    let dependents = tree
        .items
        .iter()
        .filter(|other| {
            other
                .document
                .texts("depends_on")
                .into_iter()
                .any(|raw| points_here(raw, &other.node))
        })
        .map(|dependent| tasks::row(tree, dependent))
        .collect();
    Relations {
        parent,
        children,
        depends_on,
        dependents,
        unresolved,
    }
}

/// Unit tests of reference resolution and of the inverse relations.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// A story of node `p` with two tasks, one of which depends on the other
    /// and on a task of node `q`.
    fn sample() -> Tree {
        let repo = Repo::new();
        repo.write(
            "p/stories/S1-s.md",
            "---\nariane: 1\nnode: p\nid: S1\ntitle: S\nstatus: todo\n---\n",
        )
        .write(
            "p/tasks/T1-a.md",
            "---\nariane: 1\nnode: p\nid: T1\ntitle: A\nstatus: done\nparent: S1\n---\n",
        )
        .write(
            "p/tasks/T2-b.md",
            "---\nariane: 1\nnode: p\nid: T2\ntitle: B\nstatus: todo\nparent: p#S1\ndepends_on: [T1, q#T9, T404]\n---\n",
        )
        .write(
            "q/tasks/T9-z.md",
            "---\nariane: 1\nnode: q\nid: T9\ntitle: Z\nstatus: todo\n---\n",
        );
        Tree::load(repo.root()).unwrap()
    }

    fn find<'a>(tree: &'a Tree, reference: &str) -> &'a Item {
        tree.items
            .iter()
            .find(|item| item.reference().as_deref() == Some(reference))
            .unwrap()
    }

    #[test]
    fn references_are_given_back_as_read_from_anywhere() {
        let tree = sample();
        let task = find(&tree, "p#T2");
        assert_eq!(parent(task).as_deref(), Some("p#S1"));
        assert_eq!(depends_on(task), ["p#T1", "q#T9", "p#T404"]);
    }

    #[test]
    fn a_story_and_its_tasks_belong_to_the_epic_of_the_story() {
        let repo = Repo::new();
        repo.write(
            "p/epics/E1-e.md",
            "---\nariane: 1\nnode: p\nid: E1\ntitle: E\nstatus: todo\n---\n",
        )
        .write(
            "p/stories/S1-s.md",
            "---\nariane: 1\nnode: p\nid: S1\ntitle: S\nstatus: todo\nparent: E1\n---\n",
        )
        .write(
            "p/tasks/T1-a.md",
            "---\nariane: 1\nnode: p\nid: T1\ntitle: A\nstatus: todo\nparent: S1\n---\n",
        )
        .write(
            "p/tasks/T2-b.md",
            "---\nariane: 1\nnode: p\nid: T2\ntitle: B\nstatus: todo\n---\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        let epic_of = |reference| epic(&tree, find(&tree, reference));
        assert_eq!(epic_of("p#S1").as_deref(), Some("p#E1"));
        assert_eq!(epic_of("p#T1").as_deref(), Some("p#E1"));
        assert_eq!(epic_of("p#T2"), None);
        assert_eq!(epic_of("p#E1"), None);
    }

    #[test]
    fn a_story_knows_its_children_and_a_task_its_dependents() {
        let tree = sample();
        let story = of(&tree, find(&tree, "p#S1"));
        let ids: Vec<_> = story.children.iter().map(|row| row.id).collect();
        assert_eq!(ids, [Some("T1"), Some("T2")]);
        assert!(story.parent.is_none());
        let first = of(&tree, find(&tree, "p#T1"));
        assert_eq!(first.parent.unwrap().id, Some("S1"));
        let dependents: Vec<_> = first.dependents.iter().map(|row| row.id).collect();
        assert_eq!(dependents, [Some("T2")]);
        let second = of(&tree, find(&tree, "p#T2"));
        let targets: Vec<_> = second
            .depends_on
            .iter()
            .map(|row| row.reference.clone())
            .collect();
        assert_eq!(targets, [Some("p#T1".to_owned()), Some("q#T9".to_owned())]);
        assert_eq!(second.unresolved, ["p#T404"]);
        let remote = of(&tree, find(&tree, "q#T9"));
        assert_eq!(remote.dependents.len(), 1);
    }
}

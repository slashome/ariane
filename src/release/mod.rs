//! The effective release of an item (SPEC.md §1.3, ADR-0023).
//!
//! `release` is decided on tasks and forecast on stories and epics: the
//! release a story or an epic carries is the default of its tasks that carry
//! none. The nearest declaration wins, task, then story, then epic. This module
//! resolves that precedence once, for `ariane tasks --json`, `ariane show
//! --json` and the interface, and places the result in the node's declared
//! order. [`declare`] writes releases, [`close`] closes one, [`changelog`]
//! gives its changelog entry, [`command`] is `ariane release`.

pub mod changelog;
pub mod close;
pub mod command;
pub mod declare;

use std::collections::HashMap;

use serde::Serialize;

use crate::item::Kind;
use crate::node::{Category, Effective};
use crate::rank::{self, Placement};
use crate::relations::Index;
use crate::tree::{Item, NodePath, Tree, parse_reference};

/// Where an item's effective release comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The item's own `release`.
    Own,
    /// The `release` of the task's story.
    Story,
    /// The `release` of the story's epic, or of the task's story's epic.
    Epic,
}

/// The release an item is planned in, once its story and epic are read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EffectiveRelease<'a> {
    /// The release's name, as written in the header that gives it.
    pub name: &'a str,
    /// Whose header gives it: the item itself, its story or its epic.
    pub source: Source,
    /// The item that gives it, as `<node>#<id>`.
    pub from: Option<String>,
    /// Its place in the delivery order of the item's node, from 0; `None` when
    /// no release list applies or the list lacks it.
    pub position: Option<usize>,
    /// Whether the declared release is a parking one, never scheduled.
    pub parking: bool,
}

/// The effective release of `item`, `config` being its node's effective
/// configuration: its own `release`, else its story's for a task, else its
/// epic's for a task or a story. Other kinds only have their own. `None` when
/// no header of the chain carries one.
pub fn effective<'a>(
    tree: &'a Tree,
    item: &'a Item,
    config: &Effective,
) -> Option<EffectiveRelease<'a>> {
    let (source, holder, name) = chain(tree, item)
        .into_iter()
        .find_map(|(source, holder)| Some((source, holder, holder.document.text("release")?)))?;
    let declared = config.release(name);
    Some(EffectiveRelease {
        name,
        source,
        from: holder.reference(),
        position: declared.map(|(position, _)| position),
        parking: declared.is_some_and(|(_, release)| release.parking),
    })
}

/// The items whose `release` applies to `item`, nearest first: itself, then
/// its story and that story's epic for a task, or its epic for a story.
fn chain<'a>(tree: &'a Tree, item: &'a Item) -> Vec<(Source, &'a Item)> {
    let mut chain = vec![(Source::Own, item)];
    let story = match item.kind {
        Kind::Task => parent(tree, item).filter(|parent| parent.kind == Kind::Story),
        Kind::Story => Some(item),
        _ => None,
    };
    if let Some(story) = story.filter(|story| !std::ptr::eq(*story, item)) {
        chain.push((Source::Story, story));
    }
    if let Some(epic) = story
        .and_then(|story| parent(tree, story))
        .filter(|parent| parent.kind == Kind::Epic)
    {
        chain.push((Source::Epic, epic));
    }
    chain
}

/// The item `child`'s `parent` points to, if it resolves.
fn parent<'a>(tree: &'a Tree, child: &Item) -> Option<&'a Item> {
    let (node, id) = parse_reference(child.document.text("parent")?, &child.node)?;
    tree.items
        .iter()
        .find(|candidate| candidate.node == node && candidate.id() == Some(id.as_str()))
}

/// An epic, a story or a task that plans against a node's release list.
#[derive(Debug, Clone)]
pub struct Listed<'a> {
    /// The item.
    pub item: &'a Item,
    /// Its effective release's name, `None` when no header of its chain has one.
    pub release: Option<&'a str>,
    /// The category of its status, `None` for a status its workflow lacks.
    pub category: Option<Category>,
    /// Its `parent`, when it resolves.
    pub parent: Option<&'a Item>,
    /// The items whose `parent` is this one, archived ones included.
    pub children: Vec<&'a Item>,
}

impl Listed<'_> {
    /// Tells whether its status is done or dropped.
    pub fn is_closed(&self) -> bool {
        self.category.is_some_and(Category::is_closed)
    }

    /// Tells whether its status is in the `done` category.
    pub fn is_done(&self) -> bool {
        self.category == Some(Category::Done)
    }

    /// Tells whether it is work of its own: a task, or a story or an epic
    /// not split yet.
    pub fn is_work(&self) -> bool {
        self.item.kind == Kind::Task || self.children.is_empty()
    }
}

/// The epics, stories and tasks with an id whose node plans against the list
/// `node` declares, archived ones included, in the order of their node paths,
/// then epics, stories and tasks, then the manual order (ADR-0022).
pub fn listed<'a>(tree: &'a Tree, node: &NodePath) -> Vec<Listed<'a>> {
    let mut configs: HashMap<&NodePath, Effective> = HashMap::new();
    let mut items: Vec<&Item> = Vec::new();
    for item in tree.items.iter().filter(|item| item.node.is_within(node)) {
        if !matches!(item.kind, Kind::Task | Kind::Story | Kind::Epic) || item.id().is_none() {
            continue;
        }
        let effective = configs
            .entry(&item.node)
            .or_insert_with(|| tree.configs.effective(&item.node));
        if effective.releases_from.as_ref() == Some(node) {
            items.push(item);
        }
    }
    items.sort_by(|a, b| {
        a.node
            .cmp(&b.node)
            .then_with(|| kind_order(a.kind).cmp(&kind_order(b.kind)))
            .then_with(|| rank::compare(&Placement::of_item(a), &Placement::of_item(b)))
    });
    let index = Index::new(tree);
    let parent_of = |item: &Item| {
        item.document
            .text("parent")
            .and_then(|raw| index.resolve(raw, &item.node))
    };
    let mut children: HashMap<*const Item, Vec<&Item>> = HashMap::new();
    for item in &tree.items {
        if let Some(parent) = parent_of(item) {
            children
                .entry(std::ptr::from_ref(parent))
                .or_default()
                .push(item);
        }
    }
    items
        .into_iter()
        .map(|item| Listed {
            item,
            release: configs
                .get(&item.node)
                .and_then(|effective| effective_name(tree, item, effective)),
            category: item
                .document
                .text("status")
                .and_then(|status| tree.configs.category(&item.node, status)),
            parent: parent_of(item),
            children: children
                .get(&std::ptr::from_ref(item))
                .cloned()
                .unwrap_or_default(),
        })
        .collect()
}

/// The name of the effective release of `item`.
fn effective_name<'a>(tree: &'a Tree, item: &'a Item, config: &Effective) -> Option<&'a str> {
    effective(tree, item, config).map(|found| found.name)
}

/// Epics first, then stories, then tasks.
fn kind_order(kind: Kind) -> u8 {
    match kind {
        Kind::Epic => 0,
        Kind::Story => 1,
        _ => 2,
    }
}

/// The id of `item` as a line of `node`'s release shows it: bare in `node`,
/// `<node>#<id>` in another node.
pub fn shown_id(item: &Item, node: &NodePath) -> String {
    match (item.id(), item.node == *node) {
        (Some(id), true) => id.to_owned(),
        _ => item.reference().unwrap_or_default(),
    }
}

/// Unit tests of the release precedence.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::NodePath;
    use crate::tree::fixture::Repo;

    /// An item of node `p` with `id` and extra header lines.
    fn item(id: &str, extra: &str) -> String {
        format!(
            "---\nariane: 2\nnode: p\nid: {id}\ntitle: {id}\nstatus: todo\n{extra}---\n# {id}\n"
        )
    }

    /// An epic forecast in `2.0`, a story forecast in `1.0` under it, a story
    /// without release under it, and tasks under both stories.
    fn sample() -> Tree {
        let repo = Repo::new();
        repo.write(
            "p/node.toml",
            "[[release]]\nname = \"1.0\"\n\n[[release]]\nname = \"2.0\"\n\n[[release]]\nname = \"plus-tard\"\nparking = true\n",
        )
        .write("p/epics/E1-e.md", &item("E1", "release: \"2.0\"\n"))
        .write("p/stories/S1-a.md", &item("S1", "parent: E1\nrelease: \"1.0\"\n"))
        .write("p/stories/S2-b.md", &item("S2", "parent: E1\n"))
        .write("p/tasks/T1-own.md", &item("T1", "parent: S1\nrelease: plus-tard\n"))
        .write("p/tasks/T2-story.md", &item("T2", "parent: S1\n"))
        .write("p/tasks/T3-epic.md", &item("T3", "parent: S2\n"))
        .write("p/tasks/T4-none.md", &item("T4", ""))
        .write("p/tasks/T5-free.md", &item("T5", "release: \"9.9\"\n"));
        Tree::load(repo.root()).unwrap()
    }

    /// The effective release of item `id` of node `p`.
    fn of<'a>(tree: &'a Tree, id: &str) -> Option<EffectiveRelease<'a>> {
        let item = tree
            .items
            .iter()
            .find(|item| item.id() == Some(id))
            .unwrap();
        effective(tree, item, &tree.configs.effective(&NodePath::new("p")))
    }

    #[test]
    fn the_nearest_release_wins_task_then_story_then_epic() {
        let tree = sample();
        let own = of(&tree, "T1").unwrap();
        assert_eq!((own.name, own.source), ("plus-tard", Source::Own));
        assert_eq!((own.position, own.parking), (Some(2), true));
        let story = of(&tree, "T2").unwrap();
        assert_eq!((story.name, story.source), ("1.0", Source::Story));
        assert_eq!(story.from.as_deref(), Some("p#S1"));
        assert_eq!(story.position, Some(0));
        let epic = of(&tree, "T3").unwrap();
        assert_eq!((epic.name, epic.source), ("2.0", Source::Epic));
        assert_eq!(epic.from.as_deref(), Some("p#E1"));
        assert_eq!(epic.position, Some(1));
        let forecast = of(&tree, "S2").unwrap();
        assert_eq!((forecast.name, forecast.source), ("2.0", Source::Epic));
        assert_eq!(of(&tree, "E1").unwrap().source, Source::Own);
    }

    #[test]
    fn a_task_without_release_in_its_chain_has_none_and_an_undeclared_one_has_no_position() {
        let tree = sample();
        assert!(of(&tree, "T4").is_none());
        let free = of(&tree, "T5").unwrap();
        assert_eq!(free.name, "9.9");
        assert_eq!(free.position, None);
        assert!(!free.parking);
    }
}

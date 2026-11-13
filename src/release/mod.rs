//! The effective release of an item (SPEC.md §1.3, ADR-0023).
//!
//! `release` is decided on tasks and forecast on stories and epics: the
//! release a story or an epic carries is the default of its tasks that carry
//! none. The nearest declaration wins, task, then story, then epic. This module
//! resolves that precedence once, for `ariane tasks --json`, `ariane show
//! --json` and the interface, and places the result in the node's declared
//! order. [`declare`] writes releases, [`command`] is `ariane release`.

pub mod command;
pub mod declare;

use serde::Serialize;

use crate::item::Kind;
use crate::node::Effective;
use crate::tree::{Item, Tree, parse_reference};

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

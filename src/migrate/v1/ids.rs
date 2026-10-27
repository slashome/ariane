//! Allocation of item ids during migration 0 → 1 (SPEC.md §1.4).
//!
//! Every item needs an `id` unique within its node. In order of preference, an
//! item keeps the `id` it already declares, takes a code prefix from its file
//! name (`b3-…` gives `b3`, `T038-…` gives `T038`), or receives a new one: the
//! prefix of its kind (T task or backlog, E epic, S story, R reflexion, D ressource)
//! followed by the next free number of that prefix in the node, zero-padded to
//! three digits. Allocation runs over the files sorted by path, so it is the same
//! on every run. Files are never renamed.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::schema::{id_prefix, is_valid_id};
use crate::tree::{Item, NodePath, Tree};

/// The id chosen for each item file, keyed by its path relative to the repository root.
#[derive(Debug, Default)]
pub struct Allocations(HashMap<PathBuf, String>);

impl Allocations {
    /// Chooses the id of every item of `scope`, node by node, in path order.
    pub fn plan(tree: &Tree, scope: Option<&NodePath>) -> Allocations {
        let mut by_node: BTreeMap<&NodePath, Vec<&Item>> = BTreeMap::new();
        for item in &tree.items {
            if scope.is_none_or(|scope| item.node.is_within(scope)) {
                by_node.entry(&item.node).or_default().push(item);
            }
        }
        let mut allocations = Allocations::default();
        for mut items in by_node.into_values() {
            items.sort_by(|a, b| a.relative.cmp(&b.relative));
            allocations.plan_node(&items);
        }
        allocations
    }

    /// The id chosen for the file at `relative`, if it is in the plan.
    pub fn id_of(&self, relative: &Path) -> Option<&str> {
        self.0.get(relative).map(String::as_str)
    }

    /// Chooses the ids of one node's items: declared ids first, then file-name
    /// prefixes not already taken, then new ids per kind prefix, numbered after the
    /// highest existing one and never reusing a taken id.
    fn plan_node(&mut self, items: &[&Item]) {
        let mut taken: BTreeSet<String> = items
            .iter()
            .filter_map(|item| declared_id(item))
            .map(str::to_owned)
            .collect();
        let mut pending: BTreeMap<char, Vec<PathBuf>> = BTreeMap::new();
        for item in items {
            if let Some(id) = declared_id(item) {
                self.0.insert(item.relative.clone(), id.to_owned());
                continue;
            }
            match filename_id(&item.relative) {
                Some(id) if taken.insert(id.clone()) => {
                    self.0.insert(item.relative.clone(), id);
                }
                _ => pending
                    .entry(id_prefix(item.kind))
                    .or_default()
                    .push(item.relative.clone()),
            }
        }
        for (prefix, relatives) in pending {
            let mut next = taken
                .iter()
                .filter_map(|id| allocated_number(id, prefix))
                .max()
                .unwrap_or(0)
                + 1;
            for relative in relatives {
                let mut id = format!("{prefix}{next:03}");
                while taken.contains(&id) {
                    next += 1;
                    id = format!("{prefix}{next:03}");
                }
                taken.insert(id.clone());
                self.0.insert(relative, id);
                next += 1;
            }
        }
    }
}

/// The `id` the item already declares, when it is a valid id.
fn declared_id(item: &Item) -> Option<&str> {
    item.document.text("id").filter(|id| is_valid_id(id))
}

/// A code prefix at the start of the file name: letters followed by digits,
/// before the first `-`. A purely numeric prefix, such as a Monday ticket
/// number, or a word such as `fusionner` is not an id.
fn filename_id(relative: &Path) -> Option<String> {
    let stem = relative.file_stem()?.to_str()?;
    let (prefix, _) = stem.split_once('-')?;
    let letters = prefix.trim_end_matches(|c: char| c.is_ascii_digit());
    let is_code = !letters.is_empty()
        && letters.len() < prefix.len()
        && letters.chars().all(|c| c.is_ascii_alphabetic());
    is_code.then(|| prefix.to_owned())
}

/// The number of an id made of `prefix` and digits, such as 38 for `T038`.
fn allocated_number(id: &str, prefix: char) -> Option<u32> {
    id.strip_prefix(prefix)?.parse().ok()
}

/// Unit tests of id allocation.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    #[test]
    fn keeps_declared_ids_takes_code_prefixes_and_allocates_after_the_highest_number() {
        let repo = Repo::new();
        repo.write("a/tasks/T007-declared.md", "---\nid: T007\n---\n# A\n")
            .write("a/tasks/b3-coded.md", "# B\n")
            .write("a/tasks/plain-words.md", "# C\n")
            .write("a/tasks/1234-numeric.md", "# D\n");
        let tree = crate::tree::Tree::load(repo.root()).unwrap();
        let allocations = Allocations::plan(&tree, None);
        let id = |name: &str| {
            allocations
                .id_of(Path::new(&format!("tree/a/tasks/{name}")))
                .unwrap()
                .to_owned()
        };
        assert_eq!(id("T007-declared.md"), "T007");
        assert_eq!(id("b3-coded.md"), "b3");
        assert_eq!(id("1234-numeric.md"), "T008");
        assert_eq!(id("plain-words.md"), "T009");
    }

    #[test]
    fn a_code_prefix_already_taken_in_the_node_is_not_reused() {
        let repo = Repo::new();
        repo.write("a/tasks/b3-first.md", "# A\n")
            .write("a/archives/tasks/b3-second.md", "# B\n");
        let tree = crate::tree::Tree::load(repo.root()).unwrap();
        let allocations = Allocations::plan(&tree, None);
        let ids: BTreeSet<_> = tree
            .items
            .iter()
            .map(|item| allocations.id_of(&item.relative).unwrap().to_owned())
            .collect();
        assert_eq!(ids, BTreeSet::from(["T001".to_owned(), "b3".to_owned()]));
    }

    #[test]
    fn ids_follow_the_specification_pattern() {
        assert!(is_valid_id("T038"));
        assert!(is_valid_id("b3"));
        assert!(!is_valid_id("3b"));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("T 1"));
    }

    #[test]
    fn each_kind_draws_from_its_own_prefix_sequence() {
        let repo = Repo::new();
        repo.write("a/tasks/T004-declared.md", "---\nid: T004\n---\n# A\n")
            .write("a/tasks/plain.md", "# B\n")
            .write("a/backlog/idea.md", "# C\n")
            .write("a/stories/epic.md", "# D\n")
            .write("a/reflexions/R002-old.md", "# E\n")
            .write("a/reflexions/question.md", "# F\n")
            .write("a/ressources/site.md", "# G\n")
            .write("a/archives/stories/done.md", "# H\n")
            .write("a/archives/loose.md", "# I\n");
        let tree = crate::tree::Tree::load(repo.root()).unwrap();
        let allocations = Allocations::plan(&tree, None);
        let id = |relative: &str| {
            allocations
                .id_of(Path::new(&format!("tree/a/{relative}")))
                .unwrap()
                .to_owned()
        };
        assert_eq!(id("archives/loose.md"), "T005");
        assert_eq!(id("backlog/idea.md"), "T006");
        assert_eq!(id("tasks/plain.md"), "T007");
        assert_eq!(id("archives/stories/done.md"), "S001");
        assert_eq!(id("stories/epic.md"), "S002");
        assert_eq!(id("reflexions/R002-old.md"), "R002");
        assert_eq!(id("reflexions/question.md"), "R003");
        assert_eq!(id("ressources/site.md"), "D001");
    }
}

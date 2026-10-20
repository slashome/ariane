use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use crate::tree::{Item, NodePath, Tree};

const ALLOCATED_PREFIX: char = 'T';

#[derive(Debug, Default)]
pub struct Allocations(HashMap<PathBuf, String>);

impl Allocations {
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

    pub fn id_of(&self, relative: &Path) -> Option<&str> {
        self.0.get(relative).map(String::as_str)
    }

    fn plan_node(&mut self, items: &[&Item]) {
        let mut taken: BTreeSet<String> = items
            .iter()
            .filter_map(|item| declared_id(item))
            .map(str::to_owned)
            .collect();
        let mut pending = Vec::new();
        for item in items {
            if let Some(id) = declared_id(item) {
                self.0.insert(item.relative.clone(), id.to_owned());
                continue;
            }
            match filename_id(&item.relative) {
                Some(id) if taken.insert(id.clone()) => {
                    self.0.insert(item.relative.clone(), id);
                }
                _ => pending.push(item.relative.clone()),
            }
        }
        let mut next = taken
            .iter()
            .filter_map(|id| allocated_number(id))
            .max()
            .unwrap_or(0)
            + 1;
        for relative in pending {
            let mut id = format!("{ALLOCATED_PREFIX}{next:03}");
            while taken.contains(&id) {
                next += 1;
                id = format!("{ALLOCATED_PREFIX}{next:03}");
            }
            taken.insert(id.clone());
            self.0.insert(relative, id);
            next += 1;
        }
    }
}

pub fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn declared_id(item: &Item) -> Option<&str> {
    item.document.text("id").filter(|id| is_valid_id(id))
}

fn filename_id(relative: &Path) -> Option<String> {
    let stem = relative.file_stem()?.to_str()?;
    let (prefix, _) = stem.split_once('-')?;
    let letters = prefix.trim_end_matches(|c: char| c.is_ascii_digit());
    let is_code = !letters.is_empty()
        && letters.len() < prefix.len()
        && letters.chars().all(|c| c.is_ascii_alphabetic());
    is_code.then(|| prefix.to_owned())
}

fn allocated_number(id: &str) -> Option<u32> {
    id.strip_prefix(ALLOCATED_PREFIX)?.parse().ok()
}

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
}

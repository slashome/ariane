//! The releases a release waits for (ADR-0030).
//!
//! A `[[release]]` entry MAY name, in `depends_on`, releases that must be
//! delivered before any of its work starts: `<node>@<release>` for a release
//! of the list `<node>` plans against, or `@<release>` for a release of the
//! same list. A reference is resolved to the [`Key`] of the release it names,
//! the node declaring that list and the release's name, whatever node of the
//! list it was written through. `doctor` reports a reference to an unknown
//! node, an unknown release or a parking one, and a cycle; the forecast orders
//! and gates releases with the resolved keys; `ariane release rename` and
//! `remove` keep the references of other lists right.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use crate::tree::{NodePath, Tree};

/// The node part and the release part of a reference, trimmed; `None` when it
/// has no `@`, or no release name after it.
pub fn split(reference: &str) -> Option<(&str, &str)> {
    let (node, name) = reference.split_once('@')?;
    let name = name.trim();
    (!name.is_empty()).then_some((node.trim(), name))
}

/// A declared release, named by the node whose list declares it and its name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    /// The node whose `node.toml` declares the list.
    pub list: NodePath,
    /// The release's name.
    pub name: String,
}

impl Key {
    /// The key of the release `name` of the list `list` declares.
    pub fn new(list: &NodePath, name: &str) -> Key {
        Key {
            list: list.clone(),
            name: name.to_owned(),
        }
    }
}

/// Shows a key as `<node>@<release>`, `@<release>` for the home node.
impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.list.as_str(), self.name)
    }
}

/// What a reference names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A declared release; `parking` when it is never scheduled.
    Found {
        /// The release named.
        key: Key,
        /// Whether it is a parking release.
        parking: bool,
    },
    /// No node of that path.
    UnknownNode(NodePath),
    /// The node exists, but its list does not declare the release, or no
    /// list applies to it.
    UnknownRelease {
        /// The node named.
        node: NodePath,
        /// The release named.
        name: String,
        /// The node whose list applies to it, if any.
        list: Option<NodePath>,
    },
}

/// What `reference`, written in the list `declarer` declares, names; `None`
/// when it is not a reference at all.
pub fn resolve(tree: &Tree, declarer: &NodePath, reference: &str) -> Option<Target> {
    let (node, name) = split(reference)?;
    let node = if node.is_empty() {
        declarer.clone()
    } else {
        NodePath::new(node.trim_matches('/'))
    };
    if !tree.node_dir(&node).is_dir() {
        return Some(Target::UnknownNode(node));
    }
    let effective = tree.configs.effective(&node);
    Some(match (effective.release(name), &effective.releases_from) {
        (Some((_, release)), Some(list)) => Target::Found {
            key: Key::new(list, name),
            parking: release.parking,
        },
        _ => Target::UnknownRelease {
            node,
            name: name.to_owned(),
            list: effective.releases_from,
        },
    })
}

/// One `depends_on` reference of a declared release.
#[derive(Debug, Clone)]
pub struct Dependency {
    /// The release that declares it.
    pub from: Key,
    /// The reference as written.
    pub written: String,
    /// What it names.
    pub target: Target,
}

/// Every `depends_on` reference of every `[[release]]` of the tree, in the
/// order of node paths, then of the lists, then of the references.
pub fn all(tree: &Tree) -> Vec<Dependency> {
    let mut found = Vec::new();
    for (node, file) in &tree.configs.files {
        for release in file.release.iter().flatten() {
            for written in &release.depends_on {
                if let Some(target) = resolve(tree, node, written) {
                    found.push(Dependency {
                        from: Key::new(node, &release.name),
                        written: written.clone(),
                        target,
                    });
                }
            }
        }
    }
    found
}

/// The releases each release waits for, among the releases found.
pub fn graph(dependencies: &[Dependency]) -> BTreeMap<Key, BTreeSet<Key>> {
    let mut edges: BTreeMap<Key, BTreeSet<Key>> = BTreeMap::new();
    for dependency in dependencies {
        if let Target::Found { key, .. } = &dependency.target {
            edges
                .entry(dependency.from.clone())
                .or_default()
                .insert(key.clone());
        }
    }
    edges
}

/// The cycles of `edges`, each as the path from its first release back to it,
/// that release written at both ends; a release appears in one cycle at most.
pub fn cycles(edges: &BTreeMap<Key, BTreeSet<Key>>) -> Vec<Vec<Key>> {
    let mut reported: BTreeSet<Key> = BTreeSet::new();
    let mut found = Vec::new();
    for start in edges.keys() {
        if reported.contains(start) {
            continue;
        }
        let Some(path) = path_back(edges, start) else {
            continue;
        };
        reported.extend(path.iter().cloned());
        found.push(path);
    }
    found
}

/// The shortest path from `start` back to itself, `start` at both ends.
fn path_back(edges: &BTreeMap<Key, BTreeSet<Key>>, start: &Key) -> Option<Vec<Key>> {
    let mut previous: BTreeMap<&Key, &Key> = BTreeMap::new();
    let mut queue: VecDeque<&Key> = VecDeque::from([start]);
    while let Some(current) = queue.pop_front() {
        for next in edges.get(current).into_iter().flatten() {
            if next == start {
                let mut path = vec![start.clone()];
                let mut step = current;
                while step != start {
                    path.push(step.clone());
                    step = previous[step];
                }
                path.push(start.clone());
                path.reverse();
                return Some(path);
            }
            if !previous.contains_key(next) {
                previous.insert(next, current);
                queue.push_back(next);
            }
        }
    }
    None
}

/// The releases of other lists, or of the same one, whose `depends_on`
/// names the release `name` of the list `list` declares, with the reference
/// as written.
pub fn dependents(tree: &Tree, list: &NodePath, name: &str) -> Vec<Dependency> {
    let wanted = Key::new(list, name);
    all(tree)
        .into_iter()
        .filter(
            |dependency| matches!(&dependency.target, Target::Found { key, .. } if *key == wanted),
        )
        .collect()
}

/// Refuses to give the release `name` of the list `list` declares the
/// dependencies `written`: a reference that is not one, that names an unknown
/// node, an unknown release or a parking one, or that would close a cycle;
/// the message says which and why.
pub fn check(tree: &Tree, list: &NodePath, name: &str, written: &[String]) -> Result<(), String> {
    let own = Key::new(list, name);
    let mut edges = graph(
        &all(tree)
            .into_iter()
            .filter(|dependency| dependency.from != own)
            .collect::<Vec<_>>(),
    );
    for reference in written {
        let target = resolve(tree, list, reference).ok_or_else(|| {
            format!("`{reference}` is not `<node>@<release>`, nor `@<release>` for a release of `{list}`")
        })?;
        match target {
            Target::Found {
                key,
                parking: false,
            } => {
                edges.entry(own.clone()).or_default().insert(key);
            }
            Target::Found { key, .. } => {
                return Err(format!(
                    "`{key}` is a parking release, never delivered: nothing can wait for it"
                ));
            }
            Target::UnknownNode(node) => return Err(format!("`{node}` is not a node")),
            Target::UnknownRelease { node, name, .. } => {
                return Err(format!("`{node}` plans against no release `{name}`"));
            }
        }
    }
    match path_back(&edges, &own) {
        Some(cycle) => Err(format!(
            "releases would wait for each other: {}",
            cycle
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" -> ")
        )),
        None => Ok(()),
    }
}

/// The reference `written`, read from the list `declarer` declares, with its
/// release part changed to `new` and its node part kept, when it names the
/// release `old`; `None` when it names another release.
pub fn renamed(
    tree: &Tree,
    declarer: &NodePath,
    written: &str,
    old: &Key,
    new: &str,
) -> Option<String> {
    let Some(Target::Found { key, .. }) = resolve(tree, declarer, written) else {
        return None;
    };
    let (node, _) = split(written)?;
    (key == *old).then(|| format!("{node}@{new}"))
}

/// Unit tests of release references.
#[cfg(test)]
mod tests {
    use super::*;

    /// The key of the release `name` of the list `list` declares.
    fn key(list: &str, name: &str) -> Key {
        Key::new(&NodePath::new(list), name)
    }

    #[test]
    fn a_reference_is_a_node_and_a_release_around_an_at_sign() {
        assert_eq!(split("cmc/cmc-lib@1.1.0"), Some(("cmc/cmc-lib", "1.1.0")));
        assert_eq!(split("@1.0"), Some(("", "1.0")));
        assert_eq!(split(" p @ a@b "), Some(("p", "a@b")));
        assert_eq!(split("1.0"), None);
        assert_eq!(split("p@ "), None);
    }

    #[test]
    fn a_cycle_is_reported_once_from_its_first_release() {
        let mut edges: BTreeMap<Key, BTreeSet<Key>> = BTreeMap::new();
        edges
            .entry(key("a", "1"))
            .or_default()
            .insert(key("b", "1"));
        edges
            .entry(key("b", "1"))
            .or_default()
            .insert(key("c", "1"));
        edges
            .entry(key("c", "1"))
            .or_default()
            .insert(key("a", "1"));
        edges
            .entry(key("d", "1"))
            .or_default()
            .insert(key("d", "1"));
        edges
            .entry(key("e", "1"))
            .or_default()
            .insert(key("a", "1"));
        let shown: Vec<Vec<String>> = cycles(&edges)
            .iter()
            .map(|cycle| cycle.iter().map(Key::to_string).collect())
            .collect();
        assert_eq!(
            shown,
            [vec!["a@1", "b@1", "c@1", "a@1"], vec!["d@1", "d@1"],]
        );
    }
}

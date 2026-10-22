//! The content repository loaded in memory: its nodes and its items.
//!
//! Under `tree/`, every directory is a potential **node**, whose logical path is
//! its location (`cmc/cmc-lib`, ADR-0001); the root `tree/` itself is the home
//! node, whose path is empty. A node holds **item directories** (`tasks/`,
//! `stories/`, `reflexions/`, `ressources/`, and `archives/` with the same
//! subdirectories); every other subdirectory is walked as a child node. The
//! former `backlog/` directory is still read, its items as tasks marked
//! [`Item::legacy`], so that `doctor` reports them and `ariane fix` moves them.
//!
//! [`Tree::load`](crate::tree::Tree::load) reads every item once and parses its front-matter, so that each
//! command works on the same snapshot without touching the disk again. Files and
//! names that cannot be read are collected in [`Tree::unreadable`](crate::tree::Tree::unreadable) for `doctor`
//! to report, never silently skipped.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::item::{Document, Kind, LEGACY_BACKLOG};
use crate::node::Configs;
use crate::schema::is_valid_id;

/// File that makes a directory a node even when it holds no item (PLAN §2).
const INDEX: &str = "INDEX.md";
/// Markdown files that sit among items but are not items (SPEC.md §1.1).
const NOT_ITEMS: [&str; 3] = ["INDEX.md", "HANDOFF.md", "README.md"];
/// Directory holding the archived items of a node.
const ARCHIVES: &str = "archives";

/// Logical path of a node, such as `cmc/cmc-lib`; empty for the home node.
///
/// A newtype (a struct wrapping one value) keeps node paths from being mixed up
/// with any other string. `#[serde(transparent)]` serializes it as the plain
/// string it wraps.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize)]
#[serde(transparent)]
pub struct NodePath(String);

impl NodePath {
    /// Wraps a path. `impl Into<String>` accepts a `&str` as well as a `String`,
    /// converting only when needed.
    pub fn new(path: impl Into<String>) -> NodePath {
        NodePath(path.into())
    }

    /// The path as a string slice, borrowed from the node path.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Tells whether this is the home node, the root of the tree.
    pub fn is_home(&self) -> bool {
        self.0.is_empty()
    }

    /// Every node from the home node down to this one, this one included:
    /// `a/b` gives `["", "a", "a/b"]`. Used to apply inherited `node.toml` files in
    /// order (SPEC.md §1.5).
    pub fn ancestors_from_root(&self) -> Vec<NodePath> {
        let mut chain = vec![NodePath::default()];
        let mut prefix = String::new();
        for segment in self.0.split('/').filter(|segment| !segment.is_empty()) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(segment);
            chain.push(NodePath(prefix.clone()));
        }
        chain
    }

    /// Tells whether this node is `scope` or one of its descendants. `cmc/lib-legacy`
    /// is not within `cmc/lib`: the remainder after the prefix must start with `/`.
    pub fn is_within(&self, scope: &NodePath) -> bool {
        scope.is_home()
            || self.0 == scope.0
            || self
                .0
                .strip_prefix(&scope.0)
                .is_some_and(|rest| rest.starts_with('/'))
    }

    /// The path of the child node named `segment`.
    fn child(&self, segment: &str) -> NodePath {
        if self.is_home() {
            NodePath(segment.to_owned())
        } else {
            NodePath(format!("{}/{segment}", self.0))
        }
    }
}

/// Shows the home node as `(home)` and any other node as its path.
impl fmt::Display for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_home() {
            f.write_str("(home)")
        } else {
            f.write_str(&self.0)
        }
    }
}

/// Parses a reference (SPEC.md §1.4): `T022` stands for an item of the
/// `current` node, `node/path#T022` for an item of another node, `#T022` for
/// an item of the home node. `None` when the id part is not a valid id.
/// `bool::then` turns a condition into an `Option`.
pub fn parse_reference(raw: &str, current: &NodePath) -> Option<(NodePath, String)> {
    let (node, id) = match raw.split_once('#') {
        Some((node, id)) => (NodePath::new(node), id),
        None => (current.clone(), raw),
    };
    is_valid_id(id).then(|| (node, id.to_owned()))
}

/// The reference of the item `id` of `node` written so that it reads the same
/// from any node: `<node>#<id>`, `#<id>` for the home node.
pub fn full_reference(node: &NodePath, id: &str) -> String {
    format!("{}#{id}", node.as_str())
}

/// A directory of the tree that is a node.
#[derive(Debug)]
pub struct Node {
    pub path: NodePath,
    /// Whether the node has its `INDEX.md` entry point; `doctor` warns when it does not.
    pub has_index: bool,
}

/// One item file, with where it sits and what it contains.
#[derive(Debug)]
pub struct Item {
    /// Node holding the item's directory.
    pub node: NodePath,
    /// Kind given by the directory.
    pub kind: Kind,
    /// Whether the item sits under `archives/`.
    pub archived: bool,
    /// Whether the item sits in the former `backlog/` directory, which
    /// `ariane fix` replaces by `tasks/`.
    pub legacy: bool,
    /// Absolute path of the file.
    pub file: PathBuf,
    /// Path relative to the repository root, for messages.
    pub relative: PathBuf,
    /// The parsed header and the body.
    pub document: Document,
}

impl Item {
    /// The item's `id`, when its header carries a valid one.
    pub fn id(&self) -> Option<&str> {
        self.document.text("id").filter(|id| is_valid_id(id))
    }

    /// The item's reference as read from any node, `<node>#<id>`.
    pub fn reference(&self) -> Option<String> {
        self.id().map(|id| full_reference(&self.node, id))
    }
}

/// The whole content repository: nodes, items, `node.toml` files, and what could not be read.
#[derive(Debug)]
pub struct Tree {
    /// Root of the content repository (the directory holding `tree/`).
    pub root: PathBuf,
    pub nodes: Vec<Node>,
    pub items: Vec<Item>,
    /// Every `node.toml` found along the nodes, parsed.
    pub configs: Configs,
    /// Files or names that could not be read, with the reason.
    pub unreadable: Vec<(PathBuf, String)>,
}

impl Tree {
    /// Walks `tree/` under `root`, reads every item, then loads the `node.toml` files
    /// of every node and of its ancestors.
    ///
    /// # Errors
    ///
    /// Fails when a directory cannot be listed. An unreadable item file does not
    /// fail the load; it is recorded in [`Tree::unreadable`].
    pub fn load(root: &Path) -> Result<Tree> {
        let mut tree = Tree {
            root: root.to_path_buf(),
            nodes: Vec::new(),
            items: Vec::new(),
            configs: Configs::default(),
            unreadable: Vec::new(),
        };
        let top = root.join("tree");
        tree.walk_node(&top, NodePath::default())
            .with_context(|| format!("reading {}", top.display()))?;
        let paths: Vec<NodePath> = tree.nodes.iter().map(|node| node.path.clone()).collect();
        tree.configs = Configs::load(&top, &paths);
        Ok(tree)
    }

    /// The `tree/` directory of the repository.
    pub fn tree_dir(&self) -> PathBuf {
        self.root.join("tree")
    }

    /// The directory of a node, built segment by segment under `tree/`.
    pub fn node_dir(&self, node: &NodePath) -> PathBuf {
        let mut dir = self.tree_dir();
        for segment in node
            .as_str()
            .split('/')
            .filter(|segment| !segment.is_empty())
        {
            dir.push(segment);
        }
        dir
    }

    /// The items of `scope` and its descendants, or every item when `scope` is
    /// `None`.
    ///
    /// Returns a lazy iterator: nothing is filtered until the caller consumes it.
    /// The `'a` lifetime ties the yielded references to both the tree and the scope,
    /// and `move` hands the scope reference to the filtering closure.
    pub fn items_within<'a>(
        &'a self,
        scope: Option<&'a NodePath>,
    ) -> impl Iterator<Item = &'a Item> {
        self.items
            .iter()
            .filter(move |item| scope.is_none_or(|scope| item.node.is_within(scope)))
    }

    /// Records one directory and walks its subdirectories: item directories are
    /// collected for this node, any other directory is walked as a child node.
    ///
    /// A directory counts as a node when it has an `INDEX.md` or holds items. Hidden
    /// directories are skipped. The recursion goes as deep as the tree does.
    ///
    /// # Errors
    ///
    /// Fails when a directory cannot be listed.
    fn walk_node(&mut self, dir: &Path, path: NodePath) -> Result<()> {
        let has_index = dir.join(INDEX).is_file();
        let mut has_items = false;
        for entry in sorted_entries(dir)? {
            if !entry.is_dir() {
                continue;
            }
            let Some(name) = self.utf8_name(&entry) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            if let Some(kind) = Kind::from_dir(&name) {
                has_items = true;
                let place = Place {
                    kind,
                    archived: false,
                    legacy: name == LEGACY_BACKLOG,
                };
                self.collect_items(&entry, &path, place)?;
            } else if name == ARCHIVES {
                has_items = true;
                self.collect_archives(&entry, &path)?;
            } else {
                self.walk_node(&entry, path.child(&name))?;
            }
        }
        if has_index || has_items {
            self.nodes.push(Node { path, has_index });
        }
        Ok(())
    }

    /// Collects the items under `archives/`: each subdirectory gives its kind, and a
    /// file placed directly in `archives/` has an unknown kind.
    ///
    /// # Errors
    ///
    /// Fails when a directory cannot be listed.
    fn collect_archives(&mut self, dir: &Path, node: &NodePath) -> Result<()> {
        for entry in sorted_entries(dir)? {
            if entry.is_dir() {
                let Some(name) = self.utf8_name(&entry) else {
                    continue;
                };
                let place = Place {
                    kind: Kind::from_dir(&name).unwrap_or(Kind::Unknown),
                    archived: true,
                    legacy: name == LEGACY_BACKLOG,
                };
                self.collect_items(&entry, node, place)?;
            } else if is_item_file(&entry) {
                let place = Place {
                    kind: Kind::Unknown,
                    archived: true,
                    legacy: false,
                };
                self.push_item(&entry, node, place);
            }
        }
        Ok(())
    }

    /// Collects every item file under `dir`, subdirectories included, in the
    /// same place.
    ///
    /// # Errors
    ///
    /// Fails when a directory cannot be listed.
    fn collect_items(&mut self, dir: &Path, node: &NodePath, place: Place) -> Result<()> {
        for entry in sorted_entries(dir)? {
            if entry.is_dir() {
                self.collect_items(&entry, node, place)?;
            } else if is_item_file(&entry) {
                self.push_item(&entry, node, place);
            }
        }
        Ok(())
    }

    /// Reads and parses one item file, or records it as unreadable.
    fn push_item(&mut self, file: &Path, node: &NodePath, place: Place) {
        let text = match std::fs::read_to_string(file) {
            Ok(text) => text,
            Err(error) => {
                self.unreadable
                    .push((file.to_path_buf(), error.to_string()));
                return;
            }
        };
        let relative = file.strip_prefix(&self.root).unwrap_or(file).to_path_buf();
        self.items.push(Item {
            node: node.clone(),
            kind: place.kind,
            archived: place.archived,
            legacy: place.legacy,
            file: file.to_path_buf(),
            relative,
            document: Document::parse(&text),
        });
    }

    /// The file name of `path` as UTF-8, or `None` after recording it as
    /// unreadable: node paths and item kinds are words, so a name that is not UTF-8
    /// cannot be placed in the tree, but it is still reported.
    fn utf8_name(&mut self, path: &Path) -> Option<String> {
        match path.file_name()?.to_str() {
            Some(name) => Some(name.to_owned()),
            None => {
                self.unreadable
                    .push((path.to_path_buf(), "the name is not valid UTF-8".to_owned()));
                None
            }
        }
    }
}

/// Where an item directory puts its items: their kind, whether they are
/// archived, and whether the directory is the former `backlog/`.
#[derive(Debug, Clone, Copy)]
struct Place {
    kind: Kind,
    archived: bool,
    legacy: bool,
}

/// Tells whether a file is an item: a `.md` file that is not `INDEX.md`, `HANDOFF.md` or `README.md`.
fn is_item_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "md")
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| !NOT_ITEMS.contains(&name))
}

/// The entries of a directory, sorted by path so that every run sees the same
/// order.
///
/// `collect::<std::io::Result<Vec<_>>>()` turns an iterator of results into one
/// result: the first failed entry makes the whole listing fail.
///
/// # Errors
///
/// Fails when the directory or one of its entries cannot be read.
fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    Ok(entries)
}

/// Test helper: a throwaway content repository on disk.
#[cfg(test)]
pub mod fixture {
    use std::path::Path;

    /// A temporary content repository, deleted when the value is dropped.
    pub struct Repo {
        /// The temporary directory; deleting it is `TempDir`'s job.
        pub dir: tempfile::TempDir,
    }

    impl Repo {
        /// Creates `.git/`, `tree/` and a home `INDEX.md`.
        pub fn new() -> Repo {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(dir.path().join(".git")).unwrap();
            std::fs::create_dir_all(dir.path().join("tree")).unwrap();
            std::fs::write(dir.path().join("tree/INDEX.md"), "# Home\n").unwrap();
            Repo { dir }
        }

        /// Root of the repository.
        pub fn root(&self) -> &Path {
            self.dir.path()
        }

        /// Writes a file under `tree/`, creating its directories, and returns the repository so calls can be chained.
        pub fn write(&self, relative: &str, text: &str) -> &Repo {
            let path = self.dir.path().join("tree").join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
            self
        }
    }
}

/// Unit tests of the tree walk and of node paths.
#[cfg(test)]
mod tests {
    use super::fixture::Repo;
    use super::*;

    #[test]
    fn items_belong_to_the_node_holding_their_directory() {
        let repo = Repo::new();
        repo.write("cmc/cmc-lib/INDEX.md", "# lib\n")
            .write("cmc/cmc-lib/tasks/t1.md", "# T1\n")
            .write("cmc/cmc-lib/archives/stories/s1.md", "# S1\n")
            .write("cmc/cmc-lib/archives/loose.md", "# loose\n")
            .write("cmc/cmc-lib/backlog/README.md", "# not an item\n")
            .write("cmc/cmc-lib/backlog/b1.md", "# B1\n")
            .write("cmc/cmc-lib/audits/report.md", "# not an item either\n");
        let tree = Tree::load(repo.root()).unwrap();
        let found: Vec<_> = tree
            .items
            .iter()
            .map(|item| (item.node.as_str(), item.kind, item.archived, item.legacy))
            .collect();
        assert_eq!(
            found,
            vec![
                ("cmc/cmc-lib", Kind::Unknown, true, false),
                ("cmc/cmc-lib", Kind::Story, true, false),
                ("cmc/cmc-lib", Kind::Task, false, true),
                ("cmc/cmc-lib", Kind::Task, false, false),
            ]
        );
    }

    #[test]
    fn a_directory_is_a_node_when_it_has_an_index_or_items() {
        let repo = Repo::new();
        repo.write("cmc/INDEX.md", "# cmc\n")
            .write("cmc/cmc-ui/tasks/t.md", "# t\n")
            .write("cmc/empty/notes.md", "# nothing\n");
        let tree = Tree::load(repo.root()).unwrap();
        let mut nodes: Vec<_> = tree
            .nodes
            .iter()
            .map(|node| node.path.to_string())
            .collect();
        nodes.sort();
        assert_eq!(nodes, vec!["(home)", "cmc", "cmc/cmc-ui"]);
    }

    #[test]
    fn scopes_include_descendants_but_not_siblings_sharing_a_prefix() {
        let scope = NodePath::new("cmc/cmc-lib");
        assert!(NodePath::new("cmc/cmc-lib").is_within(&scope));
        assert!(NodePath::new("cmc/cmc-lib/x").is_within(&scope));
        assert!(!NodePath::new("cmc/cmc-lib-legacy").is_within(&scope));
        assert!(NodePath::new("cmc").is_within(&NodePath::default()));
    }
}

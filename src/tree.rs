use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::item::{Document, Kind};
use crate::node::Configs;

const INDEX: &str = "INDEX.md";
const NOT_ITEMS: [&str; 3] = ["INDEX.md", "HANDOFF.md", "README.md"];
const ARCHIVES: &str = "archives";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize)]
#[serde(transparent)]
pub struct NodePath(String);

impl NodePath {
    pub fn new(path: impl Into<String>) -> NodePath {
        NodePath(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_home(&self) -> bool {
        self.0.is_empty()
    }

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

    pub fn is_within(&self, scope: &NodePath) -> bool {
        scope.is_home()
            || self.0 == scope.0
            || self
                .0
                .strip_prefix(&scope.0)
                .is_some_and(|rest| rest.starts_with('/'))
    }

    fn child(&self, segment: &str) -> NodePath {
        if self.is_home() {
            NodePath(segment.to_owned())
        } else {
            NodePath(format!("{}/{segment}", self.0))
        }
    }
}

impl fmt::Display for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_home() {
            f.write_str("(home)")
        } else {
            f.write_str(&self.0)
        }
    }
}

#[derive(Debug)]
pub struct Node {
    pub path: NodePath,
    pub has_index: bool,
}

#[derive(Debug)]
pub struct Item {
    pub node: NodePath,
    pub kind: Kind,
    pub archived: bool,
    pub file: PathBuf,
    pub relative: PathBuf,
    pub document: Document,
}

#[derive(Debug)]
pub struct Tree {
    pub root: PathBuf,
    pub nodes: Vec<Node>,
    pub items: Vec<Item>,
    pub configs: Configs,
    pub unreadable: Vec<(PathBuf, String)>,
}

impl Tree {
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

    pub fn tree_dir(&self) -> PathBuf {
        self.root.join("tree")
    }

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

    pub fn items_within<'a>(
        &'a self,
        scope: Option<&'a NodePath>,
    ) -> impl Iterator<Item = &'a Item> {
        self.items
            .iter()
            .filter(move |item| scope.is_none_or(|scope| item.node.is_within(scope)))
    }

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
                self.collect_items(&entry, &path, kind, false)?;
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

    fn collect_archives(&mut self, dir: &Path, node: &NodePath) -> Result<()> {
        for entry in sorted_entries(dir)? {
            if entry.is_dir() {
                let Some(name) = self.utf8_name(&entry) else {
                    continue;
                };
                let kind = Kind::from_dir(&name).unwrap_or(Kind::Unknown);
                self.collect_items(&entry, node, kind, true)?;
            } else if is_item_file(&entry) {
                self.push_item(&entry, node, Kind::Unknown, true);
            }
        }
        Ok(())
    }

    fn collect_items(
        &mut self,
        dir: &Path,
        node: &NodePath,
        kind: Kind,
        archived: bool,
    ) -> Result<()> {
        for entry in sorted_entries(dir)? {
            if entry.is_dir() {
                self.collect_items(&entry, node, kind, archived)?;
            } else if is_item_file(&entry) {
                self.push_item(&entry, node, kind, archived);
            }
        }
        Ok(())
    }

    fn push_item(&mut self, file: &Path, node: &NodePath, kind: Kind, archived: bool) {
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
            kind,
            archived,
            file: file.to_path_buf(),
            relative,
            document: Document::parse(&text),
        });
    }

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

fn is_item_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "md")
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| !NOT_ITEMS.contains(&name))
}

fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    Ok(entries)
}

#[cfg(test)]
pub mod fixture {
    use std::path::Path;

    pub struct Repo {
        pub dir: tempfile::TempDir,
    }

    impl Repo {
        pub fn new() -> Repo {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(dir.path().join(".git")).unwrap();
            std::fs::create_dir_all(dir.path().join("tree")).unwrap();
            std::fs::write(dir.path().join("tree/INDEX.md"), "# Home\n").unwrap();
            Repo { dir }
        }

        pub fn root(&self) -> &Path {
            self.dir.path()
        }

        pub fn write(&self, relative: &str, text: &str) -> &Repo {
            let path = self.dir.path().join("tree").join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
            self
        }
    }
}

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
            .write("cmc/cmc-lib/audits/report.md", "# not an item either\n");
        let tree = Tree::load(repo.root()).unwrap();
        let found: Vec<_> = tree
            .items
            .iter()
            .map(|item| (item.node.as_str(), item.kind, item.archived))
            .collect();
        assert_eq!(
            found,
            vec![
                ("cmc/cmc-lib", Kind::Unknown, true),
                ("cmc/cmc-lib", Kind::Story, true),
                ("cmc/cmc-lib", Kind::Task, false),
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

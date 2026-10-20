//! Finds the Ariane content repository on disk.
//!
//! A content repository is a git repository with a `tree/` directory at its root.
//! It is found, in this order: from `--repo` or the `ARIANE_REPO` environment
//! variable; otherwise from the current directory, first following its `_ariane`
//! link when there is one (a host repository materializes its node through a
//! symlink named `_ariane`, PLAN §4), then walking up the parent directories;
//! last from the `repo` key of `~/.config/ariane/config.toml`, so a command runs
//! from anywhere. Discovery also tells which node the current directory stands
//! in, which becomes the default scope of the commands (PLAN §10).
//!
//! Paths are kept as raw [`PathBuf`](std::path::PathBuf) values, never forced into UTF-8 (ADR-0005):
//! a badly named path must still be reportable.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::tree::NodePath;

/// Name of the link through which a host repository materializes its node.
const DIR_NAME: &str = "_ariane";

/// Where the command runs: the content repository, and the node the current
/// directory stands in, if any.
#[derive(Debug, PartialEq, Eq)]
pub struct Location {
    /// Root of the content repository.
    pub root: PathBuf,
    /// The node of the current directory: the target of its `_ariane` link, or
    /// the node directory it sits in inside the content repository. `None` when
    /// the command runs from anywhere else, in which case it covers the whole tree.
    pub here: Option<NodePath>,
}

/// The part of `config.toml` that discovery reads: the path of the content
/// repository (ADR-0013), used when the current directory leads to none.
#[derive(Debug, Default, Deserialize)]
struct Config {
    /// Path of the content repository; a leading `~/` stands for the home directory.
    repo: Option<String>,
}

/// Returns the content repository to work on and the node the current directory stands in.
///
/// The repository comes, in this order, from the explicit path (`--repo` or
/// `ARIANE_REPO`), from the current directory (following its `_ariane` link
/// first, then walking up with `Path::ancestors`, which yields the path itself
/// and then each parent), and last from the `repo` key of `config.toml`, so the
/// command also works from a directory that has nothing to do with Ariane.
///
/// # Errors
///
/// Fails when the explicit or configured path is not a content repository, when
/// `config.toml` cannot be parsed, or when no source yields a repository.
pub fn discover(explicit: Option<&Path>) -> Result<Location> {
    let cwd = std::env::current_dir().context("reading the current directory")?;
    if let Some(path) = explicit {
        let root = content_repository(path)?;
        let here = node_of(&root, &cwd);
        return Ok(Location { root, here });
    }
    let start = match cwd.join(DIR_NAME).canonicalize() {
        Ok(materialized) => materialized,
        Err(_) => cwd.clone(),
    };
    if let Some(root) = start.ancestors().find(|dir| is_content_repository(dir)) {
        let here = node_of(root, &start);
        return Ok(Location {
            root: root.to_path_buf(),
            here,
        });
    }
    let config = config_file();
    match configured_repository(&config)? {
        Some(path) => Ok(Location {
            root: content_repository(&path)?,
            here: None,
        }),
        None => bail!(
            "no Ariane content repository found from {}; pass --repo, set ARIANE_REPO, \
             or set `repo` in {}",
            cwd.display(),
            config.display()
        ),
    }
}

/// Resolves `path` and checks that it is a content repository.
///
/// # Errors
///
/// Fails when the path cannot be resolved or is not a content repository.
fn content_repository(path: &Path) -> Result<PathBuf> {
    let root = path
        .canonicalize()
        .with_context(|| format!("reading {}", path.display()))?;
    if !is_content_repository(&root) {
        bail!("{} is not an Ariane content repository", root.display());
    }
    Ok(root)
}

/// Returns the node whose directory contains `dir`, when `dir` lies under `<root>/tree`.
///
/// The node is the path relative to `tree/`; `tree/` itself is the home node.
/// A path segment that is not valid UTF-8 cannot name a node, so it yields `None`.
fn node_of(root: &Path, dir: &Path) -> Option<NodePath> {
    let tree = root.join("tree");
    let relative = dir.strip_prefix(&tree).ok()?;
    let segments = relative
        .components()
        .map(|component| component.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()?;
    Some(NodePath::new(segments.join("/")))
}

/// Path of `config.toml`: `$XDG_CONFIG_HOME/ariane/config.toml`, falling back
/// to `~/.config/ariane/config.toml` (PLAN §9).
fn config_file() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_default();
    base.join("ariane").join("config.toml")
}

/// Reads the `repo` key of the configuration file, expanding a leading `~/`.
///
/// A missing file is not an error: it simply yields `None`.
///
/// # Errors
///
/// Fails when the file exists but cannot be read or parsed.
fn configured_repository(file: &Path) -> Result<Option<PathBuf>> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("reading {}", file.display()));
        }
    };
    let config: Config =
        toml::from_str(&text).with_context(|| format!("parsing {}", file.display()))?;
    Ok(config.repo.map(|repo| match repo.strip_prefix("~/") {
        Some(rest) => std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(rest))
            .unwrap_or_else(|| PathBuf::from(&repo)),
        None => PathBuf::from(repo),
    }))
}

/// Tells whether `dir` holds both a `tree/` directory and a `.git` entry.
pub fn is_content_repository(dir: &Path) -> bool {
    dir.join("tree").is_dir() && dir.join(".git").exists()
}

/// Unit tests of repository discovery.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_a_directory_with_tree_and_git() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("tree")).unwrap();
        assert!(!is_content_repository(dir.path()));
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        assert!(is_content_repository(dir.path()));
    }

    #[test]
    fn rejects_an_explicit_path_that_is_not_a_content_repository() {
        let dir = tempfile::tempdir().unwrap();
        assert!(discover(Some(dir.path())).is_err());
    }

    #[test]
    fn the_node_of_a_directory_is_its_path_under_tree() {
        let root = Path::new("/repo");
        assert_eq!(
            node_of(root, Path::new("/repo/tree/cmc/cmc-lib")),
            Some(NodePath::new("cmc/cmc-lib"))
        );
        assert_eq!(
            node_of(root, Path::new("/repo/tree")),
            Some(NodePath::new(""))
        );
        assert_eq!(node_of(root, Path::new("/elsewhere")), None);
    }

    #[test]
    fn the_configured_repository_expands_the_home_directory() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, "repo = \"~/content\"\n").unwrap();
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap();
        assert_eq!(
            configured_repository(&file).unwrap(),
            Some(home.join("content"))
        );
    }

    #[test]
    fn a_missing_configuration_file_configures_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            configured_repository(&dir.path().join("config.toml")).unwrap(),
            None
        );
    }
}

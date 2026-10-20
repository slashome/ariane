//! Finds the Ariane content repository on disk.
//!
//! A content repository is a git repository with a `tree/` directory at its root.
//! It is found, in this order: from `--repo` or the `ARIANE_REPO` environment
//! variable; otherwise from the current directory, first following its `_ariane`
//! link when there is one (a host repository materializes its node through a
//! symlink named `_ariane`, PLAN §4), then walking up the parent directories.
//!
//! Paths are kept as raw [`PathBuf`](std::path::PathBuf) values, never forced into UTF-8 (ADR-0005):
//! a badly named path must still be reportable.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Name of the link through which a host repository materializes its node.
const DIR_NAME: &str = "_ariane";

/// Returns the root of the content repository to work on.
///
/// With an explicit path, that path must itself be a content repository. Without
/// one, the search starts from the current directory, or from the target of its
/// `_ariane` link, and walks up with `Path::ancestors`, which yields the path
/// itself and then each parent in turn.
///
/// # Errors
///
/// Fails when the explicit path cannot be resolved or is not a content
/// repository, or when nothing is found walking up from the current directory.
pub fn discover(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        let root = path
            .canonicalize()
            .with_context(|| format!("reading {}", path.display()))?;
        if is_content_repository(&root) {
            return Ok(root);
        }
        bail!("{} is not an Ariane content repository", root.display());
    }
    let cwd = std::env::current_dir().context("reading the current directory")?;
    let start = match cwd.join(DIR_NAME).canonicalize() {
        Ok(materialized) => materialized,
        Err(_) => cwd.clone(),
    };
    start
        .ancestors()
        .find(|dir| is_content_repository(dir))
        .map(Path::to_path_buf)
        .with_context(|| {
            format!(
                "no Ariane content repository found from {}; pass --repo or set ARIANE_REPO",
                cwd.display()
            )
        })
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
}

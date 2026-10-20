use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

const DIR_NAME: &str = "_ariane";

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

pub fn is_content_repository(dir: &Path) -> bool {
    dir.join("tree").is_dir() && dir.join(".git").exists()
}

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

//! The branches and pull requests an item's work lives in (SPEC.md §1.3).
//!
//! An item records them in two core keys, `branches` (`<owner>/<repo>:<branch>`,
//! since a task may span several repositories) and `pull_requests` (URLs).
//! This module adds entries to those lists without duplicates, keeping every
//! other key of the header as it was, and names a host repository the way
//! `branches` does, from its `origin` remote.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use yaml_serde::{Mapping, Value};

use crate::item::{Document, Header, render};

/// What a piece of work links to: branches and pull requests to record.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Links {
    #[serde(default)]
    pub branches: Vec<String>,
    #[serde(default)]
    pub pull_requests: Vec<String>,
}

impl Links {
    pub fn is_empty(&self) -> bool {
        self.branches.is_empty() && self.pull_requests.is_empty()
    }

    /// Adds `other`'s entries that `self` does not hold yet.
    pub fn merge(&mut self, other: &Links) {
        for (into, from) in [
            (&mut self.branches, &other.branches),
            (&mut self.pull_requests, &other.pull_requests),
        ] {
            for entry in from {
                if !into.contains(entry) {
                    into.push(entry.clone());
                }
            }
        }
    }
}

/// Adds `links` to the item file `file`, entries already there skipped, and
/// returns what was actually added. A list that does not exist yet is
/// appended at the end of the header.
///
/// # Errors
///
/// Fails when the file cannot be read or written, has no readable header, or
/// holds one of the keys as something other than a list.
pub fn record(file: &Path, links: &Links) -> Result<Links> {
    let text =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let document = Document::parse(&text);
    let Header::Present(mut header) = document.header else {
        bail!("{} has no readable front-matter", file.display());
    };
    let added = Links {
        branches: append(&mut header, "branches", &links.branches)?,
        pull_requests: append(&mut header, "pull_requests", &links.pull_requests)?,
    };
    if !added.is_empty() {
        std::fs::write(file, render(&header, &document.body)?)
            .with_context(|| format!("writing {}", file.display()))?;
    }
    Ok(added)
}

/// Appends to the list `key` of `header` the `entries` it lacks; returns them.
fn append(header: &mut Mapping, key: &str, entries: &[String]) -> Result<Vec<String>> {
    let mut list = match header.get(key) {
        None => Vec::new(),
        Some(Value::Sequence(list)) => list.clone(),
        Some(_) => bail!("`{key}` is not a list"),
    };
    let added: Vec<String> = entries
        .iter()
        .filter(|entry| {
            !list
                .iter()
                .any(|present| present.as_str() == Some(entry.as_str()))
        })
        .cloned()
        .collect();
    if !added.is_empty() {
        list.extend(added.iter().cloned().map(Value::String));
        header.insert(Value::String(key.to_owned()), Value::Sequence(list));
    }
    Ok(added)
}

/// `<owner>/<repo>` of the repository at `dir`, read from its `origin` remote
/// (`git@github.com:owner/repo.git`, `https://github.com/owner/repo`); without
/// a remote, `local/<directory name>` of its top level.
pub fn repository(dir: &Path) -> Option<String> {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    if let Some(name) = git(&["remote", "get-url", "origin"]).and_then(|url| owner_repo(&url)) {
        return Some(name);
    }
    let top = git(&["rev-parse", "--show-toplevel"])?;
    let name = Path::new(&top).file_name()?.to_string_lossy().into_owned();
    Some(format!("local/{name}"))
}

/// The last two path segments of a remote URL, `.git` dropped.
fn owner_repo(url: &str) -> Option<String> {
    let path = url.trim_end_matches('/').trim_end_matches(".git");
    let path = path.rsplit_once(':').map_or(
        path,
        |(head, tail)| if head.contains("://") { path } else { tail },
    );
    let mut segments = path.rsplit('/');
    let repo = segments.next().filter(|segment| !segment.is_empty())?;
    let owner = segments
        .next()
        .filter(|segment| !segment.is_empty() && !segment.contains('.'))?;
    Some(format!("{owner}/{repo}"))
}

/// Unit tests of recording links and naming repositories.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_adds_only_new_entries_and_keeps_the_header() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("T1-a.md");
        std::fs::write(
            &file,
            "---\nariane: 1\nid: T1\nmod: Kept\nbranches:\n- o/r:a\n---\n# Body\n",
        )
        .unwrap();
        let links = Links {
            branches: vec!["o/r:a".to_owned(), "o/r:b".to_owned()],
            pull_requests: vec!["https://github.com/o/r/pull/1".to_owned()],
        };
        let added = record(&file, &links).unwrap();
        assert_eq!(added.branches, ["o/r:b"]);
        assert_eq!(added.pull_requests, ["https://github.com/o/r/pull/1"]);
        let document = Document::parse(&std::fs::read_to_string(&file).unwrap());
        assert_eq!(document.texts("branches"), ["o/r:a", "o/r:b"]);
        assert_eq!(document.text("mod"), Some("Kept"));
        assert_eq!(document.body, "# Body\n");
        assert!(record(&file, &links).unwrap().is_empty());
    }

    #[test]
    fn a_remote_url_gives_owner_and_repository() {
        assert_eq!(
            owner_repo("git@github.com:slashome/ariane.git").as_deref(),
            Some("slashome/ariane")
        );
        assert_eq!(
            owner_repo("https://github.com/slashome/_ariane").as_deref(),
            Some("slashome/_ariane")
        );
        assert_eq!(
            owner_repo("ssh://git@host:22/team/app.git").as_deref(),
            Some("team/app")
        );
        assert_eq!(owner_repo("repo"), None);
    }

    #[test]
    fn links_merge_without_duplicates() {
        let mut links = Links {
            branches: vec!["o/r:a".to_owned()],
            pull_requests: vec![],
        };
        links.merge(&Links {
            branches: vec!["o/r:a".to_owned(), "o/r:b".to_owned()],
            pull_requests: vec!["u".to_owned()],
        });
        assert_eq!(links.branches, ["o/r:a", "o/r:b"]);
        assert_eq!(links.pull_requests, ["u"]);
    }
}

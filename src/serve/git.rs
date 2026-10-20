//! The working-tree state of the content repository, read from `git status`.
//!
//! It feeds the header's commit indicator (ADR-0012). The command runs with
//! `--porcelain=v1 -z`, whose records are separated by NUL bytes and whose paths
//! are never quoted, so a path is kept as raw bytes and turned into an
//! [`OsString`] without assuming UTF-8 (ADR-0005).

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// One uncommitted path and its two-letter porcelain code, such as ` M` or `??`.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    /// The `XY` status code: index state then working-tree state.
    pub code: String,
    /// The path relative to the repository root, as git reported it.
    pub path: OsString,
}

/// The current branch and the uncommitted paths.
#[derive(Debug, PartialEq, Eq)]
pub struct Status {
    /// The checked-out branch; `None` on a detached `HEAD`.
    pub branch: Option<String>,
    /// Every path that differs from `HEAD`, untracked files included.
    pub entries: Vec<Entry>,
}

/// Runs `git status --porcelain=v1 -z --branch` in `root` and parses it.
///
/// # Errors
///
/// Fails when git cannot be run or exits with an error.
pub fn status(root: &Path) -> Result<Status> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain=v1", "-z", "--branch"])
        .output()
        .context("running git status")?;
    if !output.status.success() {
        bail!(
            "git status failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(parse(&output.stdout))
}

/// Parses the NUL-separated porcelain output.
///
/// The first record is the `## <branch>...<upstream> [ahead n]` header. A rename
/// or copy (`R` or `C` in either column) is followed by an extra record holding
/// the original path, which is skipped: the indicator shows the new path.
pub fn parse(output: &[u8]) -> Status {
    let mut records = output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    let mut status = Status {
        branch: None,
        entries: Vec::new(),
    };
    while let Some(record) = records.next() {
        if let Some(header) = record.strip_prefix(b"## ") {
            status.branch = branch(&String::from_utf8_lossy(header));
            continue;
        }
        if record.len() < 4 {
            continue;
        }
        let code = String::from_utf8_lossy(&record[..2]).into_owned();
        let path = OsString::from_vec(record[3..].to_vec());
        if code.contains(['R', 'C']) {
            records.next();
        }
        status.entries.push(Entry { code, path });
    }
    status
}

/// Extracts the branch name from the porcelain header.
///
/// `main...origin/main [ahead 1]` gives `main`; `No commits yet on main` gives
/// `main`; a detached `HEAD (no branch)` gives `None`.
fn branch(header: &str) -> Option<String> {
    if header.starts_with("HEAD (no branch)") {
        return None;
    }
    let header = header
        .strip_prefix("No commits yet on ")
        .or_else(|| header.strip_prefix("Initial commit on "))
        .unwrap_or(header);
    let name = header
        .split("...")
        .next()
        .unwrap_or(header)
        .split(' ')
        .next()
        .unwrap_or(header);
    (!name.is_empty()).then(|| name.to_owned())
}

/// Unit tests of the porcelain parser.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_branch_the_entries_and_skips_the_origin_of_a_rename() {
        let output = b"## main...origin/main [ahead 1]\0 M tree/a.md\0R  tree/new.md\0tree/old.md\0?? tree/b c.md\0";
        let status = parse(output);
        assert_eq!(status.branch.as_deref(), Some("main"));
        let entries: Vec<_> = status
            .entries
            .iter()
            .map(|entry| (entry.code.as_str(), entry.path.to_str().unwrap()))
            .collect();
        assert_eq!(
            entries,
            [
                (" M", "tree/a.md"),
                ("R ", "tree/new.md"),
                ("??", "tree/b c.md")
            ]
        );
    }

    #[test]
    fn a_detached_head_has_no_branch_and_a_new_repository_has_one() {
        assert_eq!(parse(b"## HEAD (no branch)\0").branch, None);
        assert_eq!(
            parse(b"## No commits yet on main\0").branch.as_deref(),
            Some("main")
        );
    }

    #[test]
    fn a_path_that_is_not_utf8_is_kept_as_raw_bytes() {
        let status = parse(b"## main\0?? tree/caf\xe9.md\0");
        assert_eq!(
            status.entries[0].path.as_encoded_bytes(),
            b"tree/caf\xe9.md"
        );
    }
}

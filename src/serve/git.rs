//! The working-tree state of the content repository, read from `git status`.
//!
//! It feeds the header's commit indicator (ADR-0012). The command runs with
//! `--porcelain=v1 -z`, whose records are separated by NUL bytes and whose paths
//! are never quoted, so a path is kept as raw bytes and turned into an
//! [`OsString`] without assuming UTF-8 (ADR-0005).
//!
//! It also dates each node by its latest commit, and runs the two git actions
//! the interface offers: commit what changed under `tree/`, and push. Both use
//! the machine's own git identity and configured upstream; nothing here sets
//! an author or a remote.

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

/// The date (`YYYY-MM-DD`) of the latest commit that touched `relative`, a
/// path under the repository root; `None` when git fails or no commit touched it.
pub fn last_change(root: &Path, relative: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "-1", "--format=%cs", "--"])
        .arg(relative)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let date = String::from_utf8(output.stdout).ok()?;
    let date = date.trim();
    (!date.is_empty()).then(|| date.to_owned())
}

/// What a commit request came to.
#[derive(Debug, PartialEq, Eq)]
pub enum Committed {
    /// A commit was made; its abbreviated hash.
    Commit(String),
    /// Nothing under `tree/` differed from `HEAD`.
    NothingToCommit,
}

/// Stages every change under `tree/` and commits it with `message`.
///
/// Staging is limited to `tree/`, the content the interface edits. Whether
/// anything is staged is asked to `git diff --cached --quiet`, which exits `1`
/// when there is something to commit.
///
/// # Errors
///
/// Fails when a git command cannot run or reports an error.
pub fn commit(root: &Path, message: &str) -> Result<Committed> {
    run(root, &["add", "-A", "--", "tree"])?;
    let staged = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["diff", "--cached", "--quiet"])
        .status()
        .context("running git diff")?;
    if staged.success() {
        return Ok(Committed::NothingToCommit);
    }
    run(root, &["commit", "-q", "-m", message])?;
    let hash = run(root, &["rev-parse", "--short", "HEAD"])?;
    Ok(Committed::Commit(hash.trim().to_owned()))
}

/// Pushes the current branch to its configured upstream.
///
/// # Errors
///
/// Fails with git's own message when the push is refused or impossible.
pub fn push(root: &Path) -> Result<()> {
    run(root, &["push", "-q"]).map(|_| ())
}

/// Runs `git -C root <args>` and returns its standard output.
///
/// # Errors
///
/// Fails when git cannot be started, or exits with an error, carrying its stderr.
fn run(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .with_context(|| format!("running git {}", args[0]))?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
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

    /// A real repository with one committed file under `tree/`. Its identity is
    /// set in the repository's own configuration, so [`commit`] runs exactly as
    /// it does in production, and no configuration of the machine is touched.
    fn repository() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q"]);
        git(dir.path(), &["config", "user.name", "test"]);
        git(dir.path(), &["config", "user.email", "test@example.com"]);
        std::fs::create_dir_all(dir.path().join("tree")).unwrap();
        std::fs::write(dir.path().join("tree/a.md"), "a\n").unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-q", "-m", "first"]);
        dir
    }

    /// Runs git in `root` and asserts it succeeded.
    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn a_node_is_dated_by_its_latest_commit() {
        let dir = repository();
        let date = last_change(dir.path(), Path::new("tree")).unwrap();
        assert!(crate::schema::is_date(&date));
        assert_eq!(last_change(dir.path(), Path::new("tree/none")), None);
    }

    #[test]
    fn commits_changes_under_tree_and_reports_when_there_is_nothing_left() {
        let dir = repository();
        std::fs::write(dir.path().join("tree/a.md"), "changed\n").unwrap();
        std::fs::write(dir.path().join("outside.md"), "not staged\n").unwrap();
        let Committed::Commit(hash) = commit(dir.path(), "edit a").unwrap() else {
            panic!("expected a commit");
        };
        assert!(!hash.is_empty());
        assert_eq!(
            commit(dir.path(), "again").unwrap(),
            Committed::NothingToCommit
        );
        let untracked = status(dir.path()).unwrap();
        assert_eq!(untracked.entries.len(), 1);
        assert_eq!(untracked.entries[0].path, OsString::from("outside.md"));
    }

    #[test]
    fn pushing_needs_an_upstream_and_then_sends_the_branch() {
        let dir = repository();
        assert!(push(dir.path()).is_err());
        let remote = tempfile::tempdir().unwrap();
        git(remote.path(), &["init", "-q", "--bare"]);
        git(
            dir.path(),
            &["remote", "add", "origin", remote.path().to_str().unwrap()],
        );
        git(dir.path(), &["push", "-q", "-u", "origin", "HEAD"]);
        std::fs::write(dir.path().join("tree/a.md"), "pushed\n").unwrap();
        commit(dir.path(), "to push").unwrap();
        push(dir.path()).unwrap();
        let local = run(dir.path(), &["rev-parse", "HEAD"]).unwrap();
        let pushed = run(remote.path(), &["rev-parse", "HEAD"]).unwrap();
        assert_eq!(local, pushed);
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

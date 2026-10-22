//! The working-tree state of the content repository, read from `git status`.
//!
//! It feeds the header's commit indicator (ADR-0012). The command runs with
//! `--porcelain=v1 -z`, whose records are separated by NUL bytes and whose paths
//! are never quoted, so a path is kept as raw bytes and turned into an
//! [`OsString`] without assuming UTF-8 (ADR-0005).
//!
//! It also dates each node by its latest commit, and runs the git actions the
//! interface offers: commit what changed under `tree/`, push, discard changes
//! under `tree/`, and show the diff of one file. Commit and push use the
//! machine's own git identity and configured upstream; nothing here sets an
//! author or a remote.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// One uncommitted path and its two-letter porcelain code, such as ` M` or `??`.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    /// The `XY` status code: index state then working-tree state.
    pub code: String,
    /// The path relative to the repository root, as git reported it.
    pub path: OsString,
    /// The original path of a rename or a copy, `None` otherwise.
    pub origin: Option<OsString>,
}

/// The current branch and the uncommitted paths.
#[derive(Debug, PartialEq, Eq)]
pub struct Status {
    /// The checked-out branch; `None` on a detached `HEAD`.
    pub branch: Option<String>,
    /// How many local commits the upstream does not have yet; `0` without upstream.
    pub ahead: u32,
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

/// Checks that `path` names something under `tree/`, relative to the repository
/// root, and returns it as a path.
///
/// Only plain components are accepted: an absolute path, `.` or `..` anywhere
/// is refused, so the result can never leave `tree/`.
///
/// # Errors
///
/// Fails with the reason the path is refused.
pub fn under_tree(path: &str) -> Result<PathBuf> {
    let candidate = Path::new(path);
    if !candidate
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        bail!("{path} is not a plain relative path");
    }
    if candidate.components().next() != Some(Component::Normal(OsStr::new("tree"))) {
        bail!("{path} is not under tree/");
    }
    Ok(candidate.to_path_buf())
}

/// What a discard request came to.
#[derive(Debug, PartialEq, Eq)]
pub enum Discarded {
    /// These paths were restored from `HEAD` or removed.
    Paths(Vec<OsString>),
    /// Nothing under the requested paths differed from `HEAD`.
    NothingToDiscard,
}

/// Throws away the uncommitted changes under `paths`, or under all of `tree/`
/// when `paths` is empty.
///
/// The changed paths are read from `git status` over `tree/`, then kept when
/// they fall under one of `paths` (a status limited to a path would not see the
/// original of a rename, which lies outside it). A
/// tracked one is restored from `HEAD` in both the index and the working tree,
/// an untracked one (`??`) is removed with `git clean`. The original path of a
/// rename is restored too when it is under `tree/`. Pathspecs are literal, so a
/// `*` or a `:` in a name is never read as a pattern.
///
/// # Errors
///
/// Fails when a path is not under `tree/`, or a git command reports an error.
pub fn discard(root: &Path, paths: &[PathBuf]) -> Result<Discarded> {
    let scope = if paths.is_empty() {
        vec![PathBuf::from("tree")]
    } else {
        paths.to_vec()
    };
    for path in &scope {
        under_tree(&path.to_string_lossy())?;
    }
    let output = git_literal(root)
        .args(["status", "--porcelain=v1", "-z", "--", "tree"])
        .output()
        .context("running git status")?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    let mut entries = parse(&output.stdout).entries;
    entries.retain(|entry| {
        scope
            .iter()
            .any(|path| Path::new(&entry.path).starts_with(path))
    });
    if entries.is_empty() {
        return Ok(Discarded::NothingToDiscard);
    }
    let mut tracked = Vec::new();
    let mut untracked = Vec::new();
    for entry in &entries {
        if entry.code == "??" {
            untracked.push(entry.path.clone());
            continue;
        }
        tracked.push(entry.path.clone());
        if let Some(origin) = &entry.origin
            && under_tree(&origin.to_string_lossy()).is_ok()
        {
            tracked.push(origin.clone());
        }
    }
    if !tracked.is_empty() {
        run_literal(
            root,
            &["restore", "--source=HEAD", "--staged", "--worktree", "--"],
            &tracked,
        )?;
    }
    if !untracked.is_empty() {
        run_literal(root, &["clean", "-f", "-d", "-q", "--"], &untracked)?;
    }
    Ok(Discarded::Paths(
        entries.into_iter().map(|entry| entry.path).collect(),
    ))
}

/// The diff of one file under `tree/` against `HEAD`, as unified-diff text.
///
/// A file git knows (in `HEAD` or in the index) is compared with
/// `git diff HEAD`; an untracked file is compared with an empty file through
/// `git diff --no-index`, which exits `1` when the two differ: that is its
/// normal answer, not an error. The text is empty when nothing differs.
///
/// # Errors
///
/// Fails when the path is not under `tree/`, or git reports an error.
pub fn diff(root: &Path, path: &str) -> Result<String> {
    let path = under_tree(path)?;
    let known = git_literal(root)
        .args(["ls-files", "--error-unmatch", "--"])
        .arg(&path)
        .output()
        .context("running git ls-files")?
        .status
        .success()
        || run(
            root,
            &["cat-file", "-e", &format!("HEAD:{}", path.display())],
        )
        .is_ok();
    let mut command = git_literal(root);
    command.args(["diff", "--no-color", "--no-ext-diff"]);
    if known {
        command.args(["HEAD", "--"]).arg(&path);
    } else {
        command.args(["--no-index", "--", "/dev/null"]).arg(&path);
    }
    let output = command.output().context("running git diff")?;
    match output.status.code() {
        Some(0) => {}
        Some(1) if !known => {}
        _ => bail!("{}", String::from_utf8_lossy(&output.stderr).trim()),
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// A `git -C root --literal-pathspecs` command, to which arguments are added.
fn git_literal(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).arg("--literal-pathspecs");
    command
}

/// Runs `git -C root --literal-pathspecs <args> <paths>`, failing with git's
/// stderr when it exits with an error.
fn run_literal(root: &Path, args: &[&str], paths: &[OsString]) -> Result<()> {
    let output = git_literal(root)
        .args(args)
        .args(paths)
        .output()
        .with_context(|| format!("running git {}", args[0]))?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(())
}

/// Parses the NUL-separated porcelain output.
///
/// The first record is the `## <branch>...<upstream> [ahead n]` header, absent
/// when `--branch` is not passed. A rename or copy (`R` or `C` in either column)
/// is followed by an extra record holding the original path, kept as the
/// entry's `origin`: the indicator shows the new path.
pub fn parse(output: &[u8]) -> Status {
    let mut records = output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    let mut status = Status {
        branch: None,
        ahead: 0,
        entries: Vec::new(),
    };
    while let Some(record) = records.next() {
        if let Some(header) = record.strip_prefix(b"## ") {
            let header = String::from_utf8_lossy(header);
            status.branch = branch(&header);
            status.ahead = ahead(&header);
            continue;
        }
        if record.len() < 4 {
            continue;
        }
        let code = String::from_utf8_lossy(&record[..2]).into_owned();
        let path = OsString::from_vec(record[3..].to_vec());
        let origin = if code.contains(['R', 'C']) {
            records
                .next()
                .map(|origin| OsString::from_vec(origin.to_vec()))
        } else {
            None
        };
        status.entries.push(Entry { code, path, origin });
    }
    status
}

/// Reads the `ahead n` count of the porcelain header, `0` when absent.
///
/// `main...origin/main [ahead 2, behind 1]` gives `2`.
fn ahead(header: &str) -> u32 {
    header
        .split_once("[ahead ")
        .and_then(|(_, rest)| rest.split([',', ']']).next())
        .and_then(|count| count.trim().parse().ok())
        .unwrap_or(0)
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
        assert_eq!(status.ahead, 1);
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
    fn the_ahead_count_is_read_with_or_without_behind() {
        assert_eq!(ahead("main...origin/main [ahead 2, behind 1]"), 2);
        assert_eq!(ahead("main...origin/main [behind 3]"), 0);
        assert_eq!(ahead("main"), 0);
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
    fn only_plain_paths_under_tree_are_accepted() {
        assert!(under_tree("tree/a.md").is_ok());
        assert!(under_tree("tree").is_ok());
        for refused in [
            "/etc/passwd",
            "tree/../outside.md",
            "../tree/a.md",
            "./tree/a.md",
            "outside.md",
            "treehouse/a.md",
            "",
        ] {
            assert!(under_tree(refused).is_err(), "{refused} was accepted");
        }
    }

    #[test]
    fn discards_one_path_and_leaves_the_others() {
        let dir = repository();
        std::fs::write(dir.path().join("tree/a.md"), "changed\n").unwrap();
        std::fs::write(dir.path().join("tree/b.md"), "new\n").unwrap();
        let discarded = discard(dir.path(), &[PathBuf::from("tree/a.md")]).unwrap();
        assert_eq!(
            discarded,
            Discarded::Paths(vec![OsString::from("tree/a.md")])
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("tree/a.md")).unwrap(),
            "a\n"
        );
        assert!(dir.path().join("tree/b.md").exists());
    }

    #[test]
    fn discarding_everything_restores_tracked_files_and_removes_untracked_ones_under_tree_only() {
        let dir = repository();
        std::fs::write(dir.path().join("tree/a.md"), "changed\n").unwrap();
        git(dir.path(), &["add", "tree/a.md"]);
        std::fs::create_dir_all(dir.path().join("tree/new")).unwrap();
        std::fs::write(dir.path().join("tree/new/b.md"), "new\n").unwrap();
        std::fs::write(dir.path().join("tree/*.md"), "a pattern name\n").unwrap();
        std::fs::write(dir.path().join("outside.md"), "kept\n").unwrap();
        let Discarded::Paths(paths) = discard(dir.path(), &[]).unwrap() else {
            panic!("expected discarded paths");
        };
        assert_eq!(paths.len(), 3);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("tree/a.md")).unwrap(),
            "a\n"
        );
        assert!(!dir.path().join("tree/new").exists());
        assert!(!dir.path().join("tree/*.md").exists());
        let left = status(dir.path()).unwrap();
        assert_eq!(left.entries.len(), 1);
        assert_eq!(left.entries[0].path, OsString::from("outside.md"));
        assert_eq!(
            discard(dir.path(), &[]).unwrap(),
            Discarded::NothingToDiscard
        );
    }

    #[test]
    fn discarding_a_staged_rename_brings_the_original_back() {
        let dir = repository();
        git(dir.path(), &["mv", "tree/a.md", "tree/renamed.md"]);
        discard(dir.path(), &[PathBuf::from("tree/renamed.md")]).unwrap();
        assert!(dir.path().join("tree/a.md").exists());
        assert!(!dir.path().join("tree/renamed.md").exists());
        assert!(status(dir.path()).unwrap().entries.is_empty());
    }

    #[test]
    fn a_discard_outside_tree_is_refused_before_git_runs() {
        let dir = repository();
        std::fs::write(dir.path().join("outside.md"), "kept\n").unwrap();
        assert!(discard(dir.path(), &[PathBuf::from("tree/../outside.md")]).is_err());
        assert!(dir.path().join("outside.md").exists());
    }

    #[test]
    fn diffs_a_tracked_file_against_head_and_an_untracked_one_against_nothing() {
        let dir = repository();
        std::fs::write(dir.path().join("tree/a.md"), "b\n").unwrap();
        let tracked = diff(dir.path(), "tree/a.md").unwrap();
        assert!(tracked.contains("-a\n+b\n"), "{tracked}");
        std::fs::write(dir.path().join("tree/new.md"), "fresh\n").unwrap();
        let untracked = diff(dir.path(), "tree/new.md").unwrap();
        assert!(untracked.contains("+fresh"), "{untracked}");
        std::fs::remove_file(dir.path().join("tree/a.md")).unwrap();
        assert!(diff(dir.path(), "tree/a.md").unwrap().contains("-a"));
        assert!(diff(dir.path(), "../etc/passwd").is_err());
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

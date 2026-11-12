//! What git remembers of each item file: when it last changed, and the
//! statuses it went through.
//!
//! One `git log --raw` walks the history of `tree/` once. Newest commit first,
//! the first date met for a path is its latest change; a file never committed
//! has no date, and a moved file is dated from its move. Oldest commit first,
//! with renames followed, the `status` of every version of a file gives the
//! statuses it went through and the date of each change: the record S020
//! builds on, from which [`crate::closed::approximate`] approaches the closing
//! date of an item finished before `closed` existed (ADR-0027). The headers of
//! those versions are read in one `git cat-file --batch`.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::item::Document;

/// Separates the date of a commit from the files it touched in the log output.
const DATE_MARK: char = '\u{0}';

/// One status a file took, from the commit that gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    /// The commit date, `YYYY-MM-DD`.
    pub date: String,
    /// The `status` of the file's header from then on; `None` without one.
    pub status: Option<String>,
}

/// What git remembers of the item files, by path relative to the repository
/// root. Empty when the root is not a git repository or git cannot run: dates
/// are a nicety, never a failure.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    /// The date (`YYYY-MM-DD`) of the latest commit touching each file.
    pub updated: HashMap<PathBuf, String>,
    /// The statuses each file went through, oldest first, renames followed,
    /// one entry per change of status.
    pub statuses: HashMap<PathBuf, Vec<Transition>>,
}

impl History {
    /// Reads the history of `tree/` under `root`.
    pub fn read(root: &Path) -> History {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args([
                "-c",
                "core.quotePath=false",
                "log",
                "-M",
                "--raw",
                "--no-abbrev",
                "--format=%x00%cs",
                "--",
                "tree",
            ])
            .output();
        let commits = match output {
            Ok(output) if output.status.success() => {
                parse(&String::from_utf8_lossy(&output.stdout))
            }
            _ => return History::default(),
        };
        let blobs: HashSet<&str> = commits
            .iter()
            .flat_map(|commit| &commit.changes)
            .filter(|change| change.kind != 'D')
            .map(|change| change.blob.as_str())
            .collect();
        let statuses = statuses_of(root, &blobs);
        History::from_commits(&commits, &statuses)
    }

    /// The statuses the file at `path` went through, oldest first; empty when
    /// git knows nothing of it.
    pub fn transitions(&self, path: &Path) -> &[Transition] {
        self.statuses.get(path).map_or(&[], Vec::as_slice)
    }

    /// Builds the history from `commits`, newest first, and the `status` of
    /// each blob they name.
    fn from_commits(commits: &[Commit], statuses: &HashMap<String, Option<String>>) -> History {
        let mut history = History::default();
        for commit in commits {
            for change in surviving(commit) {
                history
                    .updated
                    .entry(change.path.clone())
                    .or_insert_with(|| commit.date.clone());
            }
        }
        for commit in commits.iter().rev() {
            for change in &commit.changes {
                let mut timeline = match (change.kind, &change.from) {
                    ('D', _) => {
                        history.statuses.remove(&change.path);
                        continue;
                    }
                    ('R', Some(from)) => history.statuses.remove(from).unwrap_or_default(),
                    ('C', Some(from)) => history.statuses.get(from).cloned().unwrap_or_default(),
                    _ => history.statuses.remove(&change.path).unwrap_or_default(),
                };
                let status = statuses.get(&change.blob).cloned().flatten();
                if timeline.last().is_none_or(|last| last.status != status) {
                    timeline.push(Transition {
                        date: commit.date.clone(),
                        status,
                    });
                }
                history.statuses.insert(change.path.clone(), timeline);
            }
        }
        history
    }
}

/// The changes of `commit` that leave a file behind.
fn surviving(commit: &Commit) -> impl Iterator<Item = &Change> {
    commit.changes.iter().filter(|change| change.kind != 'D')
}

/// One commit of the log: its date and the files it changed.
#[derive(Debug)]
struct Commit {
    /// The commit date, `YYYY-MM-DD`.
    date: String,
    /// The files it changed, in log order.
    changes: Vec<Change>,
}

/// One file changed by a commit, as a `--raw` line gives it.
#[derive(Debug)]
struct Change {
    /// `A`, `M`, `D`, `R`, `C` or `T`, without the similarity score.
    kind: char,
    /// The former path of a renamed or copied file.
    from: Option<PathBuf>,
    /// The path after the commit.
    path: PathBuf,
    /// The object id of the file after the commit.
    blob: String,
}

/// Reads the output of `git log --raw --no-abbrev --format=%x00%cs`: a line
/// that starts with the mark opens a commit, each `:` line is one file it
/// changed.
fn parse(log: &str) -> Vec<Commit> {
    let mut commits: Vec<Commit> = Vec::new();
    for line in log.lines() {
        if let Some(date) = line.strip_prefix(DATE_MARK) {
            commits.push(Commit {
                date: date.to_owned(),
                changes: Vec::new(),
            });
        } else if let (Some(raw), Some(commit)) = (line.strip_prefix(':'), commits.last_mut())
            && let Some(change) = change(raw)
        {
            commit.changes.push(change);
        }
    }
    commits
}

/// Reads one `--raw` line without its leading `:`:
/// `<mode> <mode> <blob> <blob> <kind>\t<path>[\t<path>]`.
fn change(raw: &str) -> Option<Change> {
    let (meta, paths) = raw.split_once('\t')?;
    let fields: Vec<&str> = meta.split_whitespace().collect();
    let [_, _, _, blob, kind] = fields.as_slice() else {
        return None;
    };
    let kind = kind.chars().next()?;
    let (from, path) = match paths.split_once('\t') {
        Some((from, to)) => (Some(PathBuf::from(from)), PathBuf::from(to)),
        None => (None, PathBuf::from(paths)),
    };
    Some(Change {
        kind,
        from,
        path,
        blob: (*blob).to_owned(),
    })
}

/// The `status` of the header of each blob, read in one `git cat-file --batch`;
/// empty when git cannot run.
fn statuses_of(root: &Path, blobs: &HashSet<&str>) -> HashMap<String, Option<String>> {
    let Ok(mut child) = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return HashMap::new();
    };
    let request: String = blobs.iter().map(|blob| format!("{blob}\n")).collect();
    let Some(mut stdin) = child.stdin.take() else {
        return HashMap::new();
    };
    let writer = std::thread::spawn(move || stdin.write_all(request.as_bytes()));
    let statuses = child
        .stdout
        .take()
        .map(|stdout| read_batch(BufReader::new(stdout)))
        .unwrap_or_default();
    let _ = writer.join();
    let _ = child.wait();
    statuses
}

/// Reads the answers of `git cat-file --batch`: an `<id> <type> <size>` line,
/// then that many bytes and a newline; a missing object takes one line.
fn read_batch(mut reader: impl BufRead) -> HashMap<String, Option<String>> {
    let mut statuses = HashMap::new();
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok_and(|read| read > 0) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if let [id, _, size] = fields.as_slice()
            && let Ok(size) = size.parse::<usize>()
        {
            let mut content = vec![0; size + 1];
            if reader.read_exact(&mut content).is_err() {
                break;
            }
            let text = String::from_utf8_lossy(&content[..size]);
            let status = Document::parse(&text).text("status").map(str::to_owned);
            statuses.insert((*id).to_owned(), status);
        }
        line.clear();
    }
    statuses
}

/// Unit tests of the log parsing and of the walk over a real repository.
#[cfg(test)]
mod tests {
    use super::*;

    fn transition(date: &str, status: &str) -> Transition {
        Transition {
            date: date.to_owned(),
            status: Some(status.to_owned()),
        }
    }

    #[test]
    fn the_newest_commit_dates_a_file_and_a_rename_carries_its_statuses() {
        let log = "\u{0}2026-10-21\n\n:100644 100644 b1 b3 R090\ttree/p/tasks/T1-a.md\ttree/p/archives/tasks/T1-a.md\n\u{0}2026-10-05\n\n:100644 100644 b0 b1 M\ttree/p/tasks/T1-a.md\n:000000 100644 0000 b2 A\ttree/p/tasks/T2-b.md\n\u{0}2026-10-01\n\n:000000 100644 0000 b0 A\ttree/p/tasks/T1-a.md\n";
        let statuses = HashMap::from([
            ("b0".to_owned(), Some("todo".to_owned())),
            ("b1".to_owned(), Some("done".to_owned())),
            ("b2".to_owned(), None),
            ("b3".to_owned(), Some("done".to_owned())),
        ]);
        let history = History::from_commits(&parse(log), &statuses);
        let archived = Path::new("tree/p/archives/tasks/T1-a.md");
        assert_eq!(history.updated[archived], "2026-10-21");
        assert_eq!(
            history.updated[Path::new("tree/p/tasks/T2-b.md")],
            "2026-10-05"
        );
        assert_eq!(
            history.transitions(archived),
            [
                transition("2026-10-01", "todo"),
                transition("2026-10-05", "done")
            ]
        );
        assert!(
            history
                .transitions(Path::new("tree/p/tasks/T1-a.md"))
                .is_empty()
        );
    }

    #[test]
    fn a_directory_that_is_not_a_repository_has_no_history() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(History::read(dir.path()), History::default());
    }

    #[test]
    fn a_real_repository_gives_each_version_its_status_through_a_move() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let git = |args: &[&str]| {
            let status = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(args)
                .env("GIT_AUTHOR_NAME", "test")
                .env("GIT_AUTHOR_EMAIL", "test@example.com")
                .env("GIT_COMMITTER_NAME", "test")
                .env("GIT_COMMITTER_EMAIL", "test@example.com")
                .stdout(Stdio::null())
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?}");
        };
        let task = |status: &str| {
            format!(
                "---\nariane: 2\nnode: p\nid: T1\ntitle: A\nstatus: {status}\n---\n# A\n\nA body long enough for git to follow the file through its move.\n"
            )
        };
        git(&["init", "-q"]);
        std::fs::create_dir_all(root.join("tree/p/tasks")).unwrap();
        std::fs::create_dir_all(root.join("tree/p/archives/tasks")).unwrap();
        std::fs::write(root.join("tree/p/tasks/T1-a.md"), task("todo")).unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "one"]);
        std::fs::write(root.join("tree/p/tasks/T1-a.md"), task("done")).unwrap();
        git(&["commit", "-qam", "two"]);
        git(&[
            "mv",
            "tree/p/tasks/T1-a.md",
            "tree/p/archives/tasks/T1-a.md",
        ]);
        git(&["commit", "-qm", "three"]);
        let history = History::read(root);
        let archived = Path::new("tree/p/archives/tasks/T1-a.md");
        let statuses: Vec<Option<&str>> = history
            .transitions(archived)
            .iter()
            .map(|transition| transition.status.as_deref())
            .collect();
        assert_eq!(statuses, [Some("todo"), Some("done")]);
        assert!(history.updated.contains_key(archived));
    }
}

//! When each item file last changed, according to git.
//!
//! One `git log` walks the history of `tree/` once, newest commit first, and
//! the first date met for a path is its latest change. A file never committed
//! has no date. Renames are not followed: a moved file is dated from its move.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Separates the date of a commit from the paths it touched in the log output.
const DATE_MARK: char = '\u{0}';

/// The date (`YYYY-MM-DD`) of the latest commit touching each file under
/// `tree/`, by path relative to `root`. Empty when `root` is not a git
/// repository or git cannot run: dates are a nicety, never a failure.
pub fn file_dates(root: &Path) -> HashMap<PathBuf, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "core.quotePath=false",
            "log",
            "--format=%x00%cs",
            "--name-only",
            "--",
            "tree",
        ])
        .output();
    match output {
        Ok(output) if output.status.success() => parse(&String::from_utf8_lossy(&output.stdout)),
        _ => HashMap::new(),
    }
}

/// Reads the output of `git log --format=%x00%cs --name-only`: a line that
/// starts with the mark gives the date of the paths listed after it.
fn parse(log: &str) -> HashMap<PathBuf, String> {
    let mut dates = HashMap::new();
    let mut current = "";
    for line in log.lines() {
        if let Some(date) = line.strip_prefix(DATE_MARK) {
            current = date;
        } else if !line.is_empty() && !current.is_empty() {
            dates
                .entry(PathBuf::from(line))
                .or_insert_with(|| current.to_owned());
        }
    }
    dates
}

/// Unit tests of the log parsing.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_commit_dates_a_file() {
        let log = "\u{0}2026-10-21\n\ntree/p/tasks/T1-a.md\n\u{0}2026-10-01\n\ntree/p/tasks/T1-a.md\ntree/p/tasks/T2-b.md\n";
        let dates = parse(log);
        assert_eq!(dates[Path::new("tree/p/tasks/T1-a.md")], "2026-10-21");
        assert_eq!(dates[Path::new("tree/p/tasks/T2-b.md")], "2026-10-01");
    }

    #[test]
    fn a_directory_that_is_not_a_repository_has_no_dates() {
        let dir = tempfile::tempdir().unwrap();
        assert!(file_dates(dir.path()).is_empty());
    }
}

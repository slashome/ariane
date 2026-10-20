//! `ariane migrate`: brings item files to the current schema version (ADR-0009).
//!
//! Each item's version is read from its `ariane` key (absent means version 0).
//! Migrations are applied one version at a time, `N` to `N + 1`, until the file
//! reaches [`SCHEMA_VERSION`](crate::item::SCHEMA_VERSION); each migration is idempotent, so a migrated file
//! is left unchanged by a second run.
//!
//! By default nothing is written: the command prints a unified diff per file and
//! a summary. `--apply` writes the files and is the only write path of the
//! command; it never commits, so the changes stay in the working tree for review.
//! A file newer than this binary is reported and never touched. The exit code is
//! `2` when at least one file needs a human, `0` otherwise.

/// Migration 0 → 1: from the legacy prose headers and pharmakon's French keys to the version-1 header.
mod v1;

use std::process::ExitCode;

use anyhow::{Context, Result};
use serde::Serialize;
use similar::TextDiff;

use crate::cli::MigrateArgs;
use crate::item::{Document, Header, SCHEMA_VERSION};
use crate::tree::{Item, NodePath, Tree};

/// What one migration produced for one file.
#[derive(Debug)]
pub struct Step {
    /// The whole new text of the file.
    pub text: String,
    /// Keys whose value was guessed rather than read, for the reviewer.
    pub inferred: Vec<String>,
    /// Remarks about what was done, such as a header read tolerantly.
    pub notes: Vec<String>,
}

/// One migration from version `source()` to the next one.
///
/// A trait is Rust's interface: the engine only knows this contract, so adding
/// the migration 1 → 2 means writing one more type that implements it.
pub trait Migration {
    /// The version this migration upgrades from.
    fn source(&self) -> u64;
    /// Upgrades `text`, the current text of `item`, by one version. Returns the
    /// reasons a human must step in instead, as `Err`, when it cannot decide.
    fn apply(&self, item: &Item, text: &str) -> Result<Step, Vec<String>>;
}

/// What happens to one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The file changes (or would, in a dry run).
    Migrated,
    /// The file is already current.
    Unchanged,
    /// The file cannot be migrated without a decision.
    NeedsHuman,
}

/// The plan for one file, serialized by `--json`. The `#[serde(skip)]` fields
/// are working data that the JSON output leaves out.
#[derive(Debug, Serialize)]
pub struct Entry {
    /// Path relative to the repository root.
    pub path: String,
    /// Version found in the file; `None` when it cannot be read.
    pub from: Option<u64>,
    /// Version the file is brought to.
    pub to: u64,
    /// What happens to the file.
    pub outcome: Outcome,
    /// Why a human is needed, when that is the outcome.
    pub reasons: Vec<String>,
    /// Keys guessed by the migrations.
    pub inferred: Vec<String>,
    /// Remarks of the migrations.
    pub notes: Vec<String>,
    /// Text of the file before migration.
    #[serde(skip)]
    before: String,
    /// Text after migration, when it changes.
    #[serde(skip)]
    after: Option<String>,
    /// Absolute path, to write the file with `--apply`.
    #[serde(skip)]
    file: std::path::PathBuf,
}

/// Runs `ariane migrate`: plans every file in scope, writes the migrated ones
/// with `--apply`, then prints the diffs or the JSON plan.
///
/// # Errors
///
/// Fails when a file cannot be read or written, or when the plan cannot be
/// serialized to JSON.
pub fn run(tree: &Tree, args: &MigrateArgs) -> Result<ExitCode> {
    let scope = args.node.as_deref().map(NodePath::new);
    let entries = plan(tree, scope.as_ref())?;
    if args.apply {
        for entry in entries
            .iter()
            .filter(|entry| entry.outcome == Outcome::Migrated)
        {
            let after = entry.after.as_deref().unwrap_or_default();
            std::fs::write(&entry.file, after)
                .with_context(|| format!("writing {}", entry.file.display()))?;
        }
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        report(&entries, args.apply);
    }
    let blocked = entries
        .iter()
        .any(|entry| entry.outcome == Outcome::NeedsHuman);
    Ok(if blocked {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

/// Plans the migration of every item in `scope` without writing anything.
///
/// The migrations are held as `&dyn Migration`, trait objects: the engine calls
/// them through the trait without knowing their concrete type. Collecting an
/// iterator of `Result` into a `Result<Vec<_>>` stops at the first error.
///
/// # Errors
///
/// Fails when an item file cannot be read.
pub fn plan(tree: &Tree, scope: Option<&NodePath>) -> Result<Vec<Entry>> {
    let zero_to_one = v1::ZeroToOne::new(tree, scope);
    let migrations: [&dyn Migration; 1] = [&zero_to_one];
    tree.items_within(scope)
        .map(|item| {
            let before = std::fs::read_to_string(&item.file)
                .with_context(|| format!("reading {}", item.file.display()))?;
            Ok(migrate(&migrations, item, before))
        })
        .collect()
}

/// Plans one file: reads its version, then chains the migrations from that
/// version to the current one, each taking the previous one's output.
fn migrate(migrations: &[&dyn Migration], item: &Item, before: String) -> Entry {
    let mut entry = Entry {
        path: item.relative.to_string_lossy().into_owned(),
        from: None,
        to: SCHEMA_VERSION,
        outcome: Outcome::Unchanged,
        reasons: Vec::new(),
        inferred: Vec::new(),
        notes: Vec::new(),
        after: None,
        file: item.file.clone(),
        before,
    };
    let version = match version_of(&entry.before) {
        Ok(version) => version,
        Err(reason) => return entry.needs_human(vec![reason]),
    };
    entry.from = Some(version);
    if version > SCHEMA_VERSION {
        return entry.needs_human(vec![format!(
            "the file is at version {version}, newer than this ariane ({SCHEMA_VERSION}); upgrade ariane"
        )]);
    }
    let mut text = entry.before.clone();
    for current in version..SCHEMA_VERSION {
        let Some(migration) = migrations
            .iter()
            .find(|migration| migration.source() == current)
        else {
            return entry.needs_human(vec![format!("no migration from version {current}")]);
        };
        match migration.apply(item, &text) {
            Ok(step) => {
                text = step.text;
                entry.inferred.extend(step.inferred);
                entry.notes.extend(step.notes);
            }
            Err(reasons) => return entry.needs_human(reasons),
        }
    }
    if text != entry.before {
        entry.outcome = Outcome::Migrated;
        entry.after = Some(text);
    }
    entry
}

/// Builders of an entry's outcome.
impl Entry {
    /// Marks the entry as needing a human for `reasons`. `mut self` takes the entry by value, changes it and hands it back.
    fn needs_human(mut self, reasons: Vec<String>) -> Entry {
        self.outcome = Outcome::NeedsHuman;
        self.reasons = reasons;
        self.inferred.clear();
        self
    }
}

/// The schema version of a file's text: `0` without front-matter or without an
/// `ariane` key. A front-matter that strict YAML rejects is still version 0 when
/// the tolerant line-by-line reader of [`v1::flat_front_matter`] can read it.
/// Returns the reason as `Err` when the version cannot be told.
fn version_of(text: &str) -> Result<u64, String> {
    let document = Document::parse(text);
    match &document.header {
        Header::Missing => Ok(0),
        Header::Malformed(error) => match v1::flat_front_matter(text) {
            Some(mapping) if !mapping.contains_key("ariane") => Ok(0),
            _ => Err(format!("the front-matter is not valid YAML: {error}")),
        },
        Header::Present(mapping) => match mapping.get("ariane") {
            None => Ok(0),
            Some(value) => value
                .as_u64()
                .ok_or_else(|| format!("`ariane` is not a version number: {value:?}")),
        },
    }
}

/// Prints a unified diff per migrated file (or `wrote <path>` after `--apply`), the reasons of the files that need a human, and a summary.
fn report(entries: &[Entry], applied: bool) {
    for entry in entries {
        match (entry.outcome, &entry.after) {
            (Outcome::Migrated, Some(after)) if !applied => {
                let diff = TextDiff::from_lines(entry.before.as_str(), after.as_str());
                print!(
                    "{}",
                    diff.unified_diff()
                        .context_radius(2)
                        .header(&entry.path, &entry.path)
                );
                if !entry.inferred.is_empty() {
                    println!("  inferred: {}", entry.inferred.join(", "));
                }
            }
            (Outcome::Migrated, _) => println!("wrote {}", entry.path),
            (Outcome::NeedsHuman, _) => {
                println!("needs a human: {}", entry.path);
                for reason in &entry.reasons {
                    println!("  - {reason}");
                }
            }
            (Outcome::Unchanged, _) => {}
        }
    }
    let count = |outcome| {
        entries
            .iter()
            .filter(|entry| entry.outcome == outcome)
            .count()
    };
    let verb = if applied { "migrated" } else { "to migrate" };
    println!(
        "\n{} {verb}, {} unchanged, {} need a human",
        count(Outcome::Migrated),
        count(Outcome::Unchanged),
        count(Outcome::NeedsHuman)
    );
}

/// Unit tests of the engine: dry run, apply, idempotence, newer files, scope.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// The arguments of `ariane migrate`, with or without `--apply`.
    fn args(apply: bool) -> MigrateArgs {
        MigrateArgs {
            node: None,
            apply,
            json: false,
        }
    }

    /// A repository with a few legacy items to migrate.
    fn sample() -> Repo {
        let repo = Repo::new();
        repo.write(
            "game/tasks/T001-forge.md",
            "---\nnode: game\nid: T001\ntitre: Forger\nstatut: todo\nmod: Smithing\n---\n# T001 — Forger\n",
        );
        repo
    }

    #[test]
    fn a_dry_run_writes_nothing() {
        let repo = sample();
        let path = repo.root().join("tree/game/tasks/T001-forge.md");
        let before = std::fs::read_to_string(&path).unwrap();
        let tree = Tree::load(repo.root()).unwrap();
        let code = run(&tree, &args(false)).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn applying_writes_and_a_second_run_changes_nothing() {
        let repo = sample();
        let path = repo.root().join("tree/game/tasks/T001-forge.md");
        run(&Tree::load(repo.root()).unwrap(), &args(true)).unwrap();
        let migrated = std::fs::read_to_string(&path).unwrap();
        assert!(migrated.starts_with("---\nariane: 1\nnode: game\nid: T001\ntitle: Forger\nstatus: todo\nmod: Smithing\n---\n"));
        let entries = plan(&Tree::load(repo.root()).unwrap(), None).unwrap();
        assert_eq!(entries[0].outcome, Outcome::Unchanged);
        run(&Tree::load(repo.root()).unwrap(), &args(true)).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), migrated);
    }

    #[test]
    fn a_file_newer_than_the_binary_is_reported_and_never_touched() {
        let repo = Repo::new();
        let text = "---\nariane: 99\nid: T001\n---\n# X\n";
        repo.write("game/tasks/T001-x.md", text);
        let tree = Tree::load(repo.root()).unwrap();
        let entries = plan(&tree, None).unwrap();
        assert_eq!(entries[0].outcome, Outcome::NeedsHuman);
        assert!(entries[0].reasons[0].contains("newer"));
        let code = run(&tree, &args(true)).unwrap();
        assert_eq!(code, ExitCode::from(2));
        let path = repo.root().join("tree/game/tasks/T001-x.md");
        assert_eq!(std::fs::read_to_string(path).unwrap(), text);
    }

    #[test]
    fn the_scope_limits_the_files_considered() {
        let repo = sample();
        repo.write(
            "other/tasks/T001-x.md",
            "---\nid: T001\ntitre: X\nstatut: todo\n---\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        let entries = plan(&tree, Some(&NodePath::new("other"))).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].path.contains("other"));
    }
}

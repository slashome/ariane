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
//!
//! Since migration 1 → 2 turns milestones into releases (ADR-0024), the report
//! ends with the distinct releases the items of each migrated node carry, those
//! its `node.toml` does not declare marked, so that the owner writes the
//! `[[release]]` list by hand: `node.toml` has no version, no migration writes it.

/// Migration 0 → 1: from the legacy prose headers and pharmakon's French keys to the version-1 header.
mod v1;
/// Migration 1 → 2: `milestone` leaves the header and becomes the item's release.
mod v2;

use std::collections::{BTreeMap, BTreeSet};
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
    /// The node holding the item.
    pub node: NodePath,
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
    /// The item's `release` once migrated; `None` without one, or when the file
    /// needs a human.
    pub release: Option<String>,
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
pub fn run(tree: &Tree, args: &MigrateArgs, scope: Option<&NodePath>) -> Result<ExitCode> {
    let entries = plan(tree, scope)?;
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
        report_releases(tree, &entries);
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
    let migrations: [&dyn Migration; 2] = [&zero_to_one, &v2::OneToTwo];
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
        node: item.node.clone(),
        from: None,
        to: SCHEMA_VERSION,
        outcome: Outcome::Unchanged,
        reasons: Vec::new(),
        inferred: Vec::new(),
        notes: Vec::new(),
        release: None,
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
    entry.release = Document::parse(&text).text("release").map(str::to_owned);
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

/// Prints, for every node with a migrated file, the distinct releases its
/// items carry once migrated, in natural order, marking those the node's
/// effective `[[release]]` list does not declare.
fn report_releases(tree: &Tree, entries: &[Entry]) {
    for (node, names) in releases_by_node(entries) {
        let effective = tree.configs.effective(node);
        let listed: Vec<String> = names
            .iter()
            .map(|name| match effective.release(name) {
                Some(_) => (*name).to_owned(),
                None => format!("{name} (undeclared)"),
            })
            .collect();
        let label = if node.is_home() {
            "(home)"
        } else {
            node.as_str()
        };
        println!("releases of {label}: {}", listed.join(", "));
    }
}

/// The distinct releases the items of each node carry once migrated, in
/// natural order, for the nodes where at least one file is migrated.
fn releases_by_node(entries: &[Entry]) -> BTreeMap<&NodePath, Vec<&str>> {
    let mut by_node: BTreeMap<&NodePath, BTreeSet<&str>> = BTreeMap::new();
    for entry in entries {
        if entry.outcome == Outcome::Migrated {
            by_node.entry(&entry.node).or_default();
        }
    }
    for entry in entries {
        if let (Some(releases), Some(release)) =
            (by_node.get_mut(&entry.node), entry.release.as_deref())
        {
            releases.insert(release);
        }
    }
    by_node
        .into_iter()
        .filter(|(_, releases)| !releases.is_empty())
        .map(|(node, releases)| {
            let mut names: Vec<&str> = releases.into_iter().collect();
            names.sort_by_key(|name| natural(name));
            (node, names)
        })
        .collect()
}

/// A sort key that orders release names naturally: dot-separated numbers by
/// value (`0.9` before `0.10`), then the names that are not numbers.
fn natural(name: &str) -> (bool, Vec<u64>, &str) {
    let numbers: Option<Vec<u64>> = name.split('.').map(|part| part.parse().ok()).collect();
    match numbers {
        Some(numbers) => (false, numbers, name),
        None => (true, Vec::new(), name),
    }
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
            all: false,
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
        let code = run(&tree, &args(false), None).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn applying_writes_and_a_second_run_changes_nothing() {
        let repo = sample();
        let path = repo.root().join("tree/game/tasks/T001-forge.md");
        run(&Tree::load(repo.root()).unwrap(), &args(true), None).unwrap();
        let migrated = std::fs::read_to_string(&path).unwrap();
        assert!(migrated.starts_with("---\nariane: 2\nnode: game\nid: T001\ntitle: Forger\nstatus: todo\nmod: Smithing\n---\n"));
        let entries = plan(&Tree::load(repo.root()).unwrap(), None).unwrap();
        assert_eq!(entries[0].outcome, Outcome::Unchanged);
        run(&Tree::load(repo.root()).unwrap(), &args(true), None).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), migrated);
    }

    #[test]
    fn a_french_jalon_goes_from_version_0_to_its_release_in_one_run() {
        let repo = Repo::new();
        repo.write(
            "game/tasks/T001-forge.md",
            "---\nnode: game\nid: T001\ntitre: Forger\nstatut: todo\nrelease: 1.0\njalon: 0.10\nmod: Smithing\n---\n# T001\n",
        );
        let entries = plan(&Tree::load(repo.root()).unwrap(), None).unwrap();
        assert_eq!(entries[0].from, Some(0));
        assert_eq!(
            entries[0].after.as_deref(),
            Some(
                "---\nariane: 2\nnode: game\nid: T001\ntitle: Forger\nstatus: todo\nrelease: '0.10'\nmod: Smithing\n---\n# T001\n"
            )
        );
        assert_eq!(entries[0].release.as_deref(), Some("0.10"));
    }

    #[test]
    fn the_releases_of_every_migrated_node_are_listed_in_natural_order() {
        let repo = Repo::new();
        let item = |id: &str, release: &str, milestone: &str| {
            format!(
                "---\nariane: 1\nnode: game\nid: {id}\ntitle: {id}\nstatus: todo\nrelease: '{release}'\n{milestone}---\n"
            )
        };
        repo.write(
            "game/tasks/T001-a.md",
            &item("T001", "1.0", "milestone: '0.10'\n"),
        )
        .write(
            "game/tasks/T002-b.md",
            &item("T002", "1.0", "milestone: '0.9'\n"),
        )
        .write(
            "game/archives/tasks/T003-c.md",
            &item("T003", "plus-tard", ""),
        )
        .write("game/stories/S001-d.md", &item("S001", "2.0", ""))
        .write(
            "other/tasks/T001-e.md",
            "---\nariane: 2\nnode: other\nid: T001\ntitle: E\nstatus: todo\nrelease: '9.0'\n---\n",
        );
        let entries = plan(&Tree::load(repo.root()).unwrap(), None).unwrap();
        let releases = releases_by_node(&entries);
        assert_eq!(releases.len(), 1);
        assert_eq!(
            releases[&NodePath::new("game")],
            ["0.9", "0.10", "2.0", "plus-tard"]
        );
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
        let code = run(&tree, &args(true), None).unwrap();
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

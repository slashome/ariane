mod v1;

use std::process::ExitCode;

use anyhow::{Context, Result};
use serde::Serialize;
use similar::TextDiff;

use crate::cli::MigrateArgs;
use crate::item::{Document, Header, SCHEMA_VERSION};
use crate::tree::{Item, NodePath, Tree};

#[derive(Debug)]
pub struct Step {
    pub text: String,
    pub inferred: Vec<String>,
    pub notes: Vec<String>,
}

pub trait Migration {
    fn source(&self) -> u64;
    fn apply(&self, item: &Item, text: &str) -> Result<Step, Vec<String>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Migrated,
    Unchanged,
    NeedsHuman,
}

#[derive(Debug, Serialize)]
pub struct Entry {
    pub path: String,
    pub from: Option<u64>,
    pub to: u64,
    pub outcome: Outcome,
    pub reasons: Vec<String>,
    pub inferred: Vec<String>,
    pub notes: Vec<String>,
    #[serde(skip)]
    before: String,
    #[serde(skip)]
    after: Option<String>,
    #[serde(skip)]
    file: std::path::PathBuf,
}

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

impl Entry {
    fn needs_human(mut self, reasons: Vec<String>) -> Entry {
        self.outcome = Outcome::NeedsHuman;
        self.reasons = reasons;
        self.inferred.clear();
        self
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    fn args(apply: bool) -> MigrateArgs {
        MigrateArgs {
            node: None,
            apply,
            json: false,
        }
    }

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

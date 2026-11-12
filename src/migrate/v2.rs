//! Migration 1 → 2: `milestone` leaves the header and becomes the item's release (ADR-0024).
//!
//! On an item carrying `milestone`, `release` takes its value and `milestone`
//! is removed (`release: '1.0'` + `milestone: '0.3'` → `release: '0.3'`); an
//! item with `milestone` alone gains `release`. A release written as a YAML
//! number or boolean is rewritten as the quoted text it was written as, since
//! a float loses `1.10` to `1.1`. Every other byte of the file stays as it was:
//! the lines are edited in place, never re-rendered, so unknown keys, their
//! order and quoting, and the body are kept byte for byte. An item without
//! `milestone` only has its version raised.

use yaml_serde::{Mapping, Value};

use crate::item::{self, Document, Header};
use crate::tree::Item;

use super::{Migration, Step};

/// The key this migration removes.
const MILESTONE: &str = "milestone";
/// The key that takes its value.
const RELEASE: &str = "release";

/// The migration from version 1 to version 2. It needs nothing from the tree:
/// each file is migrated on its own text.
pub struct OneToTwo;

/// The migration as seen by the engine.
impl Migration for OneToTwo {
    /// Version 1, the first versioned header.
    fn source(&self) -> u64 {
        1
    }

    /// Raises `ariane` to 2, moves `milestone` into `release` and quotes a
    /// numeric release, editing those lines only.
    ///
    /// Returns `Err` when the front-matter cannot be edited line by line, when
    /// `release` or `milestone` is not a scalar, or when the edited header would
    /// read differently from the intended one.
    fn apply(&self, _item: &Item, text: &str) -> Result<Step, Vec<String>> {
        let document = Document::parse(text);
        let Header::Present(mapping) = &document.header else {
            return Err(vec!["the front-matter cannot be read as YAML".to_owned()]);
        };
        let raw = raw_front_matter(text);
        let release = scalar(mapping, raw, RELEASE)?;
        let milestone = scalar(mapping, raw, MILESTONE)?;
        let numeric = mapping
            .get(RELEASE)
            .is_some_and(|value| value.is_number() || value.is_bool());
        let target = milestone.clone().or_else(|| release.clone());
        let mut notes = Vec::new();
        if let (Some(old), Some(new)) = (&release, &milestone)
            && old != new
        {
            notes.push(format!("release `{old}` replaced by milestone `{new}`"));
        }
        let mut edited = edit(text, "ariane", Some("2"))?;
        if milestone.is_some() || numeric {
            let line = target.as_deref().map(quoted).transpose()?;
            edited = edit(&edited, RELEASE, line.as_deref())?;
        }
        if mapping.contains_key(MILESTONE) {
            edited = edit(&edited, MILESTONE, None)?;
        }
        verify(&edited, mapping, target.as_deref(), &document.body)?;
        Ok(Step {
            text: edited,
            inferred: Vec::new(),
            notes,
        })
    }
}

/// The value of `key` as text: a string as is, a number or a boolean as the
/// text it was written as. `None` when the key is absent or null.
///
/// Returns `Err` when the value is a list or a mapping, which no release can be.
fn scalar(mapping: &Mapping, raw: Option<&str>, key: &str) -> Result<Option<String>, Vec<String>> {
    match mapping.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(value @ (Value::Number(_) | Value::Bool(_))) => Ok(Some(
            raw.and_then(|raw| raw_scalar(raw, key))
                .map_or_else(|| display(value), str::to_owned),
        )),
        Some(_) => Err(vec![format!(
            "`{key}` is not a single value; set the item's release by hand"
        )]),
    }
}

/// The YAML text of `name` as a string scalar, quoted when YAML would read it
/// as anything else: `0.3` gives `'0.3'`, `plus-tard` stays as is.
fn quoted(name: &str) -> Result<String, Vec<String>> {
    yaml_serde::to_string(&Value::from(name))
        .map(|yaml| yaml.trim_end().to_owned())
        .map_err(|error| vec![error.to_string()])
}

/// Sets or removes one top-level scalar line of the front-matter.
fn edit(text: &str, key: &str, value: Option<&str>) -> Result<String, Vec<String>> {
    item::set_scalar_line(text, key, value).ok_or_else(|| {
        vec!["the front-matter is not opened and closed by plain `---` lines".to_owned()]
    })
}

/// Checks that the edited text reads as intended: version 2, `release` set to
/// `target` when there is one, no `milestone`, every other key unchanged and the body untouched.
fn verify(
    edited: &str,
    before: &Mapping,
    target: Option<&str>,
    body: &str,
) -> Result<(), Vec<String>> {
    let document = Document::parse(edited);
    let mut expected = before.clone();
    expected.insert(Value::from("ariane"), Value::from(2_u64));
    expected.shift_remove(MILESTONE);
    if let Some(name) = target {
        expected.insert(Value::from(RELEASE), Value::from(name));
    }
    let same_keys = document.mapping().is_some_and(|after| {
        after.len() == expected.len()
            && expected
                .iter()
                .all(|(key, value)| after.get(key) == Some(value))
    });
    if same_keys && document.body == body {
        Ok(())
    } else {
        Err(vec![format!(
            "`{RELEASE}` or `{MILESTONE}` spans several lines; move the milestone into `release` by hand"
        )])
    }
}

/// The raw text between the opening and closing `---`, unparsed.
fn raw_front_matter(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// The raw text of `key`'s value in the front-matter, when it is written on its
/// own line, without a trailing `# comment`.
fn raw_scalar<'a>(front_matter: &'a str, key: &str) -> Option<&'a str> {
    front_matter.lines().find_map(|line| {
        let value = line.strip_prefix(key)?.strip_prefix(':')?;
        let value = value.split(" #").next().unwrap_or(value).trim();
        (!value.is_empty()).then_some(value)
    })
}

/// A value as YAML text, for a number or a boolean whose line was not found.
fn display(value: &Value) -> String {
    yaml_serde::to_string(value)
        .map(|text| text.trim_end().to_owned())
        .unwrap_or_default()
}

/// Unit tests of migration 1 → 2.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Tree;
    use crate::tree::fixture::Repo;

    /// Runs migration 1 → 2 on `text`, as the file `relative` of node `game`.
    fn migrate(relative: &str, text: &str) -> Result<Step, Vec<String>> {
        let repo = Repo::new();
        repo.write(&format!("game/{relative}"), text);
        let tree = Tree::load(repo.root()).unwrap();
        let item = tree
            .items
            .iter()
            .find(|item| item.relative.ends_with(relative))
            .unwrap();
        OneToTwo.apply(item, text)
    }

    #[test]
    fn the_milestone_becomes_the_release_and_every_other_byte_stays() {
        let before = "---\nariane: 1\nnode: game\nid: T004\ntitle: 'Forger : une lame'\nstatus: to-do\ndepends_on:\n- T002\nrelease: '1.0'\nmilestone: '0.3'\nrefs:\n- DR 0002\nmod:   Smithing\ntests: [blade_forged]\n---\n\n# T004 — Forger\n\nBody  with  spaces\n";
        let step = migrate("tasks/T004-forge.md", before).unwrap();
        assert_eq!(
            step.text,
            "---\nariane: 2\nnode: game\nid: T004\ntitle: 'Forger : une lame'\nstatus: to-do\ndepends_on:\n- T002\nrelease: '0.3'\nrefs:\n- DR 0002\nmod:   Smithing\ntests: [blade_forged]\n---\n\n# T004 — Forger\n\nBody  with  spaces\n"
        );
        assert_eq!(step.notes, ["release `1.0` replaced by milestone `0.3`"]);
    }

    #[test]
    fn a_milestone_alone_gives_the_item_its_release() {
        let step = migrate(
            "stories/S001-a.md",
            "---\nariane: 1\nnode: game\nid: S001\ntitle: A\nstatus: todo\ncreated: 2026-01-02\nmilestone: plus-tard\nmod: M\n---\n# A\n",
        )
        .unwrap();
        assert_eq!(
            step.text,
            "---\nariane: 2\nnode: game\nid: S001\ntitle: A\nstatus: todo\ncreated: 2026-01-02\nrelease: plus-tard\nmod: M\n---\n# A\n"
        );
    }

    #[test]
    fn numbers_are_quoted_as_they_were_written() {
        let step = migrate(
            "archives/tasks/T001-a.md",
            "---\nariane: 1\nnode: game\nid: T001\ntitle: A\nstatus: done\nrelease: 1.10\n---\n",
        )
        .unwrap();
        assert!(step.text.contains("\nrelease: '1.10'\n"));
        let step = migrate(
            "tasks/T002-b.md",
            "---\nariane: 1\nnode: game\nid: T002\ntitle: B\nstatus: todo\nrelease: 1.0\nmilestone: 0.9\n---\n",
        )
        .unwrap();
        assert!(step.text.contains("\nrelease: '0.9'\n---\n"));
        assert!(!step.text.contains(MILESTONE));
    }

    #[test]
    fn an_item_without_milestone_only_has_its_version_raised() {
        let before = "---\nariane: 1\nnode: game\nid: R001\ntitle: R\nstatus: open\nrelease: '2.0'\n---\nR\n";
        let step = migrate("reflexions/R001-r.md", before).unwrap();
        assert_eq!(step.text, before.replacen("ariane: 1", "ariane: 2", 1));
        let ressource = "---\nariane: 1\nnode: game\nid: D001\ntitle: D\n---\n";
        let step = migrate("ressources/D001-d.md", ressource).unwrap();
        assert_eq!(step.text, ressource.replacen("ariane: 1", "ariane: 2", 1));
    }

    #[test]
    fn an_empty_milestone_is_dropped_and_the_release_kept() {
        let step = migrate(
            "tasks/T001-a.md",
            "---\nariane: 1\nnode: game\nid: T001\ntitle: A\nstatus: todo\nrelease: '1.0'\nmilestone:\n---\n",
        )
        .unwrap();
        assert!(step.text.ends_with("\nrelease: '1.0'\n---\n"));
    }

    #[test]
    fn a_milestone_that_is_not_a_single_value_needs_a_human() {
        let reasons = migrate(
            "tasks/T001-a.md",
            "---\nariane: 1\nnode: game\nid: T001\ntitle: A\nstatus: todo\nmilestone: [0.1, 0.2]\n---\n",
        )
        .unwrap_err();
        assert!(reasons[0].contains("not a single value"));
    }

    #[test]
    fn a_milestone_written_over_several_lines_needs_a_human() {
        let reasons = migrate(
            "tasks/T001-a.md",
            "---\nariane: 1\nnode: game\nid: T001\ntitle: A\nstatus: todo\nmilestone: >-\n  0.3\n---\n",
        )
        .unwrap_err();
        assert!(reasons[0].contains("several lines"));
    }
}

//! Edits of item headers requested by the interface (ADR-0012, "Writing").
//!
//! The interface may change four keys of an item — `status`, `size`, `impact`,
//! `priority` — and nothing else. Every change carries the value the browser
//! last saw (`from`) next to the value it wants (`to`): when the file on disk no
//! longer holds `from`, the whole edit is refused with the current values, and
//! nothing is merged. Values are checked against the schema vocabularies and,
//! for `status`, against the node's effective workflow (SPEC §1.5). The file is
//! rewritten through [`item::render`], the writer the CLI uses: unknown and
//! custom keys keep their place, the body is kept byte for byte, a new core key
//! lands at its canonical position (SPEC §1.3).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value as Json;
use yaml_serde::{Mapping, Value};

use crate::item::{self, Document, Header, Kind};
use crate::schema::{self, LEVELS, REFLEXION_STATUSES, SIZES};
use crate::tree::{Item, Tree};

/// The keys the interface is allowed to change.
pub const EDITABLE: [&str; 4] = ["status", "size", "impact", "priority"];

/// One requested change: the value the client saw and the value it wants.
/// `null` on either side means "absent".
#[derive(Debug, Deserialize)]
pub struct Change {
    /// The value the client believes the file holds.
    pub from: Json,
    /// The value to write; `null` removes the key.
    pub to: Json,
}

/// The body of `PATCH /api/items`.
#[derive(Debug, Deserialize)]
pub struct Patch {
    /// Repository-relative path of the item, as `/api/items` gives it.
    pub path: String,
    /// The changes, keyed by header key.
    pub changes: BTreeMap<String, Change>,
}

/// Why an edit was refused, each variant mapping to one HTTP status.
#[derive(Debug, PartialEq)]
pub enum Refusal {
    /// The request names no item, or a key that cannot be edited: `400`.
    BadRequest(String),
    /// The file changed since the client read it: `409`, with the current
    /// value of every requested key.
    Conflict(BTreeMap<String, Json>),
    /// The requested value is not allowed: `422`.
    Invalid(String),
    /// The file could not be read or written: `500`.
    Failed(String),
}

/// Applies `patch` to the item file it names and returns that file's path.
///
/// The file is read again from disk rather than trusted from `tree`, so the
/// concurrency check compares against what is really there. `tree` only says
/// which files are items, their kind and their node's workflow.
///
/// # Errors
///
/// Returns a [`Refusal`] and writes nothing when the item is unknown, a key is
/// not editable, a `from` is stale, a `to` is invalid, or the file cannot be
/// read or rewritten.
pub fn apply(tree: &Tree, patch: &Patch) -> Result<std::path::PathBuf, Refusal> {
    let item = find(tree, &patch.path)?;
    if patch.changes.is_empty() {
        return Err(Refusal::BadRequest("no change requested".to_owned()));
    }
    if let Some(key) = patch
        .changes
        .keys()
        .find(|key| !EDITABLE.contains(&key.as_str()))
    {
        return Err(Refusal::BadRequest(format!(
            "`{key}` cannot be edited from the interface; editable keys are {}",
            EDITABLE.join(", ")
        )));
    }
    let text = std::fs::read_to_string(&item.file)
        .map_err(|error| Refusal::Failed(format!("reading {}: {error}", patch.path)))?;
    let document = Document::parse(&text);
    let Header::Present(mut header) = document.header else {
        return Err(Refusal::Invalid(
            "the item has no versioned header; run `ariane migrate` first".to_owned(),
        ));
    };
    check_fresh(&header, &patch.changes)?;
    for (key, change) in &patch.changes {
        let wanted = wanted(key, &change.to)?;
        validate(tree, item, key, wanted)?;
        set(&mut header, key, wanted);
    }
    let rewritten = item::render(&header, &document.body)
        .map_err(|error| Refusal::Failed(format!("rendering {}: {error}", patch.path)))?;
    std::fs::write(&item.file, rewritten)
        .map_err(|error| Refusal::Failed(format!("writing {}: {error}", patch.path)))?;
    Ok(item.file.clone())
}

/// Finds the item whose repository-relative path is `path`.
///
/// Only files the tree already lists as items qualify, which rules out any path
/// outside `tree/`, a `..` escape, or a non-item file such as `INDEX.md`.
fn find<'a>(tree: &'a Tree, path: &str) -> Result<&'a Item, Refusal> {
    tree.items
        .iter()
        .find(|item| item.relative == Path::new(path))
        .ok_or_else(|| Refusal::BadRequest(format!("`{path}` is not an item of the tree")))
}

/// Refuses the whole edit when any requested key no longer holds its `from`.
fn check_fresh(header: &Mapping, changes: &BTreeMap<String, Change>) -> Result<(), Refusal> {
    let current: BTreeMap<String, Json> = changes
        .keys()
        .map(|key| (key.clone(), current(header, key)))
        .collect();
    let stale = changes
        .iter()
        .any(|(key, change)| current[key] != change.from);
    if stale {
        Err(Refusal::Conflict(current))
    } else {
        Ok(())
    }
}

/// The value a header key holds, as JSON: a string, or `null` when absent.
///
/// A non-string value (which `doctor` would already flag) is shown as its YAML
/// text, so that the client can still send it back as `from`.
fn current(header: &Mapping, key: &str) -> Json {
    match header.get(key) {
        None | Some(Value::Null) => Json::Null,
        Some(Value::String(text)) => Json::String(text.clone()),
        Some(other) => Json::String(
            yaml_serde::to_string(other)
                .unwrap_or_default()
                .trim()
                .to_owned(),
        ),
    }
}

/// Reads a `to` value: a string, or `None` for `null`; anything else is refused.
fn wanted<'a>(key: &str, to: &'a Json) -> Result<Option<&'a str>, Refusal> {
    match to {
        Json::Null => Ok(None),
        Json::String(text) => Ok(Some(text.as_str())),
        _ => Err(Refusal::Invalid(format!(
            "`{key}` must be a string or null"
        ))),
    }
}

/// Checks a wanted value against the schema and, for `status`, the workflow.
///
/// A status is required on tasks, stories, backlog items and reflexions, so it
/// cannot be removed there; a ressource carries none (SPEC §1.3).
fn validate(tree: &Tree, item: &Item, key: &str, wanted: Option<&str>) -> Result<(), Refusal> {
    let allowed: Vec<String> = match key {
        "size" => SIZES.iter().map(|value| (*value).to_owned()).collect(),
        "impact" | "priority" => LEVELS.iter().map(|value| (*value).to_owned()).collect(),
        _ => return validate_status(tree, item, wanted),
    };
    match wanted {
        Some(value) if !allowed.iter().any(|allowed| allowed == value) => Err(Refusal::Invalid(
            format!("`{key}: {value}` is outside {}", allowed.join(", ")),
        )),
        _ => Ok(()),
    }
}

/// Checks a wanted `status` for the item's kind and node.
fn validate_status(tree: &Tree, item: &Item, wanted: Option<&str>) -> Result<(), Refusal> {
    let requirement = schema::core_key("status").map(|key| key.requirement_for(item.kind));
    let Some(status) = wanted else {
        return match requirement {
            Some(schema::Requirement::Must) => Err(Refusal::Invalid(
                "a status is required on this kind of item".to_owned(),
            )),
            _ => Ok(()),
        };
    };
    if requirement == Some(schema::Requirement::MustNot) {
        return Err(Refusal::Invalid("a ressource carries no status".to_owned()));
    }
    let allowed: Vec<String> = if item.kind == Kind::Reflexion {
        REFLEXION_STATUSES
            .iter()
            .map(|value| (*value).to_owned())
            .collect()
    } else {
        tree.configs
            .effective(&item.node)
            .workflow
            .into_iter()
            .map(|status| status.id)
            .collect()
    };
    if allowed.iter().any(|allowed| allowed == status) {
        Ok(())
    } else {
        Err(Refusal::Invalid(format!(
            "`status: {status}` is not in the workflow of `{}` ({})",
            item.node,
            allowed.join(", ")
        )))
    }
}

/// Writes `wanted` under `key`, or removes the key when `wanted` is `None`.
///
/// An existing key keeps its place. A new key is inserted right after the last
/// core key that precedes it in the canonical order, so custom keys stay where
/// they were. `Mapping` keeps insertion order, so the insertion rebuilds it.
fn set(header: &mut Mapping, key: &str, wanted: Option<&str>) {
    let Some(value) = wanted else {
        header.shift_remove(key);
        return;
    };
    let value = Value::String(value.to_owned());
    if let Some(slot) = header.get_mut(key) {
        *slot = value;
        return;
    }
    let rank = |name: &str| schema::CORE_KEYS.iter().position(|core| core.name == name);
    let Some(own) = rank(key) else {
        header.insert(Value::String(key.to_owned()), value);
        return;
    };
    let anchor = header
        .iter()
        .enumerate()
        .filter(|(_, (name, _))| {
            name.as_str()
                .and_then(rank)
                .is_some_and(|position| position < own)
        })
        .map(|(index, _)| index)
        .last();
    let insert_at = anchor.map_or(0, |index| index + 1);
    let mut rebuilt = Mapping::new();
    let mut pending = Some(value);
    for (index, (name, existing)) in std::mem::take(header).into_iter().enumerate() {
        if index == insert_at
            && let Some(value) = pending.take()
        {
            rebuilt.insert(Value::String(key.to_owned()), value);
        }
        rebuilt.insert(name, existing);
    }
    if let Some(value) = pending {
        rebuilt.insert(Value::String(key.to_owned()), value);
    }
    *header = rebuilt;
}

/// Unit tests of header edits, against fixture repositories.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;
    use serde_json::json;

    /// A node with a workflow, a task with a custom key, a reflexion, a ressource.
    fn sample() -> Repo {
        let repo = Repo::new();
        repo.write(
            "p/node.toml",
            "[[workflow]]\nid = \"a-faire\"\nlabel = \"À faire\"\ncategory = \"todo\"\n\n[[workflow]]\nid = \"valide\"\nlabel = \"Validé\"\ncategory = \"done\"\n",
        )
        .write(
            "p/tasks/T001-a.md",
            "---\nariane: 1\nnode: p\nid: T001\ntitle: A\nstatus: a-faire\nrelease: '1.0'\nmod: Forge\n---\n# A\n\nBody  with  spaces\n",
        )
        .write(
            "p/reflexions/R001-r.md",
            "---\nariane: 1\nnode: p\nid: R001\ntitle: R\nstatus: open\n---\n# R\n",
        )
        .write(
            "p/ressources/D001-d.md",
            "---\nariane: 1\nnode: p\nid: D001\ntitle: D\n---\n# D\n",
        );
        repo
    }

    /// Builds a patch from a path and `(key, from, to)` triples.
    fn patch(path: &str, changes: &[(&str, Json, Json)]) -> Patch {
        Patch {
            path: path.to_owned(),
            changes: changes
                .iter()
                .map(|(key, from, to)| {
                    (
                        (*key).to_owned(),
                        Change {
                            from: from.clone(),
                            to: to.clone(),
                        },
                    )
                })
                .collect(),
        }
    }

    /// Reads a file of the fixture tree.
    fn read(repo: &Repo, relative: &str) -> String {
        std::fs::read_to_string(repo.root().join("tree").join(relative)).unwrap()
    }

    #[test]
    fn changes_a_status_and_inserts_new_keys_in_canonical_order_keeping_custom_keys_and_body() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        apply(
            &tree,
            &patch(
                "tree/p/tasks/T001-a.md",
                &[
                    ("status", json!("a-faire"), json!("valide")),
                    ("size", Json::Null, json!("S")),
                    ("priority", Json::Null, json!("high")),
                ],
            ),
        )
        .unwrap();
        assert_eq!(
            read(&repo, "p/tasks/T001-a.md"),
            "---\nariane: 1\nnode: p\nid: T001\ntitle: A\nstatus: valide\nsize: S\npriority: high\nrelease: '1.0'\nmod: Forge\n---\n# A\n\nBody  with  spaces\n"
        );
    }

    #[test]
    fn a_null_to_removes_the_key_and_keeps_the_order_of_the_others() {
        let repo = sample();
        repo.write(
            "p/tasks/T002-b.md",
            "---\nariane: 1\nnode: p\nid: T002\ntitle: B\nstatus: a-faire\nimpact: low\npriority: high\ncreated: 2026-01-02\nmod: X\n---\n# B\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        apply(
            &tree,
            &patch(
                "tree/p/tasks/T002-b.md",
                &[("impact", json!("low"), Json::Null)],
            ),
        )
        .unwrap();
        assert_eq!(
            read(&repo, "p/tasks/T002-b.md"),
            "---\nariane: 1\nnode: p\nid: T002\ntitle: B\nstatus: a-faire\npriority: high\ncreated: 2026-01-02\nmod: X\n---\n# B\n"
        );
    }

    #[test]
    fn a_stale_from_is_a_conflict_and_writes_nothing() {
        let repo = sample();
        let before = read(&repo, "p/tasks/T001-a.md");
        let tree = Tree::load(repo.root()).unwrap();
        let refusal = apply(
            &tree,
            &patch(
                "tree/p/tasks/T001-a.md",
                &[
                    ("status", json!("valide"), json!("a-faire")),
                    ("size", Json::Null, json!("S")),
                ],
            ),
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            Refusal::Conflict(BTreeMap::from([
                ("size".to_owned(), Json::Null),
                ("status".to_owned(), json!("a-faire")),
            ]))
        );
        assert_eq!(read(&repo, "p/tasks/T001-a.md"), before);
    }

    #[test]
    fn values_outside_the_vocabulary_or_the_workflow_are_invalid() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let invalid = |path: &str, key: &str, from: Json, to: Json| {
            matches!(
                apply(&tree, &patch(path, &[(key, from, to)])),
                Err(Refusal::Invalid(_))
            )
        };
        assert!(invalid(
            "tree/p/tasks/T001-a.md",
            "size",
            Json::Null,
            json!("XXL")
        ));
        assert!(invalid(
            "tree/p/tasks/T001-a.md",
            "status",
            json!("a-faire"),
            json!("todo")
        ));
        assert!(invalid(
            "tree/p/tasks/T001-a.md",
            "status",
            json!("a-faire"),
            Json::Null
        ));
        assert!(invalid(
            "tree/p/reflexions/R001-r.md",
            "status",
            json!("open"),
            json!("valide")
        ));
        assert!(invalid(
            "tree/p/ressources/D001-d.md",
            "status",
            Json::Null,
            json!("a-faire")
        ));
        assert!(!invalid(
            "tree/p/reflexions/R001-r.md",
            "status",
            json!("open"),
            json!("trial")
        ));
    }

    #[test]
    fn a_path_that_is_not_an_item_or_a_key_that_is_not_editable_is_a_bad_request() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let bad = |path: &str, key: &str| {
            matches!(
                apply(&tree, &patch(path, &[(key, Json::Null, json!("x"))])),
                Err(Refusal::BadRequest(_))
            )
        };
        assert!(bad("tree/../Cargo.toml", "size"));
        assert!(bad("/etc/passwd", "size"));
        assert!(bad("tree/INDEX.md", "size"));
        assert!(bad("tree/p/tasks/T001-a.md", "title"));
    }
}

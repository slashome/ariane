//! The directory of the personas a story is written for (`[personas."<id>"]`
//! of `node.toml`, ADR-0018), as the interface edits it.
//!
//! Personas are declared on a node and inherited by its sub-nodes, like
//! people ([`crate::people`]). This module lists them with the items that
//! name each, and changes them while keeping items true to them: a renamed id
//! is rewritten in the `personas` of every item that names it; a persona still
//! named is not removed — the items are listed instead. A name changes no
//! item. `node.toml` is edited in place, comments and order kept.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use toml_edit::{DocumentMut, Item as TomlItem, Table, value};
use yaml_serde::Value;

use crate::item::{Document, Header, render};
use crate::node::FILE as NODE_FILE;
use crate::tree::{Item, NodePath, Tree};

/// One persona as listed, and the items written for it.
#[derive(Debug, Serialize)]
pub struct Entry {
    pub id: String,
    pub name: String,
    /// References of the items that name this persona.
    pub used_by: Vec<String>,
}

/// A node that declares a tracker or personas, with the personas it declares.
#[derive(Debug, Serialize)]
pub struct Directory {
    pub node: NodePath,
    pub personas: Vec<Entry>,
}

/// Why a change was refused.
#[derive(Debug, PartialEq)]
pub enum Refusal {
    /// The request is malformed: a bad id, an unknown node or persona.
    Invalid(String),
    /// The persona is still named by these items.
    Used(Vec<String>),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Invalid(message) => f.write_str(message),
            Refusal::Used(items) => write!(
                f,
                "still named by {}: take it out of their `personas` first",
                items.join(", ")
            ),
        }
    }
}

/// Whether `id` reads `<group>:<role>`, each part matching `^[a-z][a-z0-9-]*$`.
pub fn is_id(id: &str) -> bool {
    id.split_once(':').is_some_and(|(group, role)| {
        crate::people::is_handle(group) && crate::people::is_handle(role)
    })
}

/// The items within `node` that name `id` as declared at `node`: a sub-node
/// that redeclares the id has its own persona.
fn named_by<'a>(tree: &'a Tree, node: &'a NodePath, id: &str) -> Vec<&'a Item> {
    tree.items_within(Some(node))
        .filter(|item| item.document.texts("personas").contains(&id))
        .filter(|item| declarer(tree, &item.node, id).as_ref() == Some(node))
        .collect()
}

/// The nearest node, from `node` up, that declares `id`.
fn declarer(tree: &Tree, node: &NodePath, id: &str) -> Option<NodePath> {
    node.ancestors_from_root()
        .into_iter()
        .rev()
        .find(|ancestor| {
            tree.configs
                .files
                .get(ancestor)
                .is_some_and(|file| file.personas.contains_key(id))
        })
}

/// The nodes that declare a tracker, or personas, with their personas.
pub fn list(tree: &Tree) -> Vec<Directory> {
    tree.configs
        .files
        .iter()
        .filter(|(_, file)| file.tracker.is_some() || !file.personas.is_empty())
        .map(|(node, file)| Directory {
            node: node.clone(),
            personas: file
                .personas
                .iter()
                .map(|(id, persona)| Entry {
                    id: id.clone(),
                    name: persona.name.clone(),
                    used_by: named_by(tree, node, id)
                        .into_iter()
                        .filter_map(Item::reference)
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

/// The `node.toml` of `node`.
fn node_file(tree: &Tree, node: &NodePath) -> PathBuf {
    tree.node_dir(node).join(NODE_FILE)
}

fn personas_table(document: &mut DocumentMut) -> &mut Table {
    if !document.contains_table("personas") {
        let mut table = Table::new();
        table.set_implicit(true);
        document.insert("personas", TomlItem::Table(table));
    }
    document["personas"]
        .as_table_mut()
        .expect("personas is a table")
}

fn bad_id(id: &str) -> Refusal {
    Refusal::Invalid(format!(
        "`{id}` is not a persona id: `<group>:<role>`, each in lower case letters, digits and `-`"
    ))
}

/// Declares a persona at `node`.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses a bad id, an
/// empty name, an unknown node, or an id already declared there.
pub fn add(tree: &Tree, node: &NodePath, id: &str, name: &str) -> Result<Result<(), Refusal>> {
    if !is_id(id) {
        return Ok(Err(bad_id(id)));
    }
    if name.trim().is_empty() {
        return Ok(Err(Refusal::Invalid("a persona needs a name".to_owned())));
    }
    if !tree.node_dir(node).is_dir() {
        return Ok(Err(Refusal::Invalid(format!("`{node}` is not a node"))));
    }
    crate::node::edit(&node_file(tree, node), |document| {
        let personas = personas_table(document);
        if personas.contains_key(id) {
            return Err(Refusal::Invalid(format!(
                "`{id}` is already declared in `{node}`"
            )));
        }
        let mut persona = Table::new();
        persona.insert("name", value(name.trim()));
        personas.insert(id, TomlItem::Table(persona));
        Ok(())
    })
}

/// Changes the name of a persona declared at `node`.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses an unknown
/// persona or an empty name.
pub fn set_name(tree: &Tree, node: &NodePath, id: &str, name: &str) -> Result<Result<(), Refusal>> {
    if name.trim().is_empty() {
        return Ok(Err(Refusal::Invalid("a persona needs a name".to_owned())));
    }
    crate::node::edit(&node_file(tree, node), |document| {
        let Some(persona) = personas_table(document)
            .get_mut(id)
            .and_then(TomlItem::as_table_like_mut)
        else {
            return Err(Refusal::Invalid(format!("no `{id}` declared in `{node}`")));
        };
        persona.insert("name", value(name.trim()));
        Ok(())
    })
}

/// Renames an id declared at `node`, in `node.toml` and in the `personas` of
/// every item that names it; returns the items rewritten.
///
/// # Errors
///
/// Fails when a file cannot be read or written; refuses a bad or taken id, or
/// an unknown persona.
pub fn rename(
    tree: &Tree,
    node: &NodePath,
    old: &str,
    new: &str,
) -> Result<Result<Vec<String>, Refusal>> {
    if !is_id(new) {
        return Ok(Err(bad_id(new)));
    }
    let items = named_by(tree, node, old);
    let renamed = crate::node::edit(&node_file(tree, node), |document| {
        let personas = personas_table(document);
        if personas.contains_key(new) {
            return Err(Refusal::Invalid(format!(
                "`{new}` is already declared in `{node}`"
            )));
        }
        let Some(persona) = personas.remove(old) else {
            return Err(Refusal::Invalid(format!("no `{old}` declared in `{node}`")));
        };
        personas.insert(new, persona);
        Ok(())
    })?;
    if let Err(refusal) = renamed {
        return Ok(Err(refusal));
    }
    let mut rewritten = Vec::new();
    for item in items {
        replace_id(&item.file, old, new)?;
        rewritten.extend(item.reference());
    }
    Ok(Ok(rewritten))
}

/// Replaces `old` by `new` in the `personas` of the item file `file`.
fn replace_id(file: &Path, old: &str, new: &str) -> Result<()> {
    let text =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let document = Document::parse(&text);
    let Header::Present(mut header) = document.header else {
        return Ok(());
    };
    if let Some(Value::Sequence(list)) = header.get_mut("personas") {
        for entry in list.iter_mut() {
            if entry.as_str() == Some(old) {
                *entry = Value::String(new.to_owned());
            }
        }
    }
    std::fs::write(file, render(&header, &document.body)?)
        .with_context(|| format!("writing {}", file.display()))
}

/// Removes a persona declared at `node`, unless items still name it.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses an unknown or
/// still named persona.
pub fn remove(tree: &Tree, node: &NodePath, id: &str) -> Result<Result<(), Refusal>> {
    let used: Vec<String> = named_by(tree, node, id)
        .into_iter()
        .filter_map(Item::reference)
        .collect();
    if !used.is_empty() {
        return Ok(Err(Refusal::Used(used)));
    }
    crate::node::edit(&node_file(tree, node), |document| {
        if personas_table(document).remove(id).is_none() {
            return Err(Refusal::Invalid(format!("no `{id}` declared in `{node}`")));
        }
        Ok(())
    })
}

/// Unit tests of the directory and of its changes.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    fn sample() -> Repo {
        let repo = Repo::new();
        repo.write(
            "cmc/node.toml",
            "# the team\n[tracker]\nkind = \"monday\"\naccount = \"acme\"\nboard = 1\n\n[personas.\"user:rh\"]\nname = \"RH\" # the HR\n",
        )
        .write("cmc/lib/stories/S1-a.md", "---\nariane: 1\nnode: cmc/lib\nid: S1\ntitle: A\nstatus: todo\npersonas: [\"user:rh\"]\n---\n# A\n")
        .write("cmc/lib/stories/S2-b.md", "---\nariane: 1\nnode: cmc/lib\nid: S2\ntitle: B\nstatus: todo\n---\n");
        repo
    }

    #[test]
    fn the_directory_lists_the_personas_and_the_items_written_for_them() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let directories = list(&tree);
        assert_eq!(directories.len(), 1);
        assert_eq!(directories[0].personas[0].id, "user:rh");
        assert_eq!(directories[0].personas[0].name, "RH");
        assert_eq!(directories[0].personas[0].used_by, ["cmc/lib#S1"]);
    }

    #[test]
    fn adding_and_naming_keep_the_file_and_check_their_input() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let cmc = NodePath::new("cmc");
        add(&tree, &cmc, "cmc:dev", "Développeur").unwrap().unwrap();
        assert!(matches!(
            add(&tree, &cmc, "cmc:dev", "D").unwrap(),
            Err(Refusal::Invalid(_))
        ));
        assert!(matches!(
            add(&tree, &cmc, "dev", "D").unwrap(),
            Err(Refusal::Invalid(_))
        ));
        set_name(&tree, &cmc, "user:rh", "Responsable RH")
            .unwrap()
            .unwrap();
        let text = std::fs::read_to_string(repo.root().join("tree/cmc/node.toml")).unwrap();
        assert!(text.starts_with("# the team\n"), "{text}");
        let tree = Tree::load(repo.root()).unwrap();
        let personas = &tree.configs.effective(&cmc).personas;
        assert_eq!(personas["cmc:dev"].name, "Développeur");
        assert_eq!(personas["user:rh"].name, "Responsable RH");
    }

    #[test]
    fn renaming_rewrites_the_items_and_removing_refuses_while_named() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let cmc = NodePath::new("cmc");
        assert_eq!(
            remove(&tree, &cmc, "user:rh").unwrap(),
            Err(Refusal::Used(vec!["cmc/lib#S1".to_owned()]))
        );
        assert_eq!(
            rename(&tree, &cmc, "user:rh", "user:hr").unwrap().unwrap(),
            ["cmc/lib#S1"]
        );
        let story =
            std::fs::read_to_string(repo.root().join("tree/cmc/lib/stories/S1-a.md")).unwrap();
        assert!(story.contains("user:hr"), "{story}");
        assert!(!story.contains("user:rh"), "{story}");
        let tree = Tree::load(repo.root()).unwrap();
        let errors = crate::doctor::diagnose(&tree, None, false)
            .into_iter()
            .filter(|finding| serde_json::to_value(finding.severity).unwrap() == "error")
            .count();
        assert_eq!(errors, 0);
        repo.write(
            "cmc/lib/stories/S1-a.md",
            "---\nariane: 1\nnode: cmc/lib\nid: S1\ntitle: A\nstatus: todo\n---\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        remove(&tree, &cmc, "user:hr").unwrap().unwrap();
        let tree = Tree::load(repo.root()).unwrap();
        assert!(tree.configs.effective(&cmc).personas.is_empty());
    }
}

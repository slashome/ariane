//! `ariane people`: the directory of the people an item may wait for
//! (`[people.<handle>]` of `node.toml`, ADR-0015).
//!
//! People are declared where a node binds a tracker, and inherited by its
//! sub-nodes. This module lists them with how many items wait for each, and
//! changes them while keeping items true to them: a renamed handle is
//! rewritten in the `waiting_for` of every item that names it; a person still
//! awaited is not removed — the items are listed instead, for the owner to
//! settle. A name or a tracker id changes no item. `node.toml` is edited in
//! place, comments and order kept.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use toml_edit::{DocumentMut, Item as TomlItem, Table, value};
use yaml_serde::Value;

use crate::cli::{PeopleArgs, PeopleCommand};
use crate::item::{Document, Header, render};
use crate::node::FILE as NODE_FILE;
use crate::tree::{Item, NodePath, Tree};

/// One person as listed: where declared, and who waits for them.
#[derive(Debug, Serialize)]
pub struct Entry {
    pub handle: String,
    pub name: String,
    pub monday: Option<i64>,
    /// References of the items that wait for this person.
    pub awaited_by: Vec<String>,
}

/// A node that declares a tracker, with the people it declares.
#[derive(Debug, Serialize)]
pub struct Directory {
    pub node: NodePath,
    /// The tracker kind and account, such as `monday` and `choosemycompany-group`.
    pub tracker: Option<String>,
    pub account: Option<String>,
    pub people: Vec<Entry>,
}

/// Whether `handle` matches `^[a-z][a-z0-9-]*$` (SPEC.md §1.5).
pub fn is_handle(handle: &str) -> bool {
    let mut chars = handle.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The items within `node` that wait for `handle` as declared at `node`: a
/// sub-node that redeclares the handle has its own person.
fn waiting_for<'a>(tree: &'a Tree, node: &'a NodePath, handle: &str) -> Vec<&'a Item> {
    tree.items_within(Some(node))
        .filter(|item| item.document.texts("waiting_for").contains(&handle))
        .filter(|item| declarer(tree, &item.node, handle).as_ref() == Some(node))
        .collect()
}

/// The nearest node, from `node` up, that declares `handle`.
fn declarer(tree: &Tree, node: &NodePath, handle: &str) -> Option<NodePath> {
    node.ancestors_from_root()
        .into_iter()
        .rev()
        .find(|ancestor| {
            tree.configs
                .files
                .get(ancestor)
                .is_some_and(|file| file.people.contains_key(handle))
        })
}

/// The nodes that declare a tracker, or people, within `scope`, with their people.
pub fn list(tree: &Tree, scope: Option<&NodePath>) -> Vec<Directory> {
    tree.configs
        .files
        .iter()
        .filter(|(node, file)| {
            (file.tracker.is_some() || !file.people.is_empty())
                && scope.is_none_or(|scope| node.is_within(scope) || scope.is_within(node))
        })
        .map(|(node, file)| {
            let field = |key: &str| {
                file.tracker
                    .as_ref()
                    .and_then(|tracker| tracker.get(key))
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
            };
            Directory {
                node: node.clone(),
                tracker: field("kind"),
                account: field("account"),
                people: file
                    .people
                    .iter()
                    .map(|(handle, person)| Entry {
                        handle: handle.clone(),
                        name: person.name.clone(),
                        monday: person.ids.get("monday").and_then(toml::Value::as_integer),
                        awaited_by: waiting_for(tree, node, handle)
                            .into_iter()
                            .filter_map(Item::reference)
                            .collect(),
                    })
                    .collect(),
            }
        })
        .collect()
}

/// What can be changed about a person.
#[derive(Debug, Default, Clone)]
pub struct Change {
    pub name: Option<String>,
    /// `Some(None)` removes the monday id.
    pub monday: Option<Option<i64>>,
}

/// Why a change was refused.
#[derive(Debug, PartialEq)]
pub enum Refusal {
    /// The request is malformed: a bad handle, an unknown node or person.
    Invalid(String),
    /// The person is still awaited by these items.
    Awaited(Vec<String>),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Invalid(message) => f.write_str(message),
            Refusal::Awaited(items) => write!(
                f,
                "still awaited by {}: take them out of waiting, or wait for someone else, first",
                items.join(", ")
            ),
        }
    }
}

/// The `node.toml` of `node`.
fn node_file(tree: &Tree, node: &NodePath) -> PathBuf {
    tree.node_dir(node).join(NODE_FILE)
}

fn edit_node(
    file: &Path,
    change: impl FnOnce(&mut DocumentMut) -> Result<(), Refusal>,
) -> Result<Result<(), Refusal>> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error).with_context(|| format!("reading {}", file.display())),
    };
    let mut document: DocumentMut = text
        .parse()
        .with_context(|| format!("parsing {}", file.display()))?;
    if let Err(refusal) = change(&mut document) {
        return Ok(Err(refusal));
    }
    std::fs::write(file, document.to_string())
        .with_context(|| format!("writing {}", file.display()))?;
    Ok(Ok(()))
}

fn people_table(document: &mut DocumentMut) -> &mut Table {
    if !document.contains_table("people") {
        let mut table = Table::new();
        table.set_implicit(true);
        document.insert("people", TomlItem::Table(table));
    }
    document["people"]
        .as_table_mut()
        .expect("people is a table")
}

/// Declares a person at `node`.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses a bad handle, an
/// unknown node, or a handle already declared there.
pub fn add(
    tree: &Tree,
    node: &NodePath,
    handle: &str,
    name: &str,
    monday: Option<i64>,
) -> Result<Result<(), Refusal>> {
    if !is_handle(handle) {
        return Ok(Err(Refusal::Invalid(format!(
            "`{handle}` is not a handle: lower case letters, digits and `-`, starting with a letter"
        ))));
    }
    if name.trim().is_empty() {
        return Ok(Err(Refusal::Invalid("a person needs a name".to_owned())));
    }
    if !tree.node_dir(node).is_dir() {
        return Ok(Err(Refusal::Invalid(format!("`{node}` is not a node"))));
    }
    edit_node(&node_file(tree, node), |document| {
        let people = people_table(document);
        if people.contains_key(handle) {
            return Err(Refusal::Invalid(format!(
                "`{handle}` is already declared in `{node}`"
            )));
        }
        let mut person = Table::new();
        person.insert("name", value(name.trim()));
        if let Some(id) = monday {
            person.insert("monday", value(id));
        }
        people.insert(handle, TomlItem::Table(person));
        Ok(())
    })
}

/// Changes the name or the monday id of a person declared at `node`.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses an unknown person.
pub fn set(
    tree: &Tree,
    node: &NodePath,
    handle: &str,
    change: &Change,
) -> Result<Result<(), Refusal>> {
    edit_node(&node_file(tree, node), |document| {
        let people = people_table(document);
        let Some(person) = people.get_mut(handle).and_then(TomlItem::as_table_like_mut) else {
            return Err(Refusal::Invalid(format!(
                "no `{handle}` declared in `{node}`"
            )));
        };
        if let Some(name) = &change.name {
            if name.trim().is_empty() {
                return Err(Refusal::Invalid("a person needs a name".to_owned()));
            }
            person.insert("name", value(name.trim()));
        }
        match change.monday {
            Some(Some(id)) => {
                person.insert("monday", value(id));
            }
            Some(None) => {
                person.remove("monday");
            }
            None => {}
        }
        Ok(())
    })
}

/// Renames a handle declared at `node`, in `node.toml` and in the
/// `waiting_for` of every item that waits for that person; returns the items
/// rewritten.
///
/// # Errors
///
/// Fails when a file cannot be read or written; refuses a bad or taken handle,
/// or an unknown person.
pub fn rename(
    tree: &Tree,
    node: &NodePath,
    old: &str,
    new: &str,
) -> Result<Result<Vec<String>, Refusal>> {
    if !is_handle(new) {
        return Ok(Err(Refusal::Invalid(format!(
            "`{new}` is not a handle: lower case letters, digits and `-`, starting with a letter"
        ))));
    }
    let items = waiting_for(tree, node, old);
    let renamed = edit_node(&node_file(tree, node), |document| {
        let people = people_table(document);
        if people.contains_key(new) {
            return Err(Refusal::Invalid(format!(
                "`{new}` is already declared in `{node}`"
            )));
        }
        let Some(person) = people.remove(old) else {
            return Err(Refusal::Invalid(format!("no `{old}` declared in `{node}`")));
        };
        people.insert(new, person);
        Ok(())
    })?;
    if let Err(refusal) = renamed {
        return Ok(Err(refusal));
    }
    let mut rewritten = Vec::new();
    for item in items {
        replace_handle(&item.file, old, new)?;
        rewritten.extend(item.reference());
    }
    Ok(Ok(rewritten))
}

/// Replaces `old` by `new` in the `waiting_for` of the item file `file`.
fn replace_handle(file: &Path, old: &str, new: &str) -> Result<()> {
    let text =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let document = Document::parse(&text);
    let Header::Present(mut header) = document.header else {
        return Ok(());
    };
    if let Some(Value::Sequence(list)) = header.get_mut("waiting_for") {
        for entry in list.iter_mut() {
            if entry.as_str() == Some(old) {
                *entry = Value::String(new.to_owned());
            }
        }
    }
    std::fs::write(file, render(&header, &document.body)?)
        .with_context(|| format!("writing {}", file.display()))
}

/// Removes a person declared at `node`, unless items still wait for them.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses an unknown or
/// still awaited person.
pub fn remove(tree: &Tree, node: &NodePath, handle: &str) -> Result<Result<(), Refusal>> {
    let awaited: Vec<String> = waiting_for(tree, node, handle)
        .into_iter()
        .filter_map(Item::reference)
        .collect();
    if !awaited.is_empty() {
        return Ok(Err(Refusal::Awaited(awaited)));
    }
    edit_node(&node_file(tree, node), |document| {
        if people_table(document).remove(handle).is_none() {
            return Err(Refusal::Invalid(format!(
                "no `{handle}` declared in `{node}`"
            )));
        }
        Ok(())
    })
}

/// Runs `ariane people`.
///
/// # Errors
///
/// Fails when a file cannot be read or written, or a change is refused.
pub fn run(tree: &Tree, args: &PeopleArgs, scope: Option<&NodePath>) -> Result<ExitCode> {
    let outcome = match &args.command {
        None => {
            let directories = list(tree, scope);
            if args.json {
                println!("{}", serde_json::to_string_pretty(&directories)?);
            } else {
                for directory in &directories {
                    println!(
                        "{}  {}",
                        directory.node,
                        directory.account.as_deref().unwrap_or("")
                    );
                    for person in &directory.people {
                        let monday = person
                            .monday
                            .map(|id| format!("  monday:{id}"))
                            .unwrap_or_default();
                        let awaited = if person.awaited_by.is_empty() {
                            String::new()
                        } else {
                            format!("  awaited by {}", person.awaited_by.join(", "))
                        };
                        println!("  {:12} {}{monday}{awaited}", person.handle, person.name);
                    }
                }
            }
            return Ok(ExitCode::SUCCESS);
        }
        Some(PeopleCommand::Add {
            node,
            handle,
            name,
            monday,
        }) => add(tree, &NodePath::new(node), handle, name, *monday),
        Some(PeopleCommand::Set {
            node,
            handle,
            name,
            monday,
            no_monday,
        }) => set(
            tree,
            &NodePath::new(node),
            handle,
            &Change {
                name: name.clone(),
                monday: if *no_monday {
                    Some(None)
                } else {
                    monday.map(Some)
                },
            },
        ),
        Some(PeopleCommand::Rename { node, handle, to }) => {
            rename(tree, &NodePath::new(node), handle, to).map(|outcome| {
                outcome.map(|items| {
                    if !items.is_empty() {
                        println!("rewrote {}", items.join(", "));
                    }
                })
            })
        }
        Some(PeopleCommand::Remove { node, handle }) => remove(tree, &NodePath::new(node), handle),
        Some(PeopleCommand::Find { .. }) => unreachable!("answered before loading the tree"),
    }?;
    match outcome {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(refusal) => bail!("{refusal}"),
    }
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
            "# the team\n[tracker]\nkind = \"monday\"\naccount = \"acme\"\nboard = 1\n\n[people.keyvan]\nname = \"Keyvan\"\nmonday = 42 # Keyvan's id\n",
        )
        .write("cmc/lib/tasks/T1-a.md", "---\nariane: 1\nnode: cmc/lib\nid: T1\ntitle: A\nstatus: waiting\nwaiting_for: [keyvan]\nwaiting_on: review\n---\n# A\n")
        .write("cmc/lib/tasks/T2-b.md", "---\nariane: 1\nnode: cmc/lib\nid: T2\ntitle: B\nstatus: todo\n---\n");
        repo
    }

    #[test]
    fn the_directory_lists_tracker_nodes_and_who_awaits_whom() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let directories = list(&tree, None);
        assert_eq!(directories.len(), 1);
        assert_eq!(directories[0].account.as_deref(), Some("acme"));
        assert_eq!(directories[0].people[0].handle, "keyvan");
        assert_eq!(directories[0].people[0].monday, Some(42));
        assert_eq!(directories[0].people[0].awaited_by, ["cmc/lib#T1"]);
        assert_eq!(list(&tree, Some(&NodePath::new("cmc/lib"))).len(), 1);
        assert!(list(&tree, Some(&NodePath::new("other"))).is_empty());
    }

    #[test]
    fn adding_and_setting_keep_the_file_and_check_their_input() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let cmc = NodePath::new("cmc");
        add(&tree, &cmc, "doryan", "Doryan", None).unwrap().unwrap();
        assert!(matches!(
            add(&tree, &cmc, "doryan", "D", None).unwrap(),
            Err(Refusal::Invalid(_))
        ));
        assert!(matches!(
            add(&tree, &cmc, "Bad Handle", "X", None).unwrap(),
            Err(Refusal::Invalid(_))
        ));
        set(
            &tree,
            &cmc,
            "doryan",
            &Change {
                name: Some("Doryan B.".to_owned()),
                monday: Some(Some(7)),
            },
        )
        .unwrap()
        .unwrap();
        set(
            &tree,
            &cmc,
            "keyvan",
            &Change {
                monday: Some(None),
                ..Change::default()
            },
        )
        .unwrap()
        .unwrap();
        let text = std::fs::read_to_string(repo.root().join("tree/cmc/node.toml")).unwrap();
        assert!(text.starts_with("# the team\n"), "{text}");
        let tree = Tree::load(repo.root()).unwrap();
        let people = &tree.configs.effective(&cmc).people;
        assert_eq!(people["doryan"].name, "Doryan B.");
        assert_eq!(people["doryan"].ids["monday"].as_integer(), Some(7));
        assert!(!people["keyvan"].ids.contains_key("monday"));
    }

    #[test]
    fn renaming_rewrites_the_items_and_removing_refuses_while_awaited() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let cmc = NodePath::new("cmc");
        assert_eq!(
            remove(&tree, &cmc, "keyvan").unwrap(),
            Err(Refusal::Awaited(vec!["cmc/lib#T1".to_owned()]))
        );
        assert_eq!(
            rename(&tree, &cmc, "keyvan", "kcoen").unwrap().unwrap(),
            ["cmc/lib#T1"]
        );
        let task = std::fs::read_to_string(repo.root().join("tree/cmc/lib/tasks/T1-a.md")).unwrap();
        assert!(task.contains("waiting_for:\n- kcoen\n"), "{task}");
        let tree = Tree::load(repo.root()).unwrap();
        let unknown = crate::doctor::diagnose(&tree, None, false)
            .into_iter()
            .filter(|finding| serde_json::to_value(finding.severity).unwrap() == "error")
            .count();
        assert_eq!(unknown, 0);
        repo.write(
            "cmc/lib/tasks/T1-a.md",
            "---\nariane: 1\nnode: cmc/lib\nid: T1\ntitle: A\nstatus: todo\n---\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        remove(&tree, &cmc, "kcoen").unwrap().unwrap();
        let tree = Tree::load(repo.root()).unwrap();
        assert!(tree.configs.effective(&cmc).people.is_empty());
    }
}

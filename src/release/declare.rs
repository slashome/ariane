//! Writing releases: the `[[release]]` list of a node and the `release` of
//! its items (ADR-0023, SPEC.md §1.5 "Changing releases").
//!
//! A list is edited only in the `node.toml` that declares it: a node that
//! inherits its list is refused, naming the node to edit instead. Adding a
//! release to a node with no list anywhere above it starts the node's own.
//! `node.toml` is rewritten through [`crate::node::edit`], so comments, order
//! and unknown keys are kept and a list `doctor` would reject is never
//! written. A renamed release is rewritten in the `release` of every item that
//! plans against that list, archived ones included; a release still named by
//! an item is not removed, the items are listed instead. Assigning writes or
//! removes the `release` of one epic, story or task, through the header writer
//! the rest of the CLI uses, and refuses a name the item's node does not
//! declare.

use std::path::Path;

use anyhow::{Context, Result};
use toml_edit::{ArrayOfTables, DocumentMut, Item as TomlItem, Table, value};
use yaml_serde::Value;

use crate::item::{self, Document, Header, Kind};
use crate::node::{FILE as NODE_FILE, Release};
use crate::schema::is_date;
use crate::tree::{Item, NodePath, Tree};

/// Where a release goes in the delivery order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place<'a> {
    /// First of the list.
    Top,
    /// Last of the list.
    Bottom,
    /// Right before the release of this name.
    Before(&'a str),
    /// Right after the release of this name.
    After(&'a str),
}

/// A release to declare: its name, whether it is a parking one, its target date.
#[derive(Debug, Clone, Default)]
pub struct Declaration {
    pub name: String,
    pub parking: bool,
    /// `YYYY-MM-DD`, written as a TOML date.
    pub target: Option<String>,
}

/// Why a change of releases was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The request is malformed: a blank or taken name, an unknown node,
    /// release or place, an item that cannot carry a release.
    Invalid(String),
    /// `node` plans against the list `from` declares: that list is edited
    /// there, never copied down.
    Inherited { node: NodePath, from: NodePath },
    /// The release is still named by these items.
    Used { release: String, items: Vec<String> },
    /// The release still has this open work, and closing it was asked
    /// without carrying it over.
    Open { release: String, items: Vec<String> },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Invalid(message) => f.write_str(message),
            Refusal::Inherited { node, from } => write!(
                f,
                "`{node}` inherits its releases from `{from}`: change them there"
            ),
            Refusal::Used { release, items } => write!(
                f,
                "`{release}` is still the release of {}: assign them to another release, or none, first",
                items.join(", ")
            ),
            Refusal::Open { release, items } => write!(
                f,
                "`{release}` still has open work: {}; finish it, or carry it over to the next release",
                items.join(", ")
            ),
        }
    }
}

/// Refuses an unknown node, and a node whose release list is declared by an
/// ancestor.
pub fn check_declarer(tree: &Tree, node: &NodePath) -> Result<(), Refusal> {
    if !tree.node_dir(node).is_dir() {
        return Err(Refusal::Invalid(format!("`{node}` is not a node")));
    }
    match tree.configs.effective(node).releases_from {
        Some(from) if from != *node => Err(Refusal::Inherited {
            node: node.clone(),
            from,
        }),
        _ => Ok(()),
    }
}

/// A trimmed, non-blank release name.
fn check_name(name: &str) -> Result<&str, Refusal> {
    let name = name.trim();
    if name.is_empty() {
        Err(Refusal::Invalid("a release needs a name".to_owned()))
    } else {
        Ok(name)
    }
}

/// The `[[release]]` entries of `document`, created empty when absent.
fn list(document: &mut DocumentMut) -> Result<&mut ArrayOfTables, Refusal> {
    if !document.contains_key("release") {
        document.insert("release", TomlItem::ArrayOfTables(ArrayOfTables::new()));
    }
    document["release"].as_array_of_tables_mut().ok_or_else(|| {
        Refusal::Invalid("`release` of this node.toml is not a `[[release]]` list".to_owned())
    })
}

/// The names of the entries, in order.
fn names(list: &ArrayOfTables) -> Vec<String> {
    list.iter()
        .map(|entry| {
            entry
                .get("name")
                .and_then(TomlItem::as_str)
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

/// The index of the release named `name` in `names`.
fn index_of(names: &[String], name: &str, node: &NodePath) -> Result<usize, Refusal> {
    names
        .iter()
        .position(|candidate| candidate == name)
        .ok_or_else(|| Refusal::Invalid(format!("no release `{name}` declared in `{node}`")))
}

/// The index `place` designates among `names`, from 0 to `names.len()`.
fn index_for(names: &[String], place: Place, node: &NodePath) -> Result<usize, Refusal> {
    Ok(match place {
        Place::Top => 0,
        Place::Bottom => names.len(),
        Place::Before(other) => index_of(names, other, node)?,
        Place::After(other) => index_of(names, other, node)? + 1,
    })
}

/// Doubles the document position of every table, so that a free position lies
/// between any two neighbours.
fn spread(table: &mut Table) {
    if let Some(position) = table.position() {
        table.set_position(Some(position * 2));
    }
    for (_, entry) in table.iter_mut() {
        match entry {
            TomlItem::Table(inner) => spread(inner),
            TomlItem::ArrayOfTables(entries) => entries.iter_mut().for_each(spread),
            _ => {}
        }
    }
}

/// The highest document position of `table` and the tables inside it.
fn last_position(table: &Table) -> isize {
    table
        .iter()
        .map(|(_, entry)| match entry {
            TomlItem::Table(inner) => last_position(inner),
            TomlItem::ArrayOfTables(entries) => {
                entries.iter().map(last_position).max().unwrap_or(0)
            }
            _ => 0,
        })
        .chain(table.position())
        .max()
        .unwrap_or(0)
}

/// A `YYYY-MM-DD` target as a TOML date, or its refusal.
fn parse_target(date: &str) -> Result<Result<toml_edit::Datetime, Refusal>> {
    let date = date.trim();
    if !is_date(date) {
        return Ok(Err(Refusal::Invalid(format!(
            "`{date}` is not a YYYY-MM-DD date"
        ))));
    }
    Ok(Ok(date
        .parse::<toml_edit::Datetime>()
        .with_context(|| format!("reading the date {date}"))?))
}

/// Declares `release` in the list of `node`, at `place`, and returns the list.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses a blank or
/// taken name, a bad target, an unknown place, an inherited list.
pub fn add(
    tree: &Tree,
    node: &NodePath,
    release: &Declaration,
    place: Place,
) -> Result<Result<(), Refusal>> {
    if let Err(refusal) = check_declarer(tree, node) {
        return Ok(Err(refusal));
    }
    let name = match check_name(&release.name) {
        Ok(name) => name,
        Err(refusal) => return Ok(Err(refusal)),
    };
    let target = match release.target.as_deref().map(parse_target).transpose()? {
        Some(Err(refusal)) => return Ok(Err(refusal)),
        Some(Ok(date)) => Some(date),
        None => None,
    };
    crate::node::edit(&tree.node_dir(node).join(NODE_FILE), |document| {
        let top = last_position(document.as_table()) * 2 + 2;
        spread(document.as_table_mut());
        let entries = list(document)?;
        let names = names(entries);
        if names.iter().any(|taken| taken == name) {
            return Err(Refusal::Invalid(format!(
                "`{name}` is already a release of `{node}`"
            )));
        }
        let index = index_for(&names, place, node)?;
        let position = match (index.checked_sub(1), entries.get(index)) {
            (Some(previous), _) => entries
                .get(previous)
                .and_then(Table::position)
                .map(|p| p + 1),
            (None, Some(next)) => next.position().map(|p| p - 1),
            (None, None) => None,
        };
        let mut entry = Table::new();
        entry.insert("name", value(name));
        if release.parking {
            entry.insert("parking", value(true));
        }
        if let Some(target) = target {
            entry.insert("target", value(target));
        }
        entry.set_position(Some(position.unwrap_or(top)));
        entries.insert(index, entry);
        Ok(())
    })
}

/// Moves the release `name` of `node` to `place`, the other entries keeping
/// their order and their comments.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses an unknown
/// release or place, a place relative to the release itself, an inherited list.
pub fn move_to(
    tree: &Tree,
    node: &NodePath,
    name: &str,
    place: Place,
) -> Result<Result<(), Refusal>> {
    if let Err(refusal) = check_declarer(tree, node) {
        return Ok(Err(refusal));
    }
    if matches!(place, Place::Before(other) | Place::After(other) if other == name) {
        return Ok(Err(Refusal::Invalid(format!(
            "`{name}` cannot be placed relative to itself"
        ))));
    }
    crate::node::edit(&tree.node_dir(node).join(NODE_FILE), |document| {
        let entries = list(document)?;
        let from = index_of(&names(entries), name, node)?;
        let mut positions: Vec<Option<isize>> = entries.iter().map(Table::position).collect();
        positions.sort_unstable();
        let moved = entries.remove(from);
        let index = index_for(&names(entries), place, node)?;
        entries.insert(index, moved);
        for (entry, position) in entries.iter_mut().zip(positions) {
            entry.set_position(position);
        }
        Ok(())
    })
}

/// What [`set`] changes on a declared release; `None` leaves a mark as it is.
#[derive(Debug, Clone, Copy, Default)]
pub struct Marks<'a> {
    /// Whether the release is a parking one: `false` removes the key.
    pub parking: Option<bool>,
    /// The target date, `YYYY-MM-DD`; `Some(None)` removes it.
    pub target: Option<Option<&'a str>>,
}

/// Changes whether the release `name` of `node` is a parking one, and its
/// target date, the rest of its entry and of the file kept.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses an unknown
/// release, a bad target, a request that sets nothing, an inherited list.
pub fn set(tree: &Tree, node: &NodePath, name: &str, marks: Marks) -> Result<Result<(), Refusal>> {
    if let Err(refusal) = check_declarer(tree, node) {
        return Ok(Err(refusal));
    }
    if marks.parking.is_none() && marks.target.is_none() {
        return Ok(Err(Refusal::Invalid(
            "nothing to set: give a parking mark or a target".to_owned(),
        )));
    }
    let target = match marks.target {
        Some(Some(date)) => match parse_target(date)? {
            Ok(date) => Some(Some(date)),
            Err(refusal) => return Ok(Err(refusal)),
        },
        Some(None) => Some(None),
        None => None,
    };
    crate::node::edit(&tree.node_dir(node).join(NODE_FILE), |document| {
        let entries = list(document)?;
        let index = index_of(&names(entries), name, node)?;
        let Some(entry) = entries.get_mut(index) else {
            return Ok(());
        };
        if marks.parking == Some(true) && entry.contains_key("closed") {
            return Err(Refusal::Invalid(format!(
                "`{name}` is closed: a closed release cannot become a parking one"
            )));
        }
        match marks.parking {
            Some(true) => {
                entry.insert("parking", value(true));
            }
            Some(false) => {
                entry.remove("parking");
            }
            None => {}
        }
        match target {
            Some(Some(date)) => {
                entry.insert("target", value(date));
            }
            Some(None) => {
                entry.remove("target");
            }
            None => {}
        }
        Ok(())
    })
}

/// The items that plan against the list `node` declares and name `name` in
/// their own `release`, archived ones included.
fn naming<'a>(tree: &'a Tree, node: &'a NodePath, name: &str) -> Vec<&'a Item> {
    tree.items_within(Some(node))
        .filter(|item| item.document.text("release") == Some(name))
        .filter(|item| tree.configs.effective(&item.node).releases_from.as_ref() == Some(node))
        .collect()
}

/// Renames the release `old` of `node` to `new`, in `node.toml` and in the
/// `release` of every item that names it; returns the items rewritten.
///
/// # Errors
///
/// Fails when a file cannot be read or written; refuses a blank or taken
/// name, an unknown release, an inherited list.
pub fn rename(
    tree: &Tree,
    node: &NodePath,
    old: &str,
    new: &str,
) -> Result<Result<Vec<String>, Refusal>> {
    if let Err(refusal) = check_declarer(tree, node) {
        return Ok(Err(refusal));
    }
    let new = match check_name(new) {
        Ok(new) => new,
        Err(refusal) => return Ok(Err(refusal)),
    };
    let items = naming(tree, node, old);
    let renamed = crate::node::edit(&tree.node_dir(node).join(NODE_FILE), |document| {
        let entries = list(document)?;
        let names = names(entries);
        let index = index_of(&names, old, node)?;
        if old != new && names.iter().any(|taken| taken == new) {
            return Err(Refusal::Invalid(format!(
                "`{new}` is already a release of `{node}`"
            )));
        }
        if let Some(entry) = entries.get_mut(index) {
            entry.insert("name", value(new));
        }
        Ok(())
    })?;
    if let Err(refusal) = renamed {
        return Ok(Err(refusal));
    }
    let mut rewritten = Vec::new();
    for item in items {
        write_release(&item.file, Some(new))?;
        rewritten.extend(item.reference());
    }
    Ok(Ok(rewritten))
}

/// Removes the release `name` of `node`, unless an item still names it.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses an unknown or
/// still used release, an inherited list.
pub fn remove(tree: &Tree, node: &NodePath, name: &str) -> Result<Result<(), Refusal>> {
    if let Err(refusal) = check_declarer(tree, node) {
        return Ok(Err(refusal));
    }
    let used: Vec<String> = naming(tree, node, name)
        .into_iter()
        .filter_map(Item::reference)
        .collect();
    if !used.is_empty() {
        return Ok(Err(Refusal::Used {
            release: name.to_owned(),
            items: used,
        }));
    }
    crate::node::edit(&tree.node_dir(node).join(NODE_FILE), |document| {
        let entries = list(document)?;
        let index = index_of(&names(entries), name, node)?;
        entries.remove(index);
        if entries.is_empty() {
            document.remove("release");
        }
        Ok(())
    })
}

/// Refuses to give `item` the release `release` when it is not an epic, a
/// story or a task, is archived, has no versioned header, or when its node's
/// effective list does not declare that name; `None` (no release) is always
/// allowed on those kinds.
///
/// # Errors
///
/// The refusal, naming the declared releases or the absence of a list.
pub fn check_assignable(tree: &Tree, item: &Item, release: Option<&str>) -> Result<(), Refusal> {
    if !matches!(item.kind, Kind::Task | Kind::Story | Kind::Epic) {
        return Err(Refusal::Invalid(
            "only an epic, a story or a task is planned in a release".to_owned(),
        ));
    }
    if item.archived {
        return Err(Refusal::Invalid(
            "an archived item keeps the release it was closed in".to_owned(),
        ));
    }
    if !matches!(item.document.header, Header::Present(_)) {
        return Err(Refusal::Invalid(
            "the item has no versioned header; run `ariane migrate` first".to_owned(),
        ));
    }
    let Some(name) = release else {
        return Ok(());
    };
    let effective = tree.configs.effective(&item.node);
    if let Some((_, release)) = effective.release(name) {
        return match &release.closed {
            Some(closed) => Err(Refusal::Invalid(format!(
                "`{name}` was closed on {closed}: plan the item in a later release"
            ))),
            None => Ok(()),
        };
    }
    Err(Refusal::Invalid(
        match (&effective.releases, &effective.releases_from) {
            (Some(releases), Some(from)) => format!(
                "`{name}` is not a release of `{from}` ({})",
                releases
                    .iter()
                    .map(|release| release.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            _ => format!(
                "`{}` declares no releases: add `{name}` to its list first",
                item.node
            ),
        },
    ))
}

/// Writes `release` in the header of `item`, or removes the key for `None`;
/// returns whether the file changed.
///
/// # Errors
///
/// Fails when the file cannot be read or written; refuses what
/// [`check_assignable`] refuses.
pub fn assign(tree: &Tree, item: &Item, release: Option<&str>) -> Result<Result<bool, Refusal>> {
    if let Err(refusal) = check_assignable(tree, item, release) {
        return Ok(Err(refusal));
    }
    if item.document.text("release") == release {
        return Ok(Ok(false));
    }
    write_release(&item.file, release)?;
    Ok(Ok(true))
}

/// Writes `release` in the header of the item file `file`, the rest of the
/// header and the body kept.
fn write_release(file: &Path, release: Option<&str>) -> Result<()> {
    let text =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let document = Document::parse(&text);
    let Header::Present(mut header) = document.header else {
        return Ok(());
    };
    item::set_key(
        &mut header,
        "release",
        release.map(|name| Value::String(name.to_owned())),
    );
    std::fs::write(file, item::render(&header, &document.body)?)
        .with_context(|| format!("writing {}", file.display()))
}

/// The release list `node` plans against, and the node that declares it.
pub fn effective_list(tree: &Tree, node: &NodePath) -> (Option<NodePath>, Vec<Release>) {
    let effective = tree.configs.effective(node);
    (
        effective.releases_from,
        effective.releases.unwrap_or_default(),
    )
}

/// Unit tests of writing releases.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// The `node.toml` of `p`: a description, a commented release list with a
    /// target and a parking release, and a tracker with a key Ariane does not
    /// know.
    const NODE: &str = "# Releases of p\ndescription = \"P.\"\n\n# the next one\n[[release]]\nname = \"0.1\"\ntarget = 2027-01-01 # soon\n\n[[release]]\nname = \"0.2\"\n\n[[release]]\nname = \"plus-tard\"\nparking = true\n\n[tracker]\nkind = \"monday\"\nfuture = { a = 1 }\n";

    /// An item of `node` with `id` and extra header lines.
    fn item(node: &str, id: &str, extra: &str) -> String {
        format!(
            "---\nariane: 2\nnode: {node}\nid: {id}\ntitle: {id}\nstatus: todo\n{extra}custom: kept\n---\n# {id}\n\nBody  kept.\n"
        )
    }

    /// `p` declares the list; `p/c` inherits it; `p/d` declares its own;
    /// `q` declares none.
    fn sample() -> Repo {
        let repo = Repo::new();
        repo.write("p/node.toml", NODE)
            .write("p/epics/E1-e.md", &item("p", "E1", "release: '0.2'\n"))
            .write("p/stories/S1-s.md", &item("p", "S1", "parent: E1\n"))
            .write(
                "p/tasks/T1-a.md",
                &item("p", "T1", "parent: S1\nrelease: '0.1'\n"),
            )
            .write("p/tasks/T2-b.md", &item("p", "T2", ""))
            .write(
                "p/archives/T0-z.md",
                &item("p", "T0", "release: '0.1'\nclosed: 2026-11-01\n")
                    .replace("status: todo", "status: done"),
            )
            .write(
                "p/ressources/D1-d.md",
                "---\nariane: 2\nnode: p\nid: D1\ntitle: D\n---\n# D\n",
            )
            .write("p/c/tasks/T1-c.md", &item("p/c", "T1", "release: '0.1'\n"))
            .write("p/d/node.toml", "[[release]]\nname = \"0.1\"\n")
            .write("p/d/tasks/T1-d.md", &item("p/d", "T1", "release: '0.1'\n"))
            .write("q/tasks/T1-q.md", &item("q", "T1", ""));
        repo
    }

    fn tree(repo: &Repo) -> Tree {
        Tree::load(repo.root()).unwrap()
    }

    fn file(repo: &Repo, relative: &str) -> String {
        std::fs::read_to_string(repo.root().join("tree").join(relative)).unwrap()
    }

    fn node_toml(repo: &Repo, node: &str) -> String {
        file(repo, &format!("{node}/node.toml"))
    }

    fn named(name: &str) -> Declaration {
        Declaration {
            name: name.to_owned(),
            ..Declaration::default()
        }
    }

    fn find<'a>(tree: &'a Tree, node: &str, id: &str) -> &'a Item {
        tree.items
            .iter()
            .find(|item| item.node.as_str() == node && item.id() == Some(id))
            .unwrap()
    }

    fn names(tree: &Tree, node: &str) -> Vec<String> {
        effective_list(tree, &NodePath::new(node))
            .1
            .into_iter()
            .map(|release| release.name)
            .collect()
    }

    /// The errors `doctor` finds in the tree.
    fn errors(repo: &Repo) -> Vec<String> {
        crate::doctor::diagnose(&tree(repo), None, false)
            .into_iter()
            .filter(|finding| serde_json::to_value(finding.severity).unwrap() == "error")
            .map(|finding| serde_json::to_string(&finding).unwrap())
            .collect()
    }

    #[test]
    fn a_release_is_added_at_its_place_and_nothing_else_of_the_file_changes() {
        let repo = sample();
        let p = NodePath::new("p");
        add(&tree(&repo), &p, &named("0.3"), Place::After("0.2"))
            .unwrap()
            .unwrap();
        let expected = NODE.replace(
            "[[release]]\nname = \"plus-tard\"",
            "[[release]]\nname = \"0.3\"\n\n[[release]]\nname = \"plus-tard\"",
        );
        assert_eq!(node_toml(&repo, "p"), expected);
        let first = Declaration {
            name: " 0.0 ".to_owned(),
            parking: false,
            target: Some("2026-12-01".to_owned()),
        };
        add(&tree(&repo), &p, &first, Place::Top).unwrap().unwrap();
        let parked = Declaration {
            name: "later".to_owned(),
            parking: true,
            target: None,
        };
        add(&tree(&repo), &p, &parked, Place::Bottom)
            .unwrap()
            .unwrap();
        assert_eq!(
            names(&tree(&repo), "p"),
            ["0.0", "0.1", "0.2", "0.3", "plus-tard", "later"]
        );
        let text = node_toml(&repo, "p");
        assert!(
            text.starts_with("# Releases of p\ndescription = \"P.\"\n"),
            "{text}"
        );
        assert!(
            text.contains("name = \"0.0\"\ntarget = 2026-12-01\n"),
            "{text}"
        );
        assert!(
            text.contains(
                "# the next one\n[[release]]\nname = \"0.1\"\ntarget = 2027-01-01 # soon\n"
            ),
            "{text}"
        );
        assert!(
            text.contains("[tracker]\nkind = \"monday\"\nfuture = { a = 1 }\n"),
            "{text}"
        );
        assert!(errors(&repo).is_empty(), "{:?}", errors(&repo));
    }

    #[test]
    fn an_add_refuses_a_blank_or_taken_name_a_bad_target_and_an_unknown_place() {
        let repo = sample();
        let tree = tree(&repo);
        let p = NodePath::new("p");
        let late = Declaration {
            name: "0.3".to_owned(),
            parking: false,
            target: Some("soon".to_owned()),
        };
        for (declaration, place) in [
            (named("  "), Place::Bottom),
            (named("0.2"), Place::Bottom),
            (named("0.3"), Place::Before("9.9")),
            (late, Place::Bottom),
        ] {
            assert!(matches!(
                add(&tree, &p, &declaration, place).unwrap(),
                Err(Refusal::Invalid(_))
            ));
        }
        assert!(matches!(
            add(&tree, &NodePath::new("nowhere"), &named("1"), Place::Bottom).unwrap(),
            Err(Refusal::Invalid(_))
        ));
        assert_eq!(node_toml(&repo, "p"), NODE);
    }

    #[test]
    fn a_node_that_inherits_its_list_is_refused_and_a_node_without_any_starts_its_own() {
        let repo = sample();
        let tree = tree(&repo);
        let c = NodePath::new("p/c");
        assert_eq!(
            add(&tree, &c, &named("0.3"), Place::Bottom).unwrap(),
            Err(Refusal::Inherited {
                node: c.clone(),
                from: NodePath::new("p"),
            })
        );
        assert!(matches!(
            move_to(&tree, &c, "0.1", Place::Bottom).unwrap(),
            Err(Refusal::Inherited { .. })
        ));
        assert!(matches!(
            rename(&tree, &c, "0.1", "0.9").unwrap(),
            Err(Refusal::Inherited { .. })
        ));
        assert!(matches!(
            remove(&tree, &c, "0.1").unwrap(),
            Err(Refusal::Inherited { .. })
        ));
        assert!(!repo.root().join("tree/p/c/node.toml").exists());
        add(&tree, &NodePath::new("q"), &named("1.0"), Place::Top)
            .unwrap()
            .unwrap();
        assert_eq!(node_toml(&repo, "q"), "[[release]]\nname = \"1.0\"\n");
    }

    #[test]
    fn a_move_reorders_the_entries_with_their_comments() {
        let repo = sample();
        let p = NodePath::new("p");
        move_to(&tree(&repo), &p, "0.1", Place::After("0.2"))
            .unwrap()
            .unwrap();
        assert_eq!(
            node_toml(&repo, "p"),
            "# Releases of p\ndescription = \"P.\"\n\n[[release]]\nname = \"0.2\"\n\n# the next one\n[[release]]\nname = \"0.1\"\ntarget = 2027-01-01 # soon\n\n[[release]]\nname = \"plus-tard\"\nparking = true\n\n[tracker]\nkind = \"monday\"\nfuture = { a = 1 }\n"
        );
        for (name, place) in [
            ("plus-tard", Place::Top),
            ("plus-tard", Place::Bottom),
            ("0.1", Place::Before("0.2")),
        ] {
            move_to(&tree(&repo), &p, name, place).unwrap().unwrap();
        }
        assert_eq!(node_toml(&repo, "p"), NODE);
        let tree = tree(&repo);
        assert!(matches!(
            move_to(&tree, &p, "0.1", Place::After("0.1")).unwrap(),
            Err(Refusal::Invalid(_))
        ));
        assert!(matches!(
            move_to(&tree, &p, "9.9", Place::Top).unwrap(),
            Err(Refusal::Invalid(_))
        ));
    }

    #[test]
    fn a_rename_rewrites_the_items_that_plan_against_the_list_and_only_them() {
        let repo = sample();
        let p = NodePath::new("p");
        let tree = tree(&repo);
        assert!(matches!(
            rename(&tree, &p, "0.1", "0.2").unwrap(),
            Err(Refusal::Invalid(_))
        ));
        assert!(matches!(
            rename(&tree, &p, "0.1", " ").unwrap(),
            Err(Refusal::Invalid(_))
        ));
        let before = file(&repo, "p/tasks/T1-a.md");
        let mut rewritten = rename(&tree, &p, "0.1", "mango").unwrap().unwrap();
        rewritten.sort();
        assert_eq!(rewritten, ["p#T0", "p#T1", "p/c#T1"]);
        assert_eq!(
            node_toml(&repo, "p"),
            NODE.replace("name = \"0.1\"", "name = \"mango\"")
        );
        assert_eq!(
            file(&repo, "p/tasks/T1-a.md"),
            before.replace("release: '0.1'", "release: mango")
        );
        assert!(file(&repo, "p/archives/T0-z.md").contains("release: mango\n"));
        assert!(file(&repo, "p/c/tasks/T1-c.md").contains("release: mango\n"));
        assert!(file(&repo, "p/d/tasks/T1-d.md").contains("release: '0.1'\n"));
        assert!(errors(&repo).is_empty(), "{:?}", errors(&repo));
    }

    #[test]
    fn a_release_still_named_is_not_removed_and_the_last_one_takes_the_list_away() {
        let repo = sample();
        let p = NodePath::new("p");
        let Err(Refusal::Used { release, mut items }) = remove(&tree(&repo), &p, "0.1").unwrap()
        else {
            panic!("0.1 is still named by items");
        };
        items.sort();
        assert_eq!(release, "0.1");
        assert_eq!(items, ["p#T0", "p#T1", "p/c#T1"]);
        remove(&tree(&repo), &p, "plus-tard").unwrap().unwrap();
        assert_eq!(
            node_toml(&repo, "p"),
            NODE.replace("[[release]]\nname = \"plus-tard\"\nparking = true\n\n", "")
        );
        assert!(matches!(
            remove(&tree(&repo), &p, "plus-tard").unwrap(),
            Err(Refusal::Invalid(_))
        ));
        repo.write("p/d/tasks/T1-d.md", &item("p/d", "T1", ""));
        remove(&tree(&repo), &NodePath::new("p/d"), "0.1")
            .unwrap()
            .unwrap();
        assert_eq!(node_toml(&repo, "p/d"), "");
    }

    #[test]
    fn an_item_is_planned_in_a_declared_release_of_its_node_or_in_none() {
        let repo = sample();
        let tree = tree(&repo);
        let before = file(&repo, "p/tasks/T2-b.md");
        assert_eq!(
            assign(&tree, find(&tree, "p", "T2"), Some("0.2")).unwrap(),
            Ok(true)
        );
        assert_eq!(
            file(&repo, "p/tasks/T2-b.md"),
            before.replace("status: todo\n", "status: todo\nrelease: '0.2'\n")
        );
        let reloaded = Tree::load(repo.root()).unwrap();
        let task = find(&reloaded, "p", "T2");
        assert_eq!(assign(&reloaded, task, Some("0.2")).unwrap(), Ok(false));
        assert_eq!(assign(&reloaded, task, None).unwrap(), Ok(true));
        assert_eq!(file(&repo, "p/tasks/T2-b.md"), before);
        assert_eq!(
            assign(&tree, find(&tree, "p", "S1"), Some("plus-tard")).unwrap(),
            Ok(true)
        );
        assert_eq!(
            assign(&tree, find(&tree, "p", "E1"), None).unwrap(),
            Ok(true)
        );
        let refused = |node: &str, id: &str, release: Option<&str>| match assign(
            &tree,
            find(&tree, node, id),
            release,
        )
        .unwrap()
        {
            Err(Refusal::Invalid(message)) => message,
            other => panic!("{node}#{id} should be refused, got {other:?}"),
        };
        assert!(refused("p", "T2", Some("9.9")).contains("0.1, 0.2, plus-tard"));
        assert!(refused("p/d", "T1", Some("0.2")).contains("`p/d`"));
        assert!(refused("q", "T1", Some("1.0")).contains("declares no releases"));
        refused("p", "D1", None);
        refused("p", "T0", Some("0.2"));
        assert!(errors(&repo).is_empty(), "{:?}", errors(&repo));
    }

    #[test]
    fn a_set_changes_the_parking_mark_and_the_target_and_nothing_else() {
        let repo = sample();
        let p = NodePath::new("p");
        let marks =
            |parking: Option<bool>, target: Option<Option<&'static str>>| Marks { parking, target };
        set(&tree(&repo), &p, "0.1", marks(None, Some(None)))
            .unwrap()
            .unwrap();
        assert_eq!(
            node_toml(&repo, "p"),
            NODE.replace("target = 2027-01-01 # soon\n", "")
        );
        set(
            &tree(&repo),
            &p,
            "0.1",
            marks(None, Some(Some(" 2027-02-01 "))),
        )
        .unwrap()
        .unwrap();
        set(&tree(&repo), &p, "plus-tard", marks(Some(false), None))
            .unwrap()
            .unwrap();
        set(
            &tree(&repo),
            &p,
            "0.2",
            marks(Some(true), Some(Some("2027-03-01"))),
        )
        .unwrap()
        .unwrap();
        let text = node_toml(&repo, "p");
        assert!(
            text.contains("[[release]]\nname = \"0.1\"\ntarget = 2027-02-01\n"),
            "{text}"
        );
        assert!(
            text.contains("[[release]]\nname = \"0.2\"\nparking = true\ntarget = 2027-03-01\n"),
            "{text}"
        );
        assert!(
            text.contains("[[release]]\nname = \"plus-tard\"\n\n[tracker]"),
            "{text}"
        );
        assert!(text.starts_with("# Releases of p\ndescription = \"P.\"\n\n# the next one\n"));
        assert!(errors(&repo).is_empty(), "{:?}", errors(&repo));
        let tree = tree(&repo);
        for (node, name, marks) in [
            ("p", "0.1", marks(None, None)),
            ("p", "0.1", marks(None, Some(Some("soon")))),
            ("p", "9.9", marks(Some(true), None)),
        ] {
            assert!(matches!(
                set(&tree, &NodePath::new(node), name, marks).unwrap(),
                Err(Refusal::Invalid(_))
            ));
        }
        assert!(matches!(
            set(&tree, &NodePath::new("p/c"), "0.1", marks(Some(true), None)).unwrap(),
            Err(Refusal::Inherited { .. })
        ));
    }
}

//! Writing the `[estimates]` table of a node (ADR-0025, SPEC.md §1.5
//! "Changing estimates").
//!
//! Each key is inherited on its own, the nearest declaration winning, so a
//! key is always set in the node's own `node.toml`, even when the rest of its
//! table comes from an ancestor: unlike a release list, nothing is copied
//! down, the other keys keep coming from where they came from. [`Sources`]
//! says, key by key, which node a value comes from.
//!
//! A value is checked by the parser that reads `node.toml`, so a value it
//! would refuse is never written; the file is rewritten through
//! [`crate::node::edit`], comments, order and the other tables kept. Removing
//! the last key removes the table.

use std::process::ExitCode;

use anyhow::{Result, bail};
use serde::Serialize;
use toml_edit::{DocumentMut, Item as TomlItem, Table, Value as TomlValue};

use super::{Declaration, Pricing};
use crate::cli::{EstimatesArgs, EstimatesCommand};
use crate::node::{Configs, FILE as NODE_FILE, set_value};
use crate::tree::{NodePath, Tree};

/// Every key of `[estimates]`, in the order the schema lists them.
pub const KEYS: [&str; 8] = ["XS", "S", "M", "L", "XL", "default", "test_ratio", "tester"];

/// The node each key of a node's [`Pricing`] comes from; `None` for the value
/// of ADR-0025, which no node declares.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Sources {
    /// Where the hours of an `XS` come from.
    #[serde(rename = "XS")]
    pub xs: Option<NodePath>,
    /// Where the hours of an `S` come from.
    #[serde(rename = "S")]
    pub s: Option<NodePath>,
    /// Where the hours of an `M` come from.
    #[serde(rename = "M")]
    pub m: Option<NodePath>,
    /// Where the hours of an `L` come from.
    #[serde(rename = "L")]
    pub l: Option<NodePath>,
    /// Where the hours of an `XL` come from.
    #[serde(rename = "XL")]
    pub xl: Option<NodePath>,
    /// Where the default size comes from.
    pub default: Option<NodePath>,
    /// Where the test ratio comes from.
    pub test_ratio: Option<NodePath>,
    /// Where the tester comes from.
    pub tester: Option<NodePath>,
}

impl Sources {
    /// Marks every key `declaration` gives as coming from `node`.
    fn apply(&mut self, declaration: &Declaration, node: &NodePath) {
        let pairs = [
            (&mut self.xs, declaration.xs.is_some()),
            (&mut self.s, declaration.s.is_some()),
            (&mut self.m, declaration.m.is_some()),
            (&mut self.l, declaration.l.is_some()),
            (&mut self.xl, declaration.xl.is_some()),
            (&mut self.default, declaration.default.is_some()),
            (&mut self.test_ratio, declaration.test_ratio.is_some()),
            (&mut self.tester, declaration.tester.is_some()),
        ];
        for (slot, declared) in pairs {
            if declared {
                *slot = Some(node.clone());
            }
        }
    }
}

/// Where each key of `node`'s pricing comes from: the nearest node, itself or
/// an ancestor, whose `[estimates]` declares it.
pub fn sources(configs: &Configs, node: &NodePath) -> Sources {
    let mut sources = Sources::default();
    for ancestor in node.ancestors_from_root() {
        if let Some(file) = configs.files.get(&ancestor) {
            sources.apply(&file.estimates, &ancestor);
        }
    }
    sources
}

/// What a node's estimates come to: `ariane estimates <node> --json` and the
/// answer of `PATCH /api/estimates`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    /// The node.
    pub node: NodePath,
    /// What sizes cost on it, inherited key by key.
    pub estimates: Pricing,
    /// The node each key comes from; `null` for the value of ADR-0025.
    pub estimates_from: Sources,
}

/// The estimates of `node` and where each comes from.
pub fn report(tree: &Tree, node: &NodePath) -> Report {
    Report {
        node: node.clone(),
        estimates: tree.configs.pricing(node),
        estimates_from: sources(&tree.configs, node),
    }
}

/// Sets or removes keys of `node`'s own `[estimates]`: each `(key, value)`
/// sets `key` to `value`, or removes it for `None`, all at once or none.
///
/// # Errors
///
/// Fails when `node.toml` cannot be read or written; refuses an unknown node
/// or key, an empty change, and a value `node.toml` would not accept.
pub fn change(
    tree: &Tree,
    node: &NodePath,
    changes: &[(String, Option<toml::Value>)],
) -> Result<Result<(), String>> {
    if !tree.node_dir(node).is_dir() {
        return Ok(Err(format!("`{node}` is not a node")));
    }
    if changes.is_empty() {
        return Ok(Err(
            "nothing to change: give a key of [estimates] and its value".to_owned(),
        ));
    }
    if let Err(refusal) = validate(changes) {
        return Ok(Err(refusal));
    }
    crate::node::edit(&tree.node_dir(node).join(NODE_FILE), |document| {
        write(document, changes)
    })
}

/// Refuses an unknown key, and a value the `node.toml` parser would refuse.
fn validate(changes: &[(String, Option<toml::Value>)]) -> Result<(), String> {
    let mut table = toml::Table::new();
    for (key, value) in changes {
        if !KEYS.contains(&key.as_str()) {
            return Err(format!(
                "`{key}` is not a key of [estimates]: {}",
                KEYS.join(", ")
            ));
        }
        if let Some(value) = value {
            table.insert(key.clone(), value.clone());
        }
    }
    toml::Value::Table(table)
        .try_into::<Declaration>()
        .map(|_| ())
        .map_err(|error| format!("[estimates]: {}", error.message().trim()))
}

/// Applies `changes` to the `[estimates]` table of `document`, created when
/// absent and removed once empty.
fn write(
    document: &mut DocumentMut,
    changes: &[(String, Option<toml::Value>)],
) -> Result<(), String> {
    if !document.contains_key("estimates") {
        document.insert("estimates", TomlItem::Table(Table::new()));
    }
    let table = document
        .get_mut("estimates")
        .and_then(TomlItem::as_table_like_mut)
        .ok_or_else(|| "`estimates` of this node.toml is not a table".to_owned())?;
    for (key, value) in changes {
        match value {
            Some(value) => set_value(table, key, edit_value(value)?),
            None => {
                table.remove(key);
            }
        }
    }
    if table.is_empty() {
        document.remove("estimates");
    }
    Ok(())
}

/// A checked value of `[estimates]` as `toml_edit` writes it.
fn edit_value(value: &toml::Value) -> Result<TomlValue, String> {
    match value {
        toml::Value::Integer(integer) => Ok(TomlValue::from(*integer)),
        toml::Value::Float(float) => Ok(TomlValue::from(*float)),
        toml::Value::String(text) => Ok(TomlValue::from(text.as_str())),
        other => Err(format!("`{other}` is not a value of [estimates]")),
    }
}

/// The value `text` stands for on the command line: a number for the hours of
/// a size and for `test_ratio`, a string for `default` and `tester`.
pub fn parse_value(key: &str, text: &str) -> toml::Value {
    let text = text.trim();
    if matches!(key, "default" | "tester") {
        return toml::Value::String(text.to_owned());
    }
    if let Ok(integer) = text.parse::<i64>() {
        return toml::Value::Integer(integer);
    }
    match text.parse::<f64>() {
        Ok(float) => toml::Value::Float(float),
        Err(_) => toml::Value::String(text.to_owned()),
    }
}

/// Runs `ariane estimates`: prints a node's estimates and where each comes
/// from, or sets or removes one key of its own table first.
///
/// # Errors
///
/// Fails when a file cannot be read or written, and when a change is refused.
pub fn run(tree: &Tree, args: &EstimatesArgs, here: Option<&NodePath>) -> Result<ExitCode> {
    let (node, json, outcome) = match &args.command {
        Some(EstimatesCommand::Set { node, key, value }) => {
            let node = NodePath::new(node);
            let value = parse_value(key, value);
            let outcome = change(tree, &node, &[(key.clone(), Some(value))])?;
            (node, false, Some(outcome))
        }
        Some(EstimatesCommand::Unset { node, key }) => {
            let node = NodePath::new(node);
            let outcome = change(tree, &node, &[(key.clone(), None)])?;
            (node, false, Some(outcome))
        }
        None => {
            let node = args
                .node
                .as_deref()
                .map(NodePath::new)
                .or_else(|| here.cloned())
                .unwrap_or_default();
            (node, args.json, None)
        }
    };
    if let Some(Err(refusal)) = outcome {
        bail!("{refusal}");
    }
    let tree = match outcome {
        Some(_) => &Tree::load(&tree.root)?,
        None => tree,
    };
    if !tree.node_dir(&node).is_dir() {
        bail!("`{node}` is not a node");
    }
    let report = report(tree, &node);
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", describe(&report));
    }
    Ok(ExitCode::SUCCESS)
}

/// The lines `ariane estimates <node>` prints: one key per line, its value and
/// the node it comes from.
fn describe(report: &Report) -> String {
    let pricing = &report.estimates;
    let from = &report.estimates_from;
    let hours = |value: f64| format!("{value} h");
    let rows = [
        ("XS", hours(pricing.xs), &from.xs),
        ("S", hours(pricing.s), &from.s),
        ("M", hours(pricing.m), &from.m),
        ("L", hours(pricing.l), &from.l),
        ("XL", hours(pricing.xl), &from.xl),
        ("default", pricing.default.to_owned(), &from.default),
        (
            "test_ratio",
            pricing.test_ratio.to_string(),
            &from.test_ratio,
        ),
        ("tester", pricing.tester.id().to_owned(), &from.tester),
    ];
    let shown = if report.node.is_home() {
        "(home)".to_owned()
    } else {
        report.node.to_string()
    };
    let mut text = format!("{shown}  [estimates]\n");
    for (key, value, source) in rows {
        let origin = match source {
            Some(source) if source == &report.node => "own".to_owned(),
            Some(source) if source.is_home() => "from the home node".to_owned(),
            Some(source) => format!("from {source}"),
            None => "default".to_owned(),
        };
        text.push_str(&format!("  {key:<11} {value:<8} {origin}\n"));
    }
    text
}

/// Unit tests of the estimates writer.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// A node `p` with a commented `[estimates]` among other tables, and its
    /// child `p/c` without one.
    fn sample() -> (Repo, Tree) {
        let repo = Repo::new();
        repo.write(
            "p/node.toml",
            "# p\ndescription = \"p\"\n\n# hours\n[estimates]\nM = 10 # a day and a bit\ntest_ratio = 0.5\n\n[[release]]\nname = \"1.0\"\n",
        )
        .write("p/c/node.toml", "description = \"c\"\n");
        let tree = Tree::load(repo.root()).unwrap();
        (repo, tree)
    }

    /// The `node.toml` of `node`.
    fn read(repo: &Repo, node: &str) -> String {
        std::fs::read_to_string(repo.root().join("tree").join(node).join("node.toml")).unwrap()
    }

    /// Sets `key` of `node` to the command-line `text`.
    fn set(tree: &Tree, node: &str, key: &str, text: &str) -> Result<(), String> {
        change(
            tree,
            &NodePath::new(node),
            &[(key.to_owned(), Some(parse_value(key, text)))],
        )
        .unwrap()
    }

    #[test]
    fn a_key_is_set_in_place_keeping_its_comment_and_the_rest_of_the_file() {
        let (repo, tree) = sample();
        let before = read(&repo, "p");
        set(&tree, "p", "M", "12").unwrap();
        assert_eq!(read(&repo, "p"), before.replace("M = 10 #", "M = 12 #"));
        set(&tree, "p", "default", "S").unwrap();
        assert!(
            read(&repo, "p").contains("test_ratio = 0.5\ndefault = \"S\"\n"),
            "{}",
            read(&repo, "p")
        );
    }

    #[test]
    fn a_child_gets_its_own_key_and_inherits_the_others_key_by_key() {
        let (repo, tree) = sample();
        set(&tree, "p/c", "test_ratio", "0.1").unwrap();
        assert_eq!(
            read(&repo, "p/c"),
            "description = \"c\"\n\n[estimates]\ntest_ratio = 0.1\n"
        );
        let tree = Tree::load(repo.root()).unwrap();
        let report = report(&tree, &NodePath::new("p/c"));
        assert_eq!(
            (report.estimates.m, report.estimates.test_ratio),
            (10.0, 0.1)
        );
        assert_eq!(report.estimates_from.m, Some(NodePath::new("p")));
        assert_eq!(report.estimates_from.test_ratio, Some(NodePath::new("p/c")));
        assert_eq!(report.estimates_from.xs, None);
    }

    #[test]
    fn removing_the_last_key_removes_the_table() {
        let (repo, tree) = sample();
        let path = NodePath::new("p");
        change(
            &tree,
            &path,
            &[("M".to_owned(), None), ("test_ratio".to_owned(), None)],
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            read(&repo, "p"),
            "# p\ndescription = \"p\"\n\n[[release]]\nname = \"1.0\"\n"
        );
    }

    #[test]
    fn an_invalid_value_or_key_is_refused_and_nothing_is_written() {
        let (repo, tree) = sample();
        let before = read(&repo, "p");
        for (key, text, said) in [
            ("M", "0", "above zero"),
            ("M", "lots", "not a number"),
            ("test_ratio", "-1", "negative"),
            ("default", "XXL", "not one of the sizes"),
            ("tester", "nobody", "nobody"),
            ("XXL", "3", "not a key"),
        ] {
            let refusal = set(&tree, "p", key, text).unwrap_err();
            assert!(refusal.contains(said), "{key} = {text}: {refusal}");
        }
        assert_eq!(read(&repo, "p"), before);
        assert!(set(&tree, "nowhere", "M", "3").is_err());
    }

    #[test]
    fn several_keys_change_at_once_or_not_at_all() {
        let (repo, tree) = sample();
        let before = read(&repo, "p");
        let refused = change(
            &tree,
            &NodePath::new("p"),
            &[
                ("S".to_owned(), Some(toml::Value::Integer(3))),
                ("tester".to_owned(), Some(toml::Value::String("x".into()))),
            ],
        )
        .unwrap();
        assert!(refused.is_err());
        assert_eq!(read(&repo, "p"), before);
    }

    #[test]
    fn the_description_says_where_each_value_comes_from() {
        let (repo, tree) = sample();
        set(&tree, "p/c", "XL", "40").unwrap();
        let tree = Tree::load(repo.root()).unwrap();
        let text = describe(&report(&tree, &NodePath::new("p/c")));
        assert!(text.contains("XL          40 h     own"), "{text}");
        assert!(text.contains("M           10 h     from p"), "{text}");
        assert!(text.contains("XS          2 h      default"), "{text}");
    }
}

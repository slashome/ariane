use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use yaml_serde::{Mapping, Value};

use super::finding::{Check, Finding, Remedy, Severity};
use crate::item::{Header, Kind, SCHEMA_VERSION};
use crate::node::{Effective, FILE as NODE_FILE, FieldDecl, FieldType};
use crate::tree::{Item, NodePath, Tree};

const SIZES: [&str; 5] = ["XS", "S", "M", "L", "XL"];
const LEVELS: [&str; 3] = ["low", "medium", "high"];
const REFLEXION_STATUSES: [&str; 2] = ["open", "settled"];
const CORE_KEYS: [&str; 16] = [
    "ariane",
    "node",
    "id",
    "title",
    "status",
    "type",
    "parent",
    "depends_on",
    "size",
    "impact",
    "priority",
    "created",
    "release",
    "milestone",
    "refs",
    "external",
];

pub fn findings(tree: &Tree, scope: Option<&NodePath>) -> Vec<Finding> {
    let mut found = Vec::new();
    structural(tree, scope, &mut found);
    let known = known_ids(tree);
    for item in tree.items_within(scope) {
        check_item(tree, item, &known, &mut found);
    }
    duplicates(tree, scope, &mut found);
    found
}

fn structural(tree: &Tree, scope: Option<&NodePath>, found: &mut Vec<Finding>) {
    let scope_dir = scope.map(|scope| tree.node_dir(scope));
    let in_scope = |path: &Path| scope_dir.as_ref().is_none_or(|dir| path.starts_with(dir));
    for (path, reason) in &tree.unreadable {
        if in_scope(path) {
            found.push(Finding {
                check: Check::Unreadable,
                severity: Severity::Error,
                node: None,
                path: Some(relative(tree, path)),
                message: format!("cannot be read: {reason}"),
                remedy: Remedy::human([
                    "rename the path so that it is valid UTF-8",
                    "fix the file's permissions or encoding",
                ]),
            });
        }
    }
    for (path, reason) in &tree.configs.errors {
        if in_scope(path) {
            found.push(Finding {
                check: Check::InvalidNodeToml,
                severity: Severity::Error,
                node: None,
                path: Some(relative(tree, path)),
                message: format!("invalid {NODE_FILE}: {}", reason.trim()),
                remedy: Remedy::human([
                    format!("fix the {NODE_FILE} so that it matches SPEC.md §1.5"),
                    format!("delete the {NODE_FILE} to fall back on the inherited configuration"),
                ]),
            });
        }
    }
    for node in &tree.nodes {
        if node.has_index || scope.is_some_and(|scope| !node.path.is_within(scope)) {
            continue;
        }
        found.push(Finding {
            check: Check::MissingIndex,
            severity: Severity::Warning,
            node: Some(node.path.clone()),
            path: None,
            message: "the node has items but no INDEX.md".to_owned(),
            remedy: Remedy::human([
                "write an INDEX.md for this node",
                "move its items to the node they belong to",
            ]),
        });
    }
}

fn check_item(
    tree: &Tree,
    item: &Item,
    known: &HashSet<(NodePath, String)>,
    found: &mut Vec<Finding>,
) {
    let mut report = Reporter { tree, item, found };
    let mapping = match &item.document.header {
        Header::Missing => {
            report.error(
                Check::NoHeader,
                "no front-matter",
                Remedy::migrate(&item.node),
            );
            return;
        }
        Header::Malformed(reason) => {
            report.error(
                Check::MalformedHeader,
                format!("the front-matter cannot be read: {reason}"),
                Remedy::human([
                    "fix the YAML of the front-matter by hand",
                    "remove the front-matter and run `ariane migrate`",
                ]),
            );
            return;
        }
        Header::Present(mapping) => mapping,
    };
    match mapping.get("ariane") {
        None => {
            report.error(
                Check::NoHeader,
                "the `ariane` key is absent",
                Remedy::migrate(&item.node),
            );
            return;
        }
        Some(value) => match value.as_u64() {
            None => {
                report.error(
                    Check::WrongType,
                    "`ariane` is not a non-negative integer",
                    Remedy::human([format!("set `ariane: {SCHEMA_VERSION}` if the header already follows the specification"), "remove the key and run `ariane migrate`".to_owned()]),
                );
                return;
            }
            Some(version) if version < SCHEMA_VERSION => {
                report.error(
                    Check::OutdatedSchema,
                    format!("schema version {version}, the CLI implements {SCHEMA_VERSION}"),
                    Remedy::migrate(&item.node),
                );
                return;
            }
            Some(version) if version > SCHEMA_VERSION => {
                report.error(
                    Check::FutureSchema,
                    format!("schema version {version} is newer than the {SCHEMA_VERSION} this CLI implements"),
                    Remedy::human(["upgrade the ariane CLI"]),
                );
                return;
            }
            Some(_) => {}
        },
    }
    let effective = tree.configs.effective(&item.node);
    report.required(mapping);
    report.node(mapping);
    report.id(mapping);
    report.status(mapping, &effective);
    report.references(mapping, known);
    report.vocabulary(mapping);
    report.core_types(mapping);
    report.custom(mapping, &effective);
}

struct Reporter<'a> {
    tree: &'a Tree,
    item: &'a Item,
    found: &'a mut Vec<Finding>,
}

impl Reporter<'_> {
    fn push(
        &mut self,
        check: Check,
        severity: Severity,
        message: impl Into<String>,
        remedy: Remedy,
    ) {
        self.found.push(Finding {
            check,
            severity,
            node: Some(self.item.node.clone()),
            path: Some(relative(self.tree, &self.item.file)),
            message: message.into(),
            remedy,
        });
    }

    fn error(&mut self, check: Check, message: impl Into<String>, remedy: Remedy) {
        self.push(check, Severity::Error, message, remedy);
    }

    fn requires_status(&self) -> bool {
        matches!(
            self.item.kind,
            Kind::Task | Kind::Story | Kind::Backlog | Kind::Reflexion
        )
    }

    fn required(&mut self, mapping: &Mapping) {
        let mut keys = vec!["node", "id", "title"];
        if self.requires_status() {
            keys.push("status");
        }
        for key in keys {
            if mapping.get(key).is_none() {
                self.error(
                    Check::MissingKey,
                    format!("the required key `{key}` is missing"),
                    Remedy::human([format!("add `{key}` to the front-matter")]),
                );
            } else if mapping
                .get(key)
                .and_then(Value::as_str)
                .is_none_or(|text| text.trim().is_empty())
            {
                self.error(
                    Check::WrongType,
                    format!("`{key}` must be a non-empty string"),
                    Remedy::human([format!("write `{key}` as a non-empty string")]),
                );
            }
        }
    }

    fn node(&mut self, mapping: &Mapping) {
        let Some(declared) = mapping.get("node").and_then(Value::as_str) else {
            return;
        };
        let actual = self.item.node.as_str();
        if declared != actual {
            self.error(
                Check::NodeMismatch,
                format!(
                    "`node: {declared}` but the file lives in `{}`",
                    self.item.node
                ),
                Remedy::human([
                    format!("set `node: {actual}`"),
                    format!("move the file under tree/{declared}"),
                ]),
            );
        }
    }

    fn id(&mut self, mapping: &Mapping) {
        let Some(id) = mapping.get("id").and_then(Value::as_str) else {
            return;
        };
        if !id.trim().is_empty() && !is_valid_id(id) {
            self.error(
                Check::InvalidId,
                format!("`id: {id}` does not match ^[A-Za-z][A-Za-z0-9-]*$"),
                Remedy::human(["choose an id that starts with a letter and contains only letters, digits and `-`"]),
            );
        }
    }

    fn status(&mut self, mapping: &Mapping, effective: &Effective) {
        let status = mapping.get("status");
        if self.item.kind == Kind::Ressource {
            if status.is_some() {
                self.error(
                    Check::StatusOnRessource,
                    "a ressource carries a `status`",
                    Remedy::human([
                        "remove `status` from the front-matter",
                        "move the file out of ressources/ if it is work to do",
                    ]),
                );
            }
            return;
        }
        let Some(status) = status.and_then(Value::as_str) else {
            return;
        };
        if self.item.kind == Kind::Reflexion {
            if !REFLEXION_STATUSES.contains(&status) {
                self.error(
                    Check::UnknownStatus,
                    format!("`status: {status}` is not a reflexion status"),
                    Remedy::human(["set `status: open`", "set `status: settled`"]),
                );
            } else if self.item.archived && status != "settled" {
                self.open_in_archives(status);
            }
            return;
        }
        match effective.status(status) {
            None => {
                let ids: Vec<&str> = effective
                    .workflow
                    .iter()
                    .map(|status| status.id.as_str())
                    .collect();
                let origin = effective.workflow_from.as_ref().map_or_else(
                    || "the node's node.toml".to_owned(),
                    |node| format!("tree/{}/{NODE_FILE}", node.as_str()),
                );
                self.error(
                    Check::UnknownStatus,
                    format!(
                        "`status: {status}` is not in the workflow ({})",
                        ids.join(", ")
                    ),
                    Remedy::human([
                        format!("set `status` to one of: {}", ids.join(", ")),
                        format!("add `{status}` to the workflow in {origin}"),
                    ]),
                );
            }
            Some(found) if self.item.archived && !found.category.is_closed() => {
                self.open_in_archives(status);
            }
            Some(_) => {}
        }
    }

    fn open_in_archives(&mut self, status: &str) {
        self.error(
            Check::OpenInArchives,
            format!("archived with `status: {status}`, which is not closed"),
            Remedy::human([
                "set a status whose category is done or dropped",
                "move the file out of archives/",
            ]),
        );
    }

    fn references(&mut self, mapping: &Mapping, known: &HashSet<(NodePath, String)>) {
        if let Some(parent) = mapping.get("parent") {
            match parent.as_str() {
                Some(raw) => self.reference("parent", raw, known),
                None => self.wrong_type("parent", "a reference string"),
            }
        }
        if let Some(depends_on) = mapping.get("depends_on") {
            match string_list(depends_on) {
                Some(list) => {
                    for raw in list {
                        self.reference("depends_on", raw, known);
                    }
                }
                None => self.wrong_type("depends_on", "a list of reference strings"),
            }
        }
    }

    fn reference(&mut self, key: &str, raw: &str, known: &HashSet<(NodePath, String)>) {
        let Some(target) = parse_reference(raw, &self.item.node) else {
            self.error(
                Check::DanglingReference,
                format!("`{key}: {raw}` is not a valid reference"),
                Remedy::human(["write it as `<id>` or `<node>#<id>`"]),
            );
            return;
        };
        if !known.contains(&target) {
            self.error(
                Check::DanglingReference,
                format!("`{key}: {raw}` points to no existing item"),
                Remedy::human([
                    format!("fix the reference in `{key}`"),
                    format!("remove it from `{key}`"),
                ]),
            );
        }
    }

    fn vocabulary(&mut self, mapping: &Mapping) {
        for (key, allowed) in [
            ("size", &SIZES[..]),
            ("impact", &LEVELS[..]),
            ("priority", &LEVELS[..]),
        ] {
            let Some(value) = mapping.get(key) else {
                continue;
            };
            if value.as_str().is_none_or(|text| !allowed.contains(&text)) {
                self.error(
                    Check::OutOfVocabulary,
                    format!(
                        "`{key}: {}` is outside {}",
                        display(value),
                        allowed.join(", ")
                    ),
                    Remedy::human([
                        format!("set `{key}` to one of: {}", allowed.join(", ")),
                        format!("remove `{key}`"),
                    ]),
                );
            }
        }
        if let Some(created) = mapping.get("created")
            && created.as_str().is_none_or(|text| !is_date(text))
        {
            self.error(
                Check::OutOfVocabulary,
                format!("`created: {}` is not a YYYY-MM-DD date", display(created)),
                Remedy::human(["write `created` as YYYY-MM-DD", "remove `created`"]),
            );
        }
    }

    fn core_types(&mut self, mapping: &Mapping) {
        for key in ["release", "milestone"] {
            if mapping.get(key).is_some_and(|value| !value.is_string()) {
                self.wrong_type(key, "a string (quote numbers such as \"1.0\")");
            }
        }
        if mapping
            .get("refs")
            .is_some_and(|value| string_list(value).is_none())
        {
            self.wrong_type("refs", "a list of strings");
        }
        if let Some(external) = mapping.get("external") {
            match string_list(external) {
                None => self.wrong_type("external", "a list of `<connector>:<id>` strings"),
                Some(list) => {
                    for entry in list {
                        if !is_external(entry) {
                            self.error(
                                Check::WrongType,
                                format!("`external` entry `{entry}` is not of the form `<connector>:<id>`"),
                                Remedy::human(["write it as `<connector>:<id>`, e.g. `monday:2955884643`"]),
                            );
                        }
                    }
                }
            }
        }
    }

    fn custom(&mut self, mapping: &Mapping, effective: &Effective) {
        let declarations: BTreeMap<&str, &FieldDecl> = effective
            .fields
            .iter()
            .map(|(key, decl)| (key.as_str(), decl))
            .collect();
        for (key, value) in mapping {
            let Some(key) = key.as_str() else {
                self.wrong_type(&display(key), "a string key");
                continue;
            };
            match declarations.get(key) {
                Some(decl) => {
                    if let Some(expected) = mismatch(decl, value, &self.item.node) {
                        self.error(
                            Check::WrongType,
                            format!("`{key}: {}` should be {expected}", display(value)),
                            Remedy::human([
                                format!("write `{key}` as {expected}"),
                                format!("change the declaration of `{key}` in {NODE_FILE}"),
                            ]),
                        );
                    }
                }
                None if !CORE_KEYS.contains(&key) => self.push(
                    Check::UndeclaredKey,
                    Severity::Warning,
                    format!("`{key}` is not declared in any applicable {NODE_FILE}"),
                    Remedy::human([
                        format!("declare `[fields.{key}]` in the node's {NODE_FILE}"),
                        format!("remove `{key}` from the front-matter"),
                    ]),
                ),
                None => {}
            }
        }
        for (key, decl) in &declarations {
            if decl.required && mapping.get(*key).is_none() {
                self.error(
                    Check::MissingKey,
                    format!("the required custom key `{key}` is missing"),
                    Remedy::human([
                        format!("add `{key}` to the front-matter"),
                        format!("make `{key}` optional in {NODE_FILE}"),
                    ]),
                );
            }
        }
    }

    fn wrong_type(&mut self, key: &str, expected: &str) {
        self.error(
            Check::WrongType,
            format!("`{key}` should be {expected}"),
            Remedy::human([
                format!("write `{key}` as {expected}"),
                format!("remove `{key}`"),
            ]),
        );
    }
}

fn duplicates(tree: &Tree, scope: Option<&NodePath>, found: &mut Vec<Finding>) {
    let mut by_id: BTreeMap<(&NodePath, &str), Vec<&Item>> = BTreeMap::new();
    for item in tree.items_within(scope) {
        if item.document.version().is_none() {
            continue;
        }
        if let Some(id) = item.document.text("id") {
            by_id.entry((&item.node, id)).or_default().push(item);
        }
    }
    for ((node, id), items) in by_id {
        if items.len() < 2 {
            continue;
        }
        let paths: Vec<String> = items
            .iter()
            .map(|item| relative(tree, &item.file))
            .collect();
        for item in &items {
            found.push(Finding {
                check: Check::DuplicateId,
                severity: Severity::Error,
                node: Some(node.clone()),
                path: Some(relative(tree, &item.file)),
                message: format!("`id: {id}` is used by {} items in this node", items.len()),
                remedy: Remedy::human(paths.iter().map(|path| format!("give {path} a new id"))),
            });
        }
    }
}

fn known_ids(tree: &Tree) -> HashSet<(NodePath, String)> {
    tree.items
        .iter()
        .filter_map(|item| Some((item.node.clone(), item.document.text("id")?.to_owned())))
        .collect()
}

fn mismatch(decl: &FieldDecl, value: &Value, node: &NodePath) -> Option<String> {
    let ok = match decl.kind {
        FieldType::String => value.is_string(),
        FieldType::Integer => value.is_i64() || value.is_u64(),
        FieldType::Number => value.is_number(),
        FieldType::Boolean => value.is_bool(),
        FieldType::Date => value.as_str().is_some_and(is_date),
        FieldType::Ref => value
            .as_str()
            .is_some_and(|raw| parse_reference(raw, node).is_some()),
        FieldType::ListOfStrings => string_list(value).is_some(),
        FieldType::ListOfRefs => string_list(value)
            .is_some_and(|list| list.iter().all(|raw| parse_reference(raw, node).is_some())),
        FieldType::Enum => value
            .as_str()
            .is_some_and(|text| decl.values.iter().any(|allowed| allowed == text)),
    };
    if ok {
        return None;
    }
    Some(match decl.kind {
        FieldType::String => "a string".to_owned(),
        FieldType::Integer => "an integer".to_owned(),
        FieldType::Number => "a number".to_owned(),
        FieldType::Boolean => "a boolean".to_owned(),
        FieldType::Date => "a YYYY-MM-DD date".to_owned(),
        FieldType::Ref => "a reference".to_owned(),
        FieldType::ListOfStrings => "a list of strings".to_owned(),
        FieldType::ListOfRefs => "a list of references".to_owned(),
        FieldType::Enum => format!("one of: {}", decl.values.join(", ")),
    })
}

pub fn parse_reference(raw: &str, current: &NodePath) -> Option<(NodePath, String)> {
    let (node, id) = match raw.split_once('#') {
        Some((node, id)) => (NodePath::new(node), id),
        None => (current.clone(), raw),
    };
    is_valid_id(id).then(|| (node, id.to_owned()))
}

pub fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| text[range].parse::<u32>().ok();
    let (Some(_), Some(month), Some(day)) = (digits(0..4), digits(5..7), digits(8..10)) else {
        return false;
    };
    text[..4].bytes().all(|b| b.is_ascii_digit())
        && (1..=12).contains(&month)
        && (1..=31).contains(&day)
}

fn is_external(entry: &str) -> bool {
    entry.split_once(':').is_some_and(|(connector, id)| {
        !connector.is_empty()
            && !id.is_empty()
            && connector
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
}

fn string_list(value: &Value) -> Option<Vec<&str>> {
    value.as_sequence()?.iter().map(Value::as_str).collect()
}

fn display(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => yaml_serde::to_string(other)
            .map(|text| text.trim_end().to_owned())
            .unwrap_or_else(|_| "?".to_owned()),
    }
}

fn relative(tree: &Tree, path: &Path) -> String {
    path.strip_prefix(&tree.root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

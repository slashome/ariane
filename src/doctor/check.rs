use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use yaml_serde::{Mapping, Value};

use super::finding::{Check, Finding, Remedy, Severity};
use crate::item::{Header, SCHEMA_VERSION};
use crate::node::{Effective, FILE as NODE_FILE, FieldDecl, FieldType};
use crate::schema::{
    self, CORE_KEYS, CoreKey, REFLEXION_CLOSED, REFLEXION_STATUSES, Requirement, ValueType,
    is_date, is_external, is_valid_id,
};
use crate::tree::{Item, NodePath, Tree};

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
    for key in CORE_KEYS
        .iter()
        .filter(|key| key.value != ValueType::Version)
    {
        report.core_key(key, mapping, known);
    }
    report.node(mapping);
    report.status(mapping, &effective);
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

    fn core_key(&mut self, key: &CoreKey, mapping: &Mapping, known: &HashSet<(NodePath, String)>) {
        let requirement = key.requirement_for(self.item.kind);
        let Some(value) = mapping.get(key.name) else {
            if requirement == Requirement::Must {
                self.error(
                    Check::MissingKey,
                    format!("the required key `{}` is missing", key.name),
                    Remedy::human([format!("add `{}` to the front-matter", key.name)]),
                );
            }
            return;
        };
        if requirement == Requirement::MustNot {
            self.forbidden(key.name);
            return;
        }
        self.value(key, value, known);
    }

    fn forbidden(&mut self, name: &str) {
        let kind = kind_label(self.item);
        self.error(
            Check::ForbiddenKey,
            format!("a {kind} carries `{name}`, which its kind does not allow"),
            Remedy::human([
                format!("remove `{name}` from the front-matter"),
                format!("move the file to the directory of a kind that allows `{name}`"),
            ]),
        );
    }

    fn value(&mut self, key: &CoreKey, value: &Value, known: &HashSet<(NodePath, String)>) {
        let name = key.name;
        match key.value {
            ValueType::Version => {}
            ValueType::Text | ValueType::Status => {
                if non_empty(value).is_none() {
                    self.non_empty(name);
                }
            }
            ValueType::Id => match non_empty(value) {
                None => self.non_empty(name),
                Some(id) if !is_valid_id(id) => self.error(
                    Check::InvalidId,
                    format!("`id: {id}` does not match ^[A-Za-z][A-Za-z0-9-]*$"),
                    Remedy::human(["choose an id that starts with a letter and contains only letters, digits and `-`"]),
                ),
                Some(_) => {}
            },
            ValueType::Name | ValueType::Texts => {
                let ok = match key.value {
                    ValueType::Name => value.is_string(),
                    _ => string_list(value).is_some(),
                };
                if !ok {
                    self.wrong_type(name, key.value.expected());
                }
            }
            ValueType::Date => {
                if value.as_str().is_none_or(|text| !is_date(text)) {
                    self.out_of_vocabulary(
                        format!("`{name}: {}` is not a YYYY-MM-DD date", display(value)),
                        [format!("write `{name}` as YYYY-MM-DD"), format!("remove `{name}`")],
                    );
                }
            }
            ValueType::Enum(allowed) => {
                if value.as_str().is_none_or(|text| !allowed.contains(&text)) {
                    self.out_of_vocabulary(
                        format!("`{name}: {}` is outside {}", display(value), allowed.join(", ")),
                        [
                            format!("set `{name}` to one of: {}", allowed.join(", ")),
                            format!("remove `{name}`"),
                        ],
                    );
                }
            }
            ValueType::Reference => match value.as_str() {
                Some(raw) => self.reference(name, raw, known),
                None => self.wrong_type(name, key.value.expected()),
            },
            ValueType::References => match string_list(value) {
                Some(list) => {
                    for raw in list {
                        self.reference(name, raw, known);
                    }
                }
                None => self.wrong_type(name, key.value.expected()),
            },
            ValueType::External => match string_list(value) {
                None => self.wrong_type(name, key.value.expected()),
                Some(list) => {
                    for entry in list.into_iter().filter(|entry| !is_external(entry)) {
                        self.error(
                            Check::WrongType,
                            format!("`{name}` entry `{entry}` is not of the form `<connector>:<id>`"),
                            Remedy::human(["write it as `<connector>:<id>`, e.g. `monday:2955884643`"]),
                        );
                    }
                }
            },
        }
    }

    fn non_empty(&mut self, name: &str) {
        self.error(
            Check::WrongType,
            format!("`{name}` must be a non-empty string"),
            Remedy::human([format!("write `{name}` as a non-empty string")]),
        );
    }

    fn out_of_vocabulary<const N: usize>(&mut self, message: String, resolutions: [String; N]) {
        self.error(Check::OutOfVocabulary, message, Remedy::human(resolutions));
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

    fn status(&mut self, mapping: &Mapping, effective: &Effective) {
        let status_key = schema::core_key("status");
        if status_key.is_some_and(|key| key.requirement_for(self.item.kind) == Requirement::MustNot)
        {
            return;
        }
        let Some(status) = mapping.get("status").and_then(Value::as_str) else {
            return;
        };
        if self.item.kind == crate::item::Kind::Reflexion {
            if !REFLEXION_STATUSES.contains(&status) {
                self.error(
                    Check::UnknownStatus,
                    format!("`status: {status}` is not a reflexion status"),
                    Remedy::human(
                        REFLEXION_STATUSES
                            .iter()
                            .map(|allowed| format!("set `status: {allowed}`")),
                    ),
                );
            } else if self.item.archived && status != REFLEXION_CLOSED {
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
                None if !schema::is_core(key) => self.push(
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

fn non_empty(value: &Value) -> Option<&str> {
    value.as_str().filter(|text| !text.trim().is_empty())
}

fn kind_label(item: &Item) -> String {
    serde_json::to_value(item.kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "item".to_owned())
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

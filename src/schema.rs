//! The core header schema, declared once as data (SPEC.md §1.3, ADR-0008).
//!
//! [`CORE_KEYS`](crate::schema::CORE_KEYS) describes each of the sixteen core keys a single time: its value
//! type, whether it is required depending on the item kind, and its place in the
//! canonical key order. Everything else reads this table instead of keeping its
//! own list:
//!
//! - `ariane doctor` validates every item by iterating it;
//! - `ariane migrate` takes from it the order in which keys are written;
//! - `ariane schema --json` exports it, so that the interface never redeclares a
//!   vocabulary in TypeScript.
//!
//! The fixed vocabularies (sizes, levels, reflexion statuses), the id prefixes by
//! kind and the small format checks shared by those commands live here too.

use serde::Serialize;
use serde_json::{Map, Value as Json, json};

use crate::item::{Kind, SCHEMA_VERSION};
use crate::node::Category;

/// Allowed values of `size`, smallest first.
pub const SIZES: &[&str] = &["XS", "S", "M", "L", "XL"];
/// Allowed values of `impact` and `priority`.
pub const LEVELS: &[&str] = &["low", "medium", "high"];
/// Statuses of a reflexion: open, being tried (`trial`), or settled.
pub const REFLEXION_STATUSES: &[&str] = &["open", "trial", "settled"];
/// The only reflexion status that counts as closed, for archived reflexions.
pub const REFLEXION_CLOSED: &str = "settled";

/// How strongly an item must carry a key, in RFC 2119 terms (SPEC.md §1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    /// The key is required.
    Must,
    /// The key is recommended; its absence is not an error.
    Should,
    /// The key is optional.
    May,
    /// The key is forbidden, such as `status` on a ressource.
    MustNot,
}

/// The shape a core key's value must have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    /// A non-negative integer: the schema version.
    Version,
    /// A non-empty string.
    Text,
    /// The logical path of a node: a string, empty for the home node.
    Node,
    /// A string that YAML could read as a number, such as `"1.0"`; it must be quoted.
    Name,
    /// An item identifier such as `T038`.
    Id,
    /// A status id, checked against the node's workflow elsewhere.
    Status,
    /// A `YYYY-MM-DD` date.
    Date,
    /// One item reference: `T022` or `node/path#T022`.
    Reference,
    /// A list of item references.
    References,
    /// A list of strings.
    Texts,
    /// A list of `<connector>:<id>` links to external trackers.
    External,
    /// One string out of a fixed vocabulary. `&'static` means the vocabulary is
    /// compiled into the binary and lives for the whole run.
    Enum(&'static [&'static str]),
}

impl ValueType {
    /// Name of the type in `ariane schema` output.
    pub fn id(self) -> &'static str {
        match self {
            ValueType::Version => "integer",
            ValueType::Text => "text",
            ValueType::Node => "node",
            ValueType::Name => "name",
            ValueType::Id => "id",
            ValueType::Status => "status",
            ValueType::Date => "date",
            ValueType::Reference => "reference",
            ValueType::References => "list<reference>",
            ValueType::Texts => "list<string>",
            ValueType::External => "list<external>",
            ValueType::Enum(_) => "enum",
        }
    }

    /// What a valid value looks like, for doctor's messages.
    pub fn expected(self) -> &'static str {
        match self {
            ValueType::Version => "a non-negative integer",
            ValueType::Node => "a string, empty for the home node",
            ValueType::Text | ValueType::Id | ValueType::Status => "a non-empty string",
            ValueType::Name => "a string (quote numbers such as \"1.0\")",
            ValueType::Date => "a YYYY-MM-DD date",
            ValueType::Reference => "a reference string",
            ValueType::References => "a list of reference strings",
            ValueType::Texts => "a list of strings",
            ValueType::External => "a list of `<connector>:<id>` strings",
            ValueType::Enum(_) => "one of its vocabulary",
        }
    }
}

/// One core key of the header.
#[derive(Debug, Clone, Copy)]
pub struct CoreKey {
    /// The key as written in the front-matter.
    pub name: &'static str,
    /// The shape its value must have.
    pub value: ValueType,
    /// Its requirement for any kind not listed in `by_kind`.
    pub requirement: Requirement,
    /// Requirements that differ for some kinds.
    pub by_kind: &'static [(Kind, Requirement)],
}

impl CoreKey {
    /// The requirement of this key for an item of `kind`: the `by_kind` override if there is one, the default otherwise.
    pub fn requirement_for(&self, kind: Kind) -> Requirement {
        self.by_kind
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .map_or(self.requirement, |(_, requirement)| *requirement)
    }
}

/// Builds a core key with the same requirement for every kind. `const fn` lets
/// it run at compile time, inside the [`CORE_KEYS`] constant.
const fn key(name: &'static str, value: ValueType, requirement: Requirement) -> CoreKey {
    CoreKey {
        name,
        value,
        requirement,
        by_kind: &[],
    }
}

/// Every core key, in the canonical order in which `migrate` writes them.
pub const CORE_KEYS: &[CoreKey] = &[
    key("ariane", ValueType::Version, Requirement::Must),
    key("node", ValueType::Node, Requirement::Must),
    key("id", ValueType::Id, Requirement::Must),
    key("title", ValueType::Text, Requirement::Must),
    CoreKey {
        name: "status",
        value: ValueType::Status,
        requirement: Requirement::May,
        by_kind: &[
            (Kind::Task, Requirement::Must),
            (Kind::Story, Requirement::Must),
            (Kind::Backlog, Requirement::Must),
            (Kind::Reflexion, Requirement::Must),
            (Kind::Ressource, Requirement::MustNot),
        ],
    },
    key("type", ValueType::Text, Requirement::May),
    key("parent", ValueType::Reference, Requirement::May),
    key("depends_on", ValueType::References, Requirement::May),
    key("size", ValueType::Enum(SIZES), Requirement::May),
    key("impact", ValueType::Enum(LEVELS), Requirement::May),
    key("priority", ValueType::Enum(LEVELS), Requirement::May),
    key("created", ValueType::Date, Requirement::Should),
    key("release", ValueType::Name, Requirement::May),
    key("milestone", ValueType::Name, Requirement::May),
    key("refs", ValueType::Texts, Requirement::May),
    key("external", ValueType::External, Requirement::May),
];

/// Every item kind, in the order `ariane schema --json` lists them.
pub const KINDS: [Kind; 6] = [
    Kind::Task,
    Kind::Story,
    Kind::Backlog,
    Kind::Reflexion,
    Kind::Ressource,
    Kind::Unknown,
];

/// The core key named `name`, if it is one.
pub fn core_key(name: &str) -> Option<&'static CoreKey> {
    CORE_KEYS.iter().find(|key| key.name == name)
}

/// Tells whether `name` is a core key; any other key is a custom key.
pub fn is_core(name: &str) -> bool {
    core_key(name).is_some()
}

/// The names of the core keys whose value has the type `value`, lazily.
pub fn keys_of_type(value: ValueType) -> impl Iterator<Item = &'static str> {
    CORE_KEYS
        .iter()
        .filter(move |key| key.value == value)
        .map(|key| key.name)
}

/// Letter that starts a new id for an item of `kind`: T task or backlog, S story, R reflexion, D ressource (SPEC.md §1.4).
pub fn id_prefix(kind: Kind) -> char {
    match kind {
        Kind::Story => 'S',
        Kind::Reflexion => 'R',
        Kind::Ressource => 'D',
        Kind::Task | Kind::Backlog | Kind::Unknown => 'T',
    }
}

/// Tells whether `id` matches `^[A-Za-z][A-Za-z0-9-]*$` (SPEC.md §1.3).
pub fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Tells whether `text` is a `YYYY-MM-DD` date with a month from 1 to 12 and a
/// day from 1 to 31. It does not check that the day exists in that month.
pub fn is_date(text: &str) -> bool {
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

/// Tells whether `entry` is a `<connector>:<id>` link, the connector written in lower case.
pub fn is_external(entry: &str) -> bool {
    entry.split_once(':').is_some_and(|(connector, id)| {
        !connector.is_empty()
            && !id.is_empty()
            && connector
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
}

/// The schema as JSON, as printed by `ariane schema --json`: version, keys with
/// their type, requirement per kind and vocabulary, status categories,
/// reflexion statuses and id prefixes.
pub fn export() -> Json {
    let keys: Vec<Json> = CORE_KEYS
        .iter()
        .map(|key| {
            let by_kind: Map<String, Json> = KINDS
                .iter()
                .map(|kind| (kind_id(*kind), json!(key.requirement_for(*kind))))
                .collect();
            let mut entry = json!({
                "name": key.name,
                "type": key.value.id(),
                "requirement": by_kind,
            });
            if let ValueType::Enum(values) = key.value {
                entry["values"] = json!(values);
            }
            entry
        })
        .collect();
    let categories: Vec<Json> = Category::ALL
        .iter()
        .map(|category| json!({"id": category.id(), "label": category.label()}))
        .collect();
    let prefixes: Map<String, Json> = KINDS
        .iter()
        .map(|kind| (kind_id(*kind), json!(id_prefix(*kind).to_string())))
        .collect();
    json!({
        "version": SCHEMA_VERSION,
        "keys": keys,
        "categories": categories,
        "reflexion_statuses": REFLEXION_STATUSES,
        "id_prefixes": prefixes,
    })
}

/// The name of a kind as serialized in JSON, such as `ressource`.
fn kind_id(kind: Kind) -> String {
    match serde_json::to_value(kind) {
        Ok(Json::String(id)) => id,
        _ => format!("{kind:?}").to_lowercase(),
    }
}

/// Unit tests of the schema table and its export.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_core_key_is_declared_once_in_specification_order() {
        let names: Vec<_> = CORE_KEYS.iter().map(|key| key.name).collect();
        assert_eq!(
            names,
            [
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
            ]
        );
    }

    #[test]
    fn status_is_required_for_work_items_and_forbidden_on_ressources() {
        let status = core_key("status").unwrap();
        assert_eq!(status.requirement_for(Kind::Task), Requirement::Must);
        assert_eq!(status.requirement_for(Kind::Reflexion), Requirement::Must);
        assert_eq!(
            status.requirement_for(Kind::Ressource),
            Requirement::MustNot
        );
        assert_eq!(status.requirement_for(Kind::Unknown), Requirement::May);
    }

    #[test]
    fn size_and_impact_are_optional_enums() {
        for name in ["size", "impact"] {
            let key = core_key(name).unwrap();
            assert_eq!(key.requirement_for(Kind::Task), Requirement::May);
            assert!(matches!(key.value, ValueType::Enum(_)));
        }
    }

    #[test]
    fn ids_take_a_prefix_by_kind() {
        assert_eq!(id_prefix(Kind::Task), 'T');
        assert_eq!(id_prefix(Kind::Backlog), 'T');
        assert_eq!(id_prefix(Kind::Story), 'S');
        assert_eq!(id_prefix(Kind::Reflexion), 'R');
        assert_eq!(id_prefix(Kind::Ressource), 'D');
        assert_eq!(id_prefix(Kind::Unknown), 'T');
    }

    #[test]
    fn the_export_carries_keys_vocabularies_categories_and_prefixes() {
        let exported = export();
        assert_eq!(exported["version"], json!(SCHEMA_VERSION));
        assert_eq!(exported["keys"].as_array().unwrap().len(), CORE_KEYS.len());
        let size = exported["keys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|key| key["name"] == "size")
            .unwrap();
        assert_eq!(size["values"], json!(SIZES));
        assert_eq!(size["requirement"]["task"], json!("may"));
        let status = exported["keys"]
            .as_array()
            .unwrap()
            .iter()
            .find(|key| key["name"] == "status")
            .unwrap();
        assert_eq!(status["requirement"]["ressource"], json!("must_not"));
        assert_eq!(exported["categories"].as_array().unwrap().len(), 6);
        assert_eq!(
            exported["reflexion_statuses"],
            json!(["open", "trial", "settled"])
        );
        assert_eq!(exported["id_prefixes"]["ressource"], json!("D"));
    }

    #[test]
    fn dates_and_external_links_follow_their_formats() {
        assert!(is_date("2030-01-31"));
        assert!(!is_date("2030-13-01"));
        assert!(!is_date("30-01-2030"));
        assert!(is_external("monday:1000000001"));
        assert!(!is_external("Monday:1"));
        assert!(!is_external("monday:"));
    }
}

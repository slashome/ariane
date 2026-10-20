use serde::Serialize;
use serde_json::{Map, Value as Json, json};

use crate::item::{Kind, SCHEMA_VERSION};
use crate::node::Category;

pub const SIZES: &[&str] = &["XS", "S", "M", "L", "XL"];
pub const LEVELS: &[&str] = &["low", "medium", "high"];
pub const REFLEXION_STATUSES: &[&str] = &["open", "trial", "settled"];
pub const REFLEXION_CLOSED: &str = "settled";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    Must,
    Should,
    May,
    MustNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Version,
    Text,
    Name,
    Id,
    Status,
    Date,
    Reference,
    References,
    Texts,
    External,
    Enum(&'static [&'static str]),
}

impl ValueType {
    pub fn id(self) -> &'static str {
        match self {
            ValueType::Version => "integer",
            ValueType::Text => "text",
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

    pub fn expected(self) -> &'static str {
        match self {
            ValueType::Version => "a non-negative integer",
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

#[derive(Debug, Clone, Copy)]
pub struct CoreKey {
    pub name: &'static str,
    pub value: ValueType,
    pub requirement: Requirement,
    pub by_kind: &'static [(Kind, Requirement)],
}

impl CoreKey {
    pub fn requirement_for(&self, kind: Kind) -> Requirement {
        self.by_kind
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .map_or(self.requirement, |(_, requirement)| *requirement)
    }
}

const fn key(name: &'static str, value: ValueType, requirement: Requirement) -> CoreKey {
    CoreKey {
        name,
        value,
        requirement,
        by_kind: &[],
    }
}

pub const CORE_KEYS: &[CoreKey] = &[
    key("ariane", ValueType::Version, Requirement::Must),
    key("node", ValueType::Text, Requirement::Must),
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

pub const KINDS: [Kind; 6] = [
    Kind::Task,
    Kind::Story,
    Kind::Backlog,
    Kind::Reflexion,
    Kind::Ressource,
    Kind::Unknown,
];

pub fn core_key(name: &str) -> Option<&'static CoreKey> {
    CORE_KEYS.iter().find(|key| key.name == name)
}

pub fn is_core(name: &str) -> bool {
    core_key(name).is_some()
}

pub fn keys_of_type(value: ValueType) -> impl Iterator<Item = &'static str> {
    CORE_KEYS
        .iter()
        .filter(move |key| key.value == value)
        .map(|key| key.name)
}

pub fn id_prefix(kind: Kind) -> char {
    match kind {
        Kind::Story => 'S',
        Kind::Reflexion => 'R',
        Kind::Ressource => 'D',
        Kind::Task | Kind::Backlog | Kind::Unknown => 'T',
    }
}

pub fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

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

pub fn is_external(entry: &str) -> bool {
    entry.split_once(':').is_some_and(|(connector, id)| {
        !connector.is_empty()
            && !id.is_empty()
            && connector
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
}

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

fn kind_id(kind: Kind) -> String {
    match serde_json::to_value(kind) {
        Ok(Json::String(id)) => id,
        _ => format!("{kind:?}").to_lowercase(),
    }
}

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

//! Reading and writing one item file.
//!
//! An item is a Markdown file that starts with a YAML front-matter between two
//! `---` lines (SPEC.md §1.2). This module splits a file into that header and its
//! body, parses the header into an ordered YAML mapping, and renders it back.
//!
//! The mapping keeps every key in its original order, including keys this tool
//! does not know: SPEC.md §1.3 requires a tool to write back what it did not
//! understand unchanged.

use anyhow::Result;
use serde::Serialize;
use yaml_serde::{Mapping, Value};

/// Schema version this binary implements: the value of the `ariane` key it writes and expects.
pub const SCHEMA_VERSION: u64 = 1;

/// The former item directory of backlog items, whose items are now tasks
/// (SPEC.md §1.1): `doctor` reports them and `ariane fix` moves them into `tasks/`.
pub const LEGACY_BACKLOG: &str = "backlog";

/// The nature of an item, given by the directory it sits in, never by its header
/// (SPEC.md §1.1).
///
/// `clap::ValueEnum` lets `ariane tasks --kind` accept the same words the JSON
/// uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// An item in `tasks/`, or in the former `backlog/` directory (SPEC.md §1.1).
    Task,
    /// An item in `epics/`: a large piece of work that groups stories.
    Epic,
    /// An item in `stories/`.
    Story,
    /// An item in `reflexions/`.
    Reflexion,
    /// An item in `ressources/`: an external reference, never a status.
    Ressource,
    /// An archived item stored directly in `archives/`, whose kind cannot be told.
    Unknown,
}

impl Kind {
    /// Maps an item directory name to its kind, or `None` for any other directory.
    ///
    /// `backlog/` is a former item directory: its items are tasks waiting in a
    /// status of the `backlog` category, read as tasks until `ariane fix` moves
    /// them into `tasks/` ([`LEGACY_BACKLOG`]).
    pub fn from_dir(name: &str) -> Option<Kind> {
        match name {
            "tasks" | LEGACY_BACKLOG => Some(Kind::Task),
            "epics" => Some(Kind::Epic),
            "stories" => Some(Kind::Story),
            "reflexions" => Some(Kind::Reflexion),
            "ressources" => Some(Kind::Ressource),
            _ => None,
        }
    }
}

/// What was found at the top of an item file.
#[derive(Debug, Clone, PartialEq)]
pub enum Header {
    /// No front-matter: the file does not start with a closed `---` block.
    Missing,
    /// A front-matter that is not a YAML mapping; carries the parser's message.
    Malformed(String),
    /// A parsed front-matter, keys in file order.
    Present(Mapping),
}

/// An item file split into its header and its Markdown body.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The front-matter, if any.
    pub header: Header,
    /// Everything after the closing `---`, untouched.
    pub body: String,
}

impl Document {
    /// Splits `text` into a header and a body and parses the header.
    ///
    /// Never fails: a file without front-matter or with invalid YAML is still a
    /// document, and the problem is recorded in [`Header`] for `doctor` to report.
    pub fn parse(text: &str) -> Document {
        let Some((yaml, body)) = split(text) else {
            return Document {
                header: Header::Missing,
                body: text.to_owned(),
            };
        };
        let header = match yaml_serde::from_str::<Value>(yaml) {
            Ok(Value::Mapping(mapping)) => Header::Present(mapping),
            Ok(Value::Null) => Header::Present(Mapping::new()),
            Ok(_) => Header::Malformed("the front-matter is not a mapping".to_owned()),
            Err(error) => Header::Malformed(error.to_string()),
        };
        Document {
            header,
            body: body.to_owned(),
        }
    }

    /// The header mapping when the front-matter was parsed, `None` otherwise.
    pub fn mapping(&self) -> Option<&Mapping> {
        match &self.header {
            Header::Present(mapping) => Some(mapping),
            _ => None,
        }
    }

    /// The schema version written in the `ariane` key, if it is an integer.
    ///
    /// The `?` operator inside a function returning `Option` returns `None` as soon
    /// as one step is missing: no header, no key, or a value that is not an integer.
    pub fn version(&self) -> Option<u64> {
        self.mapping()?.get("ariane")?.as_u64()
    }

    /// The value of `key` when it is an integer, such as `rank`.
    pub fn integer(&self, key: &str) -> Option<i64> {
        self.mapping()?.get(key)?.as_i64()
    }

    /// The value of `key` when it is a string.
    ///
    /// The returned `&str` borrows from the document, so it lives as long as the
    /// document does and nothing is copied.
    pub fn text(&self, key: &str) -> Option<&str> {
        self.mapping()?.get(key)?.as_str()
    }

    /// The string entries of `key` when it is a list, such as `external`; empty
    /// when the key is missing or not a list, and entries that are not strings
    /// are skipped.
    pub fn texts(&self, key: &str) -> Vec<&str> {
        self.mapping()
            .and_then(|mapping| mapping.get(key))
            .and_then(Value::as_sequence)
            .map(|entries| entries.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }
}

/// Builds the text of an item file from a header mapping and a body.
///
/// # Errors
///
/// Fails if the mapping cannot be serialized to YAML.
pub fn render(header: &Mapping, body: &str) -> Result<String> {
    let yaml = yaml_serde::to_string(header)?;
    Ok(format!("---\n{yaml}---\n{body}"))
}

/// Writes `wanted` under `key`, or removes the key when `wanted` is `None`.
///
/// An existing key keeps its place. A new key is inserted right after the last
/// core key that precedes it in the canonical order, so custom keys stay where
/// they were. `Mapping` keeps insertion order, so the insertion rebuilds it.
pub fn set_key(header: &mut Mapping, key: &str, wanted: Option<Value>) {
    let Some(value) = wanted else {
        header.shift_remove(key);
        return;
    };
    if let Some(slot) = header.get_mut(key) {
        *slot = value;
        return;
    }
    let rank = |name: &str| {
        crate::schema::CORE_KEYS
            .iter()
            .position(|core| core.name == name)
    };
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

/// Cuts a file into its front-matter text and its body, without parsing either.
///
/// The front-matter must open on the very first line (after an optional byte
/// order mark) and close on a line made only of `---`. Both returned slices
/// borrow from `text`. Returns `None` when there is no closed front-matter.
fn split(text: &str) -> Option<(&str, &str)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Some((&rest[..offset], &rest[offset + line.len()..]));
        }
        offset += line.len();
    }
    None
}

/// Unit tests of front-matter splitting, parsing and rendering.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_without_front_matter_has_a_missing_header_and_keeps_its_text() {
        let document = Document::parse("# Title\n\nBody\n");
        assert_eq!(document.header, Header::Missing);
        assert_eq!(document.body, "# Title\n\nBody\n");
    }

    #[test]
    fn reads_the_front_matter_and_the_body() {
        let document = Document::parse("---\nariane: 1\nid: T001\n---\n# Title\n");
        assert_eq!(document.version(), Some(1));
        assert_eq!(document.text("id"), Some("T001"));
        assert_eq!(document.body, "# Title\n");
    }

    #[test]
    fn a_list_key_gives_its_strings_and_anything_else_gives_none() {
        let document = Document::parse("---\nexternal: [monday:1, 2, monday:3]\nid: T001\n---\n");
        assert_eq!(document.texts("external"), ["monday:1", "monday:3"]);
        assert!(document.texts("id").is_empty());
        assert!(document.texts("missing").is_empty());
    }

    #[test]
    fn an_unclosed_front_matter_is_not_a_front_matter() {
        let document = Document::parse("---\nid: T001\n# Title\n");
        assert_eq!(document.header, Header::Missing);
    }

    #[test]
    fn a_scalar_front_matter_is_malformed() {
        let document = Document::parse("---\njust text\n---\n");
        assert!(matches!(document.header, Header::Malformed(_)));
    }

    #[test]
    fn rendering_keeps_unknown_keys_and_their_order() {
        let text = "---\nariane: 1\nzeta: kept\nid: T001\nmod: Takedowns\n---\nBody\n";
        let document = Document::parse(text);
        let rendered = render(document.mapping().unwrap(), &document.body).unwrap();
        assert_eq!(rendered, text);
    }
}

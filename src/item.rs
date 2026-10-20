use anyhow::Result;
use serde::Serialize;
use yaml_serde::{Mapping, Value};

pub const SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Backlog,
    Task,
    Story,
    Reflexion,
    Ressource,
    Unknown,
}

impl Kind {
    pub fn from_dir(name: &str) -> Option<Kind> {
        match name {
            "backlog" => Some(Kind::Backlog),
            "tasks" => Some(Kind::Task),
            "stories" => Some(Kind::Story),
            "reflexions" => Some(Kind::Reflexion),
            "ressources" => Some(Kind::Ressource),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Header {
    Missing,
    Malformed(String),
    Present(Mapping),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub header: Header,
    pub body: String,
}

impl Document {
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

    pub fn mapping(&self) -> Option<&Mapping> {
        match &self.header {
            Header::Present(mapping) => Some(mapping),
            _ => None,
        }
    }

    pub fn version(&self) -> Option<u64> {
        self.mapping()?.get("ariane")?.as_u64()
    }

    pub fn text(&self, key: &str) -> Option<&str> {
        self.mapping()?.get(key)?.as_str()
    }
}

pub fn render(header: &Mapping, body: &str) -> Result<String> {
    let yaml = yaml_serde::to_string(header)?;
    Ok(format!("---\n{yaml}---\n{body}"))
}

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

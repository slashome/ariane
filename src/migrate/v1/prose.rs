use yaml_serde::Value;

use crate::item::Kind;
use crate::node::Category;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusValue {
    Category(Category),
    Open,
    Settled,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Extraction {
    pub status: Option<StatusValue>,
    pub fields: Vec<(String, Value)>,
    pub inferred: Vec<String>,
    pub leftovers: Vec<String>,
    pub body: String,
}

pub fn extract(_kind: Kind, body: &str) -> Extraction {
    Extraction {
        body: body.to_owned(),
        ..Extraction::default()
    }
}

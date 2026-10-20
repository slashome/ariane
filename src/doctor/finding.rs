use serde::Serialize;

use crate::tree::NodePath;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Remedy {
    Command { command: String },
    Human { resolutions: Vec<String> },
}

impl Remedy {
    pub fn migrate(node: &NodePath) -> Remedy {
        let command = if node.is_home() {
            "ariane migrate".to_owned()
        } else {
            format!("ariane migrate --node {}", node.as_str())
        };
        Remedy::Command { command }
    }

    pub fn human<I, S>(resolutions: I) -> Remedy
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Remedy::Human {
            resolutions: resolutions.into_iter().map(Into::into).collect(),
        }
    }

    pub fn is_command(&self) -> bool {
        matches!(self, Remedy::Command { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    Unreadable,
    InvalidNodeToml,
    MissingIndex,
    NoHeader,
    MalformedHeader,
    OutdatedSchema,
    FutureSchema,
    MissingKey,
    WrongType,
    NodeMismatch,
    InvalidId,
    DuplicateId,
    UnknownStatus,
    ForbiddenKey,
    OpenInArchives,
    DanglingReference,
    OutOfVocabulary,
    UndeclaredKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub check: Check,
    pub severity: Severity,
    pub node: Option<NodePath>,
    pub path: Option<String>,
    pub message: String,
    pub remedy: Remedy,
}

impl Finding {
    pub fn needs_migrate(&self) -> bool {
        matches!(self.check, Check::NoHeader | Check::OutdatedSchema)
    }
}

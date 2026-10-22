//! What `ariane doctor` reports: findings, their severity and their remedy.
//!
//! ADR-0004 sets the contract: every finding names how to repair it, either a
//! command (`ariane migrate`) or, when the decision belongs to a human, the list
//! of possible resolutions. A finding whose remedy is only "your call" is not
//! allowed.

use serde::Serialize;

use crate::tree::NodePath;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Makes doctor fail.
    Error,
    /// Reported; fails only with `--strict`.
    Warning,
}

impl Severity {
    /// The word printed in front of the finding.
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// How to repair a finding. `#[serde(tag = "kind")]` serializes it as an object
/// with a `kind` field (`command` or `human`) next to its data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Remedy {
    /// A command repairs it on its own.
    Command { command: String },
    /// A human must choose among these resolutions.
    Human { resolutions: Vec<String> },
}

impl Remedy {
    /// The remedy `ariane migrate`, scoped to `node` unless it is the home node.
    pub fn migrate(node: &NodePath) -> Remedy {
        let command = if node.is_home() {
            "ariane migrate".to_owned()
        } else {
            format!("ariane migrate --node {}", node.as_str())
        };
        Remedy::Command { command }
    }

    /// A human remedy built from any list of strings.
    ///
    /// The generic parameters accept an array of `&str`, a `Vec<String>`, or any
    /// other iterable of things convertible into `String`.
    pub fn human<I, S>(resolutions: I) -> Remedy
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Remedy::Human {
            resolutions: resolutions.into_iter().map(Into::into).collect(),
        }
    }

    /// Tells whether a command repairs the finding without a human.
    pub fn is_command(&self) -> bool {
        matches!(self, Remedy::Command { .. })
    }
}

/// Which rule a finding comes from, one per row of SPEC.md §1.6 plus the structural checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    /// A file or a name that could not be read.
    Unreadable,
    /// A `node.toml` that does not parse.
    InvalidNodeToml,
    /// A node with items but no `INDEX.md`.
    MissingIndex,
    /// No front-matter, or no `ariane` key: the file predates the schema.
    NoHeader,
    /// A front-matter that is not valid YAML.
    MalformedHeader,
    /// A schema version older than the binary's.
    OutdatedSchema,
    /// A schema version newer than the binary's: the CLI is outdated.
    FutureSchema,
    /// A required key is absent.
    MissingKey,
    /// A key whose value has the wrong shape.
    WrongType,
    /// `node` disagrees with where the file sits.
    NodeMismatch,
    /// An `id` that does not match the id pattern.
    InvalidId,
    /// Two items of the same node share an `id`.
    DuplicateId,
    /// A `status` absent from the node's workflow.
    UnknownStatus,
    /// A key forbidden for this kind, such as `status` on a ressource.
    ForbiddenKey,
    /// An archived item whose status is not closed.
    OpenInArchives,
    /// `parent` or `depends_on` points to no existing item.
    DanglingReference,
    /// A value outside its fixed vocabulary or format.
    OutOfVocabulary,
    /// A custom key declared in no applicable `node.toml`.
    UndeclaredKey,
    /// A file name that does not begin with the item's `id`, a `-` and a slug.
    FileName,
}

/// One problem found by doctor, serialized as is by `--json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// The rule that produced it.
    pub check: Check,
    /// Error or warning.
    pub severity: Severity,
    /// Node concerned, when there is one.
    pub node: Option<NodePath>,
    /// File concerned, relative to the repository root, when there is one.
    pub path: Option<String>,
    /// What is wrong, in one sentence.
    pub message: String,
    /// How to repair it.
    pub remedy: Remedy,
}

impl Finding {
    /// Tells whether the finding only says the file must go through `ariane migrate`.
    pub fn needs_migrate(&self) -> bool {
        matches!(self.check, Check::NoHeader | Check::OutdatedSchema)
    }
}

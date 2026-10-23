//! `node.toml`: what a node declares about its items (SPEC.md §1.5, ADR-0008).
//!
//! A node may carry a `node.toml` next to its `INDEX.md` with four tables:
//!
//! - `[fields.<key>]` declares a custom header key and its type;
//! - `[[workflow]]` lists the statuses its items may take, each with a stable
//!   `id`, a free `label` and one of the seven fixed categories;
//! - `[people.<handle>]` names the people an item may wait for, with their ids
//!   in the node's trackers;
//! - `[tracker]` binds the node to an external tracker (ADR-0011, not used yet).
//!
//! A node inherits from its ancestors: fields and people accumulate (the nearest
//! declaration wins), a workflow replaces the inherited one as a whole, and a node with no
//! workflow above it uses the default one, one status per category.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::tree::NodePath;

/// Name of the file a node declares itself in.
pub const FILE: &str = "node.toml";

/// The seven status categories fixed by Ariane. Every workflow status belongs to
/// one, which is what lets the dashboard aggregate nodes with different
/// workflows. `#[serde(rename_all = "snake_case")]` writes `InProgress` as
/// `in_progress`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Not planned yet.
    Backlog,
    /// Planned, not started.
    Todo,
    /// Being worked on, or waiting for validation.
    InProgress,
    /// Waiting for someone: a review, an answer; who is in `waiting_for`.
    Waiting,
    /// Cannot move until something else happens.
    Blocked,
    /// Finished.
    Done,
    /// Abandoned; closed without being done.
    Dropped,
}

impl Category {
    /// Every category, in display order.
    pub const ALL: [Category; 7] = [
        Category::Backlog,
        Category::Todo,
        Category::InProgress,
        Category::Waiting,
        Category::Blocked,
        Category::Done,
        Category::Dropped,
    ];

    /// The identifier written in files and JSON, such as `in_progress`.
    pub fn id(self) -> &'static str {
        match self {
            Category::Backlog => "backlog",
            Category::Todo => "todo",
            Category::InProgress => "in_progress",
            Category::Waiting => "waiting",
            Category::Blocked => "blocked",
            Category::Done => "done",
            Category::Dropped => "dropped",
        }
    }

    /// The English label of the default workflow.
    pub fn label(self) -> &'static str {
        match self {
            Category::Backlog => "Backlog",
            Category::Todo => "To do",
            Category::InProgress => "In progress",
            Category::Waiting => "Waiting",
            Category::Blocked => "Blocked",
            Category::Done => "Done",
            Category::Dropped => "Dropped",
        }
    }

    /// Tells whether an item in this category is finished, done or dropped.
    pub fn is_closed(self) -> bool {
        matches!(self, Category::Done | Category::Dropped)
    }
}

/// One status of a workflow, as declared in a `[[workflow]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// Stable identifier, the value written in items' `status` key.
    pub id: String,
    /// Text shown to humans; renaming it changes no item.
    pub label: String,
    /// The fixed category this status belongs to.
    pub category: Category,
}

/// Type of a custom key, as written in `type = "…"` of a `[fields.<key>]` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum FieldType {
    /// Any string.
    #[serde(rename = "string")]
    String,
    /// A whole number.
    #[serde(rename = "integer")]
    Integer,
    /// Any number.
    #[serde(rename = "number")]
    Number,
    /// `true` or `false`.
    #[serde(rename = "boolean")]
    Boolean,
    /// A `YYYY-MM-DD` date.
    #[serde(rename = "date")]
    Date,
    /// One item reference: `T022` or `node/path#T022`.
    #[serde(rename = "ref")]
    Ref,
    /// A list of strings, written `list<string>`.
    #[serde(rename = "list<string>")]
    ListOfStrings,
    /// A list of item references, written `list<ref>`.
    #[serde(rename = "list<ref>")]
    ListOfRefs,
    /// One string out of `values`.
    #[serde(rename = "enum")]
    Enum,
}

/// The declaration of one custom key.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FieldDecl {
    /// Its type, read from the `type` key (renamed because `type` is a Rust keyword).
    #[serde(rename = "type")]
    pub kind: FieldType,
    /// Allowed values when the type is `enum`; empty otherwise.
    #[serde(default)]
    pub values: Vec<String>,
    /// Whether every item of the node must carry the key.
    #[serde(default)]
    pub required: bool,
}

/// One person of a node's directory, `[people.<handle>]`: an item writes the
/// handle in `waiting_for`. Every other key is the person's id in a tracker,
/// such as `monday = 58294966`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    #[serde(flatten)]
    pub ids: BTreeMap<String, toml::Value>,
}

/// The content of one `node.toml` file, exactly as written.
/// `deny_unknown_fields` turns a misspelled table into a parse error instead of
/// silently ignoring it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeFile {
    /// Custom keys declared by this file.
    #[serde(default)]
    pub fields: BTreeMap<String, FieldDecl>,
    /// The workflow declared by this file, if any.
    pub workflow: Option<Vec<Status>>,
    /// The people declared by this file, by handle.
    #[serde(default)]
    pub people: BTreeMap<String, Person>,
    /// The tracker binding, kept as raw TOML until connectors exist (ADR-0011).
    pub tracker: Option<toml::Table>,
}

/// What applies to one node once its ancestors' files are combined.
#[derive(Debug, Clone)]
pub struct Effective {
    /// Custom keys of the node and its ancestors; the nearest declaration wins.
    pub fields: BTreeMap<String, FieldDecl>,
    /// The nearest workflow, or the default one.
    pub workflow: Vec<Status>,
    /// Node that declared the workflow; `None` for the default workflow.
    pub workflow_from: Option<NodePath>,
    /// People of the node and its ancestors, by handle; the nearest wins.
    pub people: BTreeMap<String, Person>,
    /// The nearest tracker binding, if any.
    pub tracker: Option<toml::Table>,
}

impl Effective {
    /// The workflow status whose id is `id`, if the workflow has one.
    pub fn status(&self, id: &str) -> Option<&Status> {
        self.workflow.iter().find(|status| status.id == id)
    }
}

/// Every `node.toml` of the tree, and those that could not be parsed.
#[derive(Debug, Default)]
pub struct Configs {
    /// Parsed files, keyed by the node holding them.
    pub files: BTreeMap<NodePath, NodeFile>,
    /// Files that exist but do not parse, with the parser's message, for `doctor`.
    pub errors: Vec<(PathBuf, String)>,
}

impl Configs {
    /// Reads the `node.toml` of every node and of every ancestor of a node.
    ///
    /// An ancestor may declare a workflow without holding any item, so every
    /// ancestor is a candidate. A missing file is normal and skipped; a file that
    /// does not parse is recorded in `errors`, never ignored.
    pub fn load(tree_dir: &Path, nodes: &[NodePath]) -> Configs {
        let mut configs = Configs::default();
        let mut candidates: Vec<NodePath> = nodes
            .iter()
            .flat_map(NodePath::ancestors_from_root)
            .collect();
        candidates.sort();
        candidates.dedup();
        for node in candidates {
            let path = file_of(tree_dir, &node);
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            match toml::from_str::<NodeFile>(&text) {
                Ok(file) => {
                    configs.files.insert(node, file);
                }
                Err(error) => configs.errors.push((path, error.to_string())),
            }
        }
        configs
    }

    /// Combines the files from the home node down to `node` (SPEC.md §1.5): fields
    /// accumulate, the nearest workflow and tracker replace inherited ones.
    pub fn effective(&self, node: &NodePath) -> Effective {
        let mut effective = Effective {
            fields: BTreeMap::new(),
            workflow: default_workflow(),
            workflow_from: None,
            people: BTreeMap::new(),
            tracker: None,
        };
        for ancestor in node.ancestors_from_root() {
            let Some(file) = self.files.get(&ancestor) else {
                continue;
            };
            effective.fields.extend(
                file.fields
                    .iter()
                    .map(|(key, decl)| (key.clone(), decl.clone())),
            );
            effective.people.extend(
                file.people
                    .iter()
                    .map(|(handle, person)| (handle.clone(), person.clone())),
            );
            if let Some(workflow) = &file.workflow {
                effective.workflow = workflow.clone();
                effective.workflow_from = Some(ancestor.clone());
            }
            if let Some(tracker) = &file.tracker {
                effective.tracker = Some(tracker.clone());
            }
        }
        effective
    }
}

/// The workflow of a node with none declared above it: one status per category, whose id is the category's.
pub fn default_workflow() -> Vec<Status> {
    Category::ALL
        .into_iter()
        .map(|category| Status {
            id: category.id().to_owned(),
            label: category.label().to_owned(),
            category,
        })
        .collect()
}

/// Path of the `node.toml` a node would hold.
fn file_of(tree_dir: &Path, node: &NodePath) -> PathBuf {
    let mut path = tree_dir.to_path_buf();
    for segment in node
        .as_str()
        .split('/')
        .filter(|segment| !segment.is_empty())
    {
        path.push(segment);
    }
    path.join(FILE)
}

/// Unit tests of `node.toml` parsing and inheritance.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Tree;
    use crate::tree::fixture::Repo;

    /// A parent `node.toml` declaring a field, a workflow and a tracker.
    const PARENT: &str = r#"
[fields.mod]
type = "string"
required = true

[[workflow]]
id = "a-faire"
label = "À faire"
category = "todo"

[tracker]
kind = "monday"
"#;

    /// A child `node.toml` overriding the field and the workflow.
    const CHILD: &str = r#"
[fields.mod]
type = "enum"
values = ["A"]

[fields.tests]
type = "list<string>"

[[workflow]]
id = "valide"
label = "Validé"
category = "done"
"#;

    #[test]
    fn a_node_without_any_node_toml_uses_one_status_per_category() {
        let configs = Configs::default();
        let effective = configs.effective(&NodePath::new("slashome/me"));
        let ids: Vec<_> = effective
            .workflow
            .iter()
            .map(|status| status.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "backlog",
                "todo",
                "in_progress",
                "waiting",
                "blocked",
                "done",
                "dropped"
            ]
        );
        assert_eq!(effective.workflow_from, None);
    }

    #[test]
    fn fields_accumulate_and_the_nearest_workflow_replaces_the_inherited_one() {
        let repo = Repo::new();
        repo.write("cmc/node.toml", PARENT)
            .write("cmc/lib/node.toml", CHILD)
            .write("cmc/lib/tasks/t.md", "# t\n");
        let tree = Tree::load(repo.root()).unwrap();
        let effective = tree.configs.effective(&NodePath::new("cmc/lib"));
        assert_eq!(effective.fields["mod"].kind, FieldType::Enum);
        assert!(effective.fields.contains_key("tests"));
        assert_eq!(effective.workflow.len(), 1);
        assert_eq!(effective.workflow[0].id, "valide");
        assert_eq!(effective.workflow_from, Some(NodePath::new("cmc/lib")));
        assert!(effective.tracker.is_some());
    }

    #[test]
    fn people_accumulate_from_the_ancestors_with_their_tracker_ids() {
        let repo = Repo::new();
        repo.write(
            "cmc/node.toml",
            "[people.keyvan]\nname = \"Keyvan Coen\"\nmonday = 58294966\n\n[people.doryan]\nname = \"Doryan\"\n",
        )
        .write("cmc/lib/node.toml", "[people.doryan]\nname = \"Doryan B.\"\n")
        .write("cmc/lib/tasks/t.md", "# t\n");
        let tree = Tree::load(repo.root()).unwrap();
        let effective = tree.configs.effective(&NodePath::new("cmc/lib"));
        assert_eq!(effective.people["keyvan"].name, "Keyvan Coen");
        assert_eq!(
            effective.people["keyvan"].ids["monday"].as_integer(),
            Some(58294966)
        );
        assert_eq!(effective.people["doryan"].name, "Doryan B.");
        assert!(
            tree.configs
                .effective(&NodePath::new("p"))
                .people
                .is_empty()
        );
    }

    #[test]
    fn the_default_workflow_has_a_waiting_status() {
        let waiting = default_workflow()
            .into_iter()
            .find(|status| status.category == Category::Waiting)
            .unwrap();
        assert_eq!(waiting.id, "waiting");
    }

    #[test]
    fn an_invalid_node_toml_is_reported_not_ignored() {
        let repo = Repo::new();
        repo.write("cmc/node.toml", "[[workflow]]\nid = 3\n")
            .write("cmc/tasks/t.md", "# t\n");
        let tree = Tree::load(repo.root()).unwrap();
        assert_eq!(tree.configs.errors.len(), 1);
    }
}

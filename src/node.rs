use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::tree::NodePath;

pub const FILE: &str = "node.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Backlog,
    Todo,
    InProgress,
    Blocked,
    Done,
    Dropped,
}

impl Category {
    pub const ALL: [Category; 6] = [
        Category::Backlog,
        Category::Todo,
        Category::InProgress,
        Category::Blocked,
        Category::Done,
        Category::Dropped,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Category::Backlog => "backlog",
            Category::Todo => "todo",
            Category::InProgress => "in_progress",
            Category::Blocked => "blocked",
            Category::Done => "done",
            Category::Dropped => "dropped",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Category::Backlog => "Backlog",
            Category::Todo => "To do",
            Category::InProgress => "In progress",
            Category::Blocked => "Blocked",
            Category::Done => "Done",
            Category::Dropped => "Dropped",
        }
    }

    pub fn is_closed(self) -> bool {
        matches!(self, Category::Done | Category::Dropped)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub id: String,
    pub label: String,
    pub category: Category,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum FieldType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "ref")]
    Ref,
    #[serde(rename = "list<string>")]
    ListOfStrings,
    #[serde(rename = "list<ref>")]
    ListOfRefs,
    #[serde(rename = "enum")]
    Enum,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FieldDecl {
    #[serde(rename = "type")]
    pub kind: FieldType,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeFile {
    #[serde(default)]
    pub fields: BTreeMap<String, FieldDecl>,
    pub workflow: Option<Vec<Status>>,
    pub tracker: Option<toml::Table>,
}

#[derive(Debug, Clone)]
pub struct Effective {
    pub fields: BTreeMap<String, FieldDecl>,
    pub workflow: Vec<Status>,
    pub workflow_from: Option<NodePath>,
    pub tracker: Option<toml::Table>,
}

impl Effective {
    pub fn status(&self, id: &str) -> Option<&Status> {
        self.workflow.iter().find(|status| status.id == id)
    }
}

#[derive(Debug, Default)]
pub struct Configs {
    pub files: BTreeMap<NodePath, NodeFile>,
    pub errors: Vec<(PathBuf, String)>,
}

impl Configs {
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

    pub fn effective(&self, node: &NodePath) -> Effective {
        let mut effective = Effective {
            fields: BTreeMap::new(),
            workflow: default_workflow(),
            workflow_from: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Tree;
    use crate::tree::fixture::Repo;

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
    fn an_invalid_node_toml_is_reported_not_ignored() {
        let repo = Repo::new();
        repo.write("cmc/node.toml", "[[workflow]]\nid = 3\n")
            .write("cmc/tasks/t.md", "# t\n");
        let tree = Tree::load(repo.root()).unwrap();
        assert_eq!(tree.configs.errors.len(), 1);
    }
}

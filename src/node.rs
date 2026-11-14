//! `node.toml`: what a node declares about its items (SPEC.md §1.5, ADR-0008).
//!
//! A node may carry a `node.toml` at the root of its directory: a top-level
//! `description` of the node in Markdown, never inherited, and nine tables:
//!
//! - `[fields.<key>]` declares a custom header key and its type;
//! - `[[workflow]]` lists the statuses its items may take, each with a stable
//!   `id`, a free `label`, one of the seven fixed categories and, on an
//!   `in_progress` status, an optional `stage` such as `testing`;
//! - `[[release]]` lists the node's releases in delivery order, each with a
//!   `name`, an optional `parking` marker and an optional `target` date
//!   (ADR-0023);
//! - `[estimates]` gives the hours of each size, the default size, the test
//!   ratio and who tests (ADR-0025);
//! - `[people.<handle>]` names the people an item may wait for, with their ids
//!   in the node's trackers;
//! - `[personas."<id>"]` names the personas a user story is written for
//!   (ADR-0018);
//! - `[changelog]` gives the language of a release's changelog and the
//!   reader it is written for (ADR-0029);
//! - `[tracker]` binds the node to an external tracker (ADR-0011, not used yet);
//! - `[proxy]`, or several `[[proxy]]`, give the node's development servers
//!   local names (ADR-0016).
//!
//! A node inherits from its ancestors: fields, estimates, changelog, people and personas accumulate key by
//! key (the nearest declaration wins), a workflow or a release list replaces the inherited one as a whole, and
//! a node with no workflow above it uses the default one, one status per category. A node with
//! no release list above it has none: its items keep free `release` values.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use toml_edit::{DocumentMut, Item as TomlItem, TableLike, Value as TomlValue};

use crate::estimate::{self, Pricing};
use crate::schema::is_date;
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

/// Where the work of a status stands within its category, for the forecast
/// (ADR-0028). Only a status of the `in_progress` category may declare one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// The work is done and only its testing is left.
    Testing,
}

impl Stage {
    /// Every stage, in the order `ariane schema --json` lists them.
    pub const ALL: [Stage; 1] = [Stage::Testing];

    /// The value written in `node.toml` and JSON.
    pub fn id(self) -> &'static str {
        match self {
            Stage::Testing => "testing",
        }
    }

    /// The category a status must belong to for declaring this stage.
    pub fn category(self) -> Category {
        match self {
            Stage::Testing => Category::InProgress,
        }
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
    /// Where the work stands within the category, such as `testing`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<Stage>,
}

/// Reads a `[[workflow]]` list, refusing a `stage` on a status of a category
/// the stage does not belong to.
fn workflow<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Vec<Status>>, D::Error> {
    let statuses = Vec::<Status>::deserialize(deserializer)?;
    for status in &statuses {
        let Some(stage) = status.stage else {
            continue;
        };
        if status.category != stage.category() {
            return Err(D::Error::custom(format!(
                "the status `{}` has `stage = \"{}\"`, which only a status of the `{}` category may declare, not `{}`",
                status.id,
                stage.id(),
                stage.category().id(),
                status.category.id()
            )));
        }
    }
    Ok(Some(statuses))
}

/// One release of a node, as declared in a `[[release]]` entry (ADR-0023): a
/// delivered version of the product, even internal. The order of the entries is
/// the delivery order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    /// The name items write in `release`, free: `1.8.0`, `0.3`, `mango`.
    pub name: String,
    /// Marks a release that is never scheduled and never closed, such as `plus-tard`.
    #[serde(default)]
    pub parking: bool,
    /// The desired date, `YYYY-MM-DD`, written as a TOML date or a string; it
    /// only checks a forecast, never changes the order (ADR-0028).
    #[serde(default, deserialize_with = "target")]
    pub target: Option<String>,
    /// The day the release was closed, `YYYY-MM-DD`, written by `ariane
    /// release close`: its work is delivered and archived, and it takes no
    /// more (ADR-0029).
    #[serde(default, deserialize_with = "target")]
    pub closed: Option<String>,
}

/// Reads a release's `target` or `closed`, a TOML date such as `2027-03-01` or the same
/// date as a string, and refuses anything else.
fn target<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    let text = match toml::Value::deserialize(deserializer)? {
        toml::Value::Datetime(date) => date.to_string(),
        toml::Value::String(text) => text,
        other => {
            return Err(D::Error::custom(format!(
                "`{other}` is not a YYYY-MM-DD date"
            )));
        }
    };
    if !is_date(&text) {
        return Err(D::Error::custom(format!(
            "`{text}` is not a YYYY-MM-DD date"
        )));
    }
    Ok(Some(text))
}

/// Reads a `[[release]]` list, refusing a blank name and a name declared twice:
/// a name is what items write, and its place in the list is its order.
fn releases<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Vec<Release>>, D::Error> {
    let releases = Vec::<Release>::deserialize(deserializer)?;
    for (index, release) in releases.iter().enumerate() {
        if release.name.trim().is_empty() {
            return Err(D::Error::custom("a `[[release]]` has a blank `name`"));
        }
        if releases[..index]
            .iter()
            .any(|earlier| earlier.name == release.name)
        {
            return Err(D::Error::custom(format!(
                "the release `{}` is declared twice",
                release.name
            )));
        }
        if release.parking && release.closed.is_some() {
            return Err(D::Error::custom(format!(
                "the parking release `{}` cannot be closed",
                release.name
            )));
        }
    }
    Ok(Some(releases))
}

/// The language of the headings of a changelog entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// `Added`, `Fixed`.
    #[default]
    En,
    /// `Nouveautés`, `Corrections`.
    Fr,
}

impl Language {
    /// Every language, in the order `ariane schema --json` lists them.
    pub const ALL: [Language; 2] = [Language::En, Language::Fr];

    /// The word written in `node.toml`.
    pub fn id(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Fr => "fr",
        }
    }
}

/// The reader a written changelog is for. Only the skill that writes the
/// changelog reads it; the raw entry of the CLI is the same for every reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Audience {
    /// A product owner.
    Po,
    /// A designer.
    Designer,
    /// A user of the product.
    User,
}

impl Audience {
    /// Every audience, in the order `ariane schema --json` lists them.
    pub const ALL: [Audience; 3] = [Audience::Po, Audience::Designer, Audience::User];

    /// The word written in `node.toml`.
    pub fn id(self) -> &'static str {
        match self {
            Audience::Po => "po",
            Audience::Designer => "designer",
            Audience::User => "user",
        }
    }
}

/// The `[changelog]` table of one `node.toml`, each key optional.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangelogDeclaration {
    /// The language of the headings.
    pub language: Option<Language>,
    /// The reader the skill writes for.
    pub audience: Option<Audience>,
}

/// What applies to a node's changelog once its ancestors' tables are
/// combined key by key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Changelog {
    /// The language of the headings, `en` when no node declares one.
    pub language: Language,
    /// The reader the skill writes for, when a node declares one.
    pub audience: Option<Audience>,
}

impl Changelog {
    /// Applies the keys `declaration` gives over the inherited ones.
    fn apply(&mut self, declaration: &ChangelogDeclaration) {
        if let Some(language) = declaration.language {
            self.language = language;
        }
        if declaration.audience.is_some() {
            self.audience = declaration.audience;
        }
    }
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

/// One persona of a node, `[personas."<id>"]`: a user story names the ids of
/// the personas it is written for in `personas`. The id, such as `cmc:dev`, is
/// a quoted TOML key because of its colon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Persona {
    /// The readable label, such as `Développeur CMC`.
    pub name: String,
}

/// The content of one `node.toml` file, exactly as written.
/// `deny_unknown_fields` turns a misspelled table into a parse error instead of
/// silently ignoring it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeFile {
    /// What the node is, in Markdown: its role, host repository, stack and
    /// sources of truth (ADR-0017); never inherited.
    pub description: Option<String>,
    /// Custom keys declared by this file.
    #[serde(default)]
    pub fields: BTreeMap<String, FieldDecl>,
    /// The workflow declared by this file, if any.
    #[serde(default, deserialize_with = "workflow")]
    pub workflow: Option<Vec<Status>>,
    /// The releases declared by this file in delivery order, if any.
    #[serde(default, deserialize_with = "releases")]
    pub release: Option<Vec<Release>>,
    /// The hours of each size and the testing rule declared by this file,
    /// each key optional (ADR-0025).
    #[serde(default)]
    pub estimates: estimate::Declaration,
    /// The changelog's language and reader declared by this file.
    #[serde(default)]
    pub changelog: ChangelogDeclaration,
    /// The people declared by this file, by handle.
    #[serde(default)]
    pub people: BTreeMap<String, Person>,
    /// The personas declared by this file, by id.
    #[serde(default)]
    pub personas: BTreeMap<String, Persona>,
    /// The tracker binding, kept as raw TOML until connectors exist (ADR-0011).
    pub tracker: Option<toml::Table>,
    /// The local names of the node's development servers, one per `[proxy]`
    /// table or `[[proxy]]` entry; never inherited.
    #[serde(default, deserialize_with = "proxies")]
    pub proxy: Vec<Proxy>,
}

/// Reads `proxy` as one `[proxy]` table or as a `[[proxy]]` array of them.
fn proxies<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Proxy>, D::Error> {
    match toml::Value::deserialize(deserializer)? {
        toml::Value::Array(entries) => toml::Value::Array(entries).try_into(),
        table => table.try_into().map(|proxy| vec![proxy]),
    }
    .map_err(D::Error::custom)
}

/// A `[proxy]` table or `[[proxy]]` entry: the name under which `ariane serve` and `ariane proxy`
/// forward requests to a port of the loopback interface (ADR-0016).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proxy {
    /// The host name typed in the browser, such as `dreamcatchr.local`.
    pub host: String,
    /// The port the development server listens on, on `127.0.0.1`.
    pub port: u16,
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
    /// The nearest release list, in delivery order; `None` when no node above
    /// declares one, and `release` values stay free.
    pub releases: Option<Vec<Release>>,
    /// Node that declared the release list.
    pub releases_from: Option<NodePath>,
    /// What sizes cost, from the `[estimates]` of the node and its ancestors,
    /// key by key, over the values of ADR-0025.
    pub estimates: Pricing,
    /// The changelog's language and reader, key by key.
    pub changelog: Changelog,
    /// People of the node and its ancestors, by handle; the nearest wins.
    pub people: BTreeMap<String, Person>,
    /// Personas of the node and its ancestors, by id; the nearest wins.
    pub personas: BTreeMap<String, Persona>,
    /// The nearest tracker binding, if any.
    pub tracker: Option<toml::Table>,
    /// Node that declared the tracker binding.
    pub tracker_from: Option<NodePath>,
}

impl Effective {
    /// The workflow status whose id is `id`, if the workflow has one.
    pub fn status(&self, id: &str) -> Option<&Status> {
        self.workflow.iter().find(|status| status.id == id)
    }

    /// The place of the release named `name` in the delivery order, from 0,
    /// with its declaration; `None` when no list applies or it lacks `name`.
    pub fn release(&self, name: &str) -> Option<(usize, &Release)> {
        self.releases
            .as_ref()?
            .iter()
            .enumerate()
            .find(|(_, release)| release.name == name)
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

    /// The `description` of `node`'s own `node.toml`, never an ancestor's.
    pub fn description(&self, node: &NodePath) -> Option<&str> {
        self.files.get(node)?.description.as_deref()
    }

    /// The category of the status `status` in `node`'s effective workflow,
    /// read without combining the rest of the files; `None` when the workflow
    /// has no such status.
    pub fn category(&self, node: &NodePath, status: &str) -> Option<Category> {
        match self.declared_workflow(node) {
            Some(workflow) => workflow
                .iter()
                .find(|entry| entry.id == status)
                .map(|entry| entry.category),
            None => Category::ALL
                .into_iter()
                .find(|category| category.id() == status),
        }
    }

    /// The stage of the status `status` in `node`'s effective workflow, read
    /// without combining the rest of the files; `None` when the status
    /// declares none, or the workflow has no such status.
    pub fn stage(&self, node: &NodePath, status: &str) -> Option<Stage> {
        self.declared_workflow(node)?
            .iter()
            .find(|entry| entry.id == status)?
            .stage
    }

    /// The workflow of `node`'s nearest `node.toml` declaring one; `None` for
    /// the default workflow.
    fn declared_workflow(&self, node: &NodePath) -> Option<&Vec<Status>> {
        node.ancestors_from_root()
            .iter()
            .rev()
            .find_map(|ancestor| self.files.get(ancestor)?.workflow.as_ref())
    }

    /// What sizes cost on `node`: the `[estimates]` of its ancestors from the
    /// root down, key by key, without combining the rest of the files.
    pub fn pricing(&self, node: &NodePath) -> Pricing {
        let mut pricing = Pricing::default();
        for ancestor in node.ancestors_from_root() {
            if let Some(file) = self.files.get(&ancestor) {
                pricing.apply(&file.estimates);
            }
        }
        pricing
    }

    /// Combines the files from the home node down to `node` (SPEC.md §1.5): fields,
    /// estimates, people and personas accumulate, the nearest workflow, release list and
    /// tracker replace inherited ones.
    pub fn effective(&self, node: &NodePath) -> Effective {
        let mut effective = Effective {
            fields: BTreeMap::new(),
            workflow: default_workflow(),
            workflow_from: None,
            releases: None,
            releases_from: None,
            estimates: Pricing::default(),
            changelog: Changelog::default(),
            people: BTreeMap::new(),
            personas: BTreeMap::new(),
            tracker: None,
            tracker_from: None,
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
            effective.estimates.apply(&file.estimates);
            effective.changelog.apply(&file.changelog);
            effective.people.extend(
                file.people
                    .iter()
                    .map(|(handle, person)| (handle.clone(), person.clone())),
            );
            effective.personas.extend(
                file.personas
                    .iter()
                    .map(|(id, persona)| (id.clone(), persona.clone())),
            );
            if let Some(workflow) = &file.workflow {
                effective.workflow = workflow.clone();
                effective.workflow_from = Some(ancestor.clone());
            }
            if let Some(releases) = &file.release {
                effective.releases = Some(releases.clone());
                effective.releases_from = Some(ancestor.clone());
            }
            if let Some(tracker) = &file.tracker {
                effective.tracker = Some(tracker.clone());
                effective.tracker_from = Some(ancestor.clone());
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
            stage: None,
        })
        .collect()
}

/// Edits the `node.toml` at `file` in place, comments, order and unknown keys
/// kept: only the lines the change touches differ. A missing file starts
/// empty. A change that refuses writes nothing and hands back its refusal.
///
/// # Errors
///
/// Fails when the file cannot be read, parsed or written, and writes nothing
/// when the edited file would no longer be a valid `node.toml`.
pub fn edit<E>(
    file: &Path,
    change: impl FnOnce(&mut DocumentMut) -> Result<(), E>,
) -> Result<Result<(), E>> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error).with_context(|| format!("reading {}", file.display())),
    };
    let mut document: DocumentMut = text
        .parse()
        .with_context(|| format!("parsing {}", file.display()))?;
    if let Err(refusal) = change(&mut document) {
        return Ok(Err(refusal));
    }
    let edited = document.to_string();
    toml::from_str::<NodeFile>(&edited).with_context(|| {
        format!(
            "the edit would leave {} invalid; nothing written",
            file.display()
        )
    })?;
    std::fs::write(file, edited).with_context(|| format!("writing {}", file.display()))?;
    Ok(Ok(()))
}

/// Sets `key` of `table` to `new`, keeping the spacing and the trailing
/// comment of the value it replaces, so that only the value changes.
pub fn set_value(table: &mut dyn TableLike, key: &str, mut new: TomlValue) {
    if let Some(old) = table.get(key).and_then(TomlItem::as_value) {
        *new.decor_mut() = old.decor().clone();
    }
    table.insert(key, TomlItem::Value(new));
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
    fn personas_accumulate_from_the_ancestors_and_the_nearest_wins() {
        let repo = Repo::new();
        repo.write(
            "cmc/node.toml",
            "[personas.\"cmc:dev\"]\nname = \"Développeur CMC\"\n\n[personas.\"user:rh\"]\nname = \"RH\"\n",
        )
        .write(
            "cmc/lib/node.toml",
            "[personas.\"user:rh\"]\nname = \"Responsable RH\"\n",
        )
        .write("cmc/lib/stories/s.md", "# s\n");
        let tree = Tree::load(repo.root()).unwrap();
        assert!(tree.configs.errors.is_empty(), "{:?}", tree.configs.errors);
        let effective = tree.configs.effective(&NodePath::new("cmc/lib"));
        assert_eq!(effective.personas["cmc:dev"].name, "Développeur CMC");
        assert_eq!(effective.personas["user:rh"].name, "Responsable RH");
        assert!(
            tree.configs
                .effective(&NodePath::new("p"))
                .personas
                .is_empty()
        );
    }

    #[test]
    fn a_description_belongs_to_its_node_only() {
        let repo = Repo::new();
        repo.write(
            "cmc/node.toml",
            "description = \"\"\"\nThe **CMC** projects.\n\nDesign system: flow.\n\"\"\"\n\n[people.keyvan]\nname = \"Keyvan\"\n",
        )
        .write("cmc/lib/node.toml", "")
        .write("cmc/lib/stories/s.md", "# s\n");
        let tree = Tree::load(repo.root()).unwrap();
        assert!(tree.configs.errors.is_empty(), "{:?}", tree.configs.errors);
        assert_eq!(
            tree.configs.description(&NodePath::new("cmc")),
            Some("The **CMC** projects.\n\nDesign system: flow.\n")
        );
        assert_eq!(tree.configs.description(&NodePath::new("cmc/lib")), None);
    }

    #[test]
    fn a_persona_takes_a_name_and_nothing_else() {
        let repo = Repo::new();
        repo.write(
            "cmc/node.toml",
            "[personas.\"cmc:dev\"]\nname = \"Dev\"\nmonday = 1\n",
        )
        .write("cmc/stories/s.md", "# s\n");
        let tree = Tree::load(repo.root()).unwrap();
        assert_eq!(tree.configs.errors.len(), 1);
    }

    #[test]
    fn the_default_workflow_has_a_waiting_status() {
        let waiting = default_workflow()
            .into_iter()
            .find(|status| status.category == Category::Waiting)
            .unwrap();
        assert_eq!(waiting.id, "waiting");
    }

    /// A project `node.toml` declaring three releases, the last one parked.
    const RELEASES: &str = r#"
[[release]]
name = "0.9"
target = 2027-01-15

[[release]]
name = "1.0"
target = "2027-03-01"

[[release]]
name = "plus-tard"
parking = true
"#;

    #[test]
    fn releases_keep_their_declared_order_and_the_nearest_list_replaces_the_inherited_one() {
        let repo = Repo::new();
        repo.write("p/node.toml", RELEASES)
            .write("p/a/tasks/t.md", "# t\n")
            .write("p/b/node.toml", "[[release]]\nname = \"mango\"\n")
            .write("p/b/tasks/t.md", "# t\n");
        let tree = Tree::load(repo.root()).unwrap();
        assert!(tree.configs.errors.is_empty(), "{:?}", tree.configs.errors);
        let inherited = tree.configs.effective(&NodePath::new("p/a"));
        let names: Vec<_> = inherited
            .releases
            .as_ref()
            .unwrap()
            .iter()
            .map(|release| release.name.as_str())
            .collect();
        assert_eq!(names, ["0.9", "1.0", "plus-tard"]);
        assert_eq!(inherited.releases_from, Some(NodePath::new("p")));
        let (position, parked) = inherited.release("plus-tard").unwrap();
        assert_eq!(position, 2);
        assert!(parked.parking);
        assert_eq!(
            inherited.release("0.9").unwrap().1.target.as_deref(),
            Some("2027-01-15")
        );
        assert_eq!(
            inherited.release("1.0").unwrap().1.target.as_deref(),
            Some("2027-03-01")
        );
        let replaced = tree.configs.effective(&NodePath::new("p/b"));
        assert_eq!(replaced.releases.as_ref().unwrap().len(), 1);
        assert!(replaced.release("1.0").is_none());
        assert_eq!(replaced.releases_from, Some(NodePath::new("p/b")));
        let free = tree.configs.effective(&NodePath::new("q"));
        assert!(free.releases.is_none());
        assert!(free.release("1.0").is_none());
    }

    #[test]
    fn a_release_list_refuses_duplicate_names_blank_names_and_bad_targets() {
        for text in [
            "[[release]]\nname = \"1.0\"\n\n[[release]]\nname = \"1.0\"\n",
            "[[release]]\nname = \" \"\n",
            "[[release]]\nname = \"1.0\"\ntarget = \"mars\"\n",
            "[[release]]\nname = \"1.0\"\ntarget = 2027-03-01T10:00:00\n",
            "[[release]]\nname = \"1.0\"\nwhen = \"soon\"\n",
            "[[release]]\nname = \"1.0\"\nclosed = \"yesterday\"\n",
            "[[release]]\nname = \"later\"\nparking = true\nclosed = 2026-11-01\n",
        ] {
            assert!(toml::from_str::<NodeFile>(text).is_err(), "{text}");
        }
        let closed: NodeFile =
            toml::from_str("[[release]]\nname = \"0.1\"\nclosed = 2026-11-14\n").unwrap();
        assert_eq!(
            closed.release.unwrap()[0].closed.as_deref(),
            Some("2026-11-14")
        );
    }

    #[test]
    fn the_changelog_table_is_inherited_key_by_key_and_writes_english_by_default() {
        let repo = Repo::new();
        repo.write("p/node.toml", "[changelog]\nlanguage = \"fr\"\n")
            .write("p/a/node.toml", "[changelog]\naudience = \"po\"\n")
            .write("p/a/tasks/t.md", "# t\n")
            .write("q/tasks/t.md", "# t\n");
        let tree = Tree::load(repo.root()).unwrap();
        let child = tree.configs.effective(&NodePath::new("p/a")).changelog;
        assert_eq!(child.language, Language::Fr);
        assert_eq!(child.audience, Some(Audience::Po));
        let free = tree.configs.effective(&NodePath::new("q")).changelog;
        assert_eq!(free, Changelog::default());
        assert_eq!(free.language, Language::En);
        for text in [
            "[changelog]\nlanguage = \"de\"\n",
            "[changelog]\naudience = \"cto\"\n",
            "[changelog]\nformat = \"md\"\n",
        ] {
            assert!(toml::from_str::<NodeFile>(text).is_err(), "{text}");
        }
    }

    #[test]
    fn a_node_without_estimates_anywhere_prices_sizes_with_the_values_of_the_adr() {
        let pricing = Configs::default().pricing(&NodePath::new("p"));
        assert_eq!(
            [pricing.xs, pricing.s, pricing.m, pricing.l, pricing.xl],
            [2.0, 4.0, 12.0, 24.0, 48.0]
        );
        assert_eq!(pricing.default, "M");
        assert_eq!(pricing.test_ratio, 0.25);
        assert_eq!(pricing.tester, estimate::Tester::Owner);
    }

    #[test]
    fn estimates_are_inherited_key_by_key_and_the_nearest_declaration_wins() {
        let repo = Repo::new();
        repo.write(
            "p/node.toml",
            "[estimates]\nXS = 1\nM = 10\ndefault = \"S\"\ntester = \"others\"\n",
        )
        .write("p/a/node.toml", "[estimates]\nM = 8.5\ntest_ratio = 0\n")
        .write("p/a/tasks/t.md", "# t\n");
        let tree = Tree::load(repo.root()).unwrap();
        assert!(tree.configs.errors.is_empty(), "{:?}", tree.configs.errors);
        let child = tree.configs.effective(&NodePath::new("p/a")).estimates;
        assert_eq!(child.xs, 1.0);
        assert_eq!(child.s, 4.0);
        assert_eq!(child.m, 8.5);
        assert_eq!(child.xl, 48.0);
        assert_eq!(child.default, "S");
        assert_eq!(child.default_hours(), 4.0);
        assert_eq!(child.test_ratio, 0.0);
        assert_eq!(child.tester, estimate::Tester::Others);
        assert_eq!(tree.configs.pricing(&NodePath::new("p/a")), child);
        let parent = tree.configs.pricing(&NodePath::new("p"));
        assert_eq!(parent.m, 10.0);
        assert_eq!(parent.test_ratio, 0.25);
    }

    #[test]
    fn an_estimates_table_refuses_unknown_keys_bad_hours_ratios_sizes_and_testers() {
        for text in [
            "[estimates]\nXXL = 96\n",
            "[estimates]\nM = 0\n",
            "[estimates]\nS = -4\n",
            "[estimates]\nL = \"24\"\n",
            "[estimates]\ndefault = \"XXL\"\n",
            "[estimates]\ndefault = \"m\"\n",
            "[estimates]\ntest_ratio = -0.1\n",
            "[estimates]\ntester = \"qa\"\n",
        ] {
            assert!(toml::from_str::<NodeFile>(text).is_err(), "{text}");
        }
        let accepted: NodeFile = toml::from_str(
            "[estimates]\nXS = 0.5\nXL = 40\ndefault = \"XL\"\ntest_ratio = 0\ntester = \"owner\"\n",
        )
        .unwrap();
        assert_eq!(accepted.estimates.xs, Some(0.5));
        assert_eq!(accepted.estimates.default.as_deref(), Some("XL"));
    }

    #[test]
    fn an_in_progress_status_may_declare_the_testing_stage_and_no_other_may() {
        let accepted: NodeFile = toml::from_str(
            "[[workflow]]\nid = \"en-cours\"\nlabel = \"En cours\"\ncategory = \"in_progress\"\n\n[[workflow]]\nid = \"a-tester\"\nlabel = \"À tester\"\ncategory = \"in_progress\"\nstage = \"testing\"\n",
        )
        .unwrap();
        let workflow = accepted.workflow.unwrap();
        assert_eq!(workflow[0].stage, None);
        assert_eq!(workflow[1].stage, Some(Stage::Testing));
        for text in [
            "[[workflow]]\nid = \"a\"\nlabel = \"A\"\ncategory = \"in_progress\"\nstage = \"review\"\n",
            "[[workflow]]\nid = \"a\"\nlabel = \"A\"\ncategory = \"todo\"\nstage = \"testing\"\n",
            "[[workflow]]\nid = \"a\"\nlabel = \"A\"\ncategory = \"waiting\"\nstage = \"testing\"\n",
        ] {
            assert!(toml::from_str::<NodeFile>(text).is_err(), "{text}");
        }
    }

    #[test]
    fn the_stage_of_a_status_is_read_from_the_nearest_workflow() {
        let repo = Repo::new();
        repo.write(
            "p/node.toml",
            "[[workflow]]\nid = \"a-tester\"\nlabel = \"À tester\"\ncategory = \"in_progress\"\nstage = \"testing\"\n\n[[workflow]]\nid = \"en-cours\"\nlabel = \"En cours\"\ncategory = \"in_progress\"\n",
        )
        .write("p/q/tasks/t.md", "# t\n");
        let tree = Tree::load(repo.root()).unwrap();
        let child = NodePath::new("p/q");
        assert_eq!(tree.configs.stage(&child, "a-tester"), Some(Stage::Testing));
        assert_eq!(tree.configs.stage(&child, "en-cours"), None);
        assert_eq!(tree.configs.stage(&NodePath::new("r"), "in_progress"), None);
        assert_eq!(
            tree.configs.effective(&child).workflow[0].stage,
            Some(Stage::Testing)
        );
    }

    #[test]
    fn an_edit_changes_only_the_lines_it_touches_and_never_leaves_the_file_invalid() {
        let repo = Repo::new();
        let original = "# the team\ndescription = \"\"\"\nA node.\n\"\"\"\n\n[tracker]\nkind = \"monday\"  # bound\nboard = 1\nfuture = { a = 1 }\n\n[people.ann]\nname = \"Ann\"\n";
        repo.write("cmc/node.toml", original);
        let file = repo.root().join("tree/cmc/node.toml");
        edit(&file, |document| {
            document["tracker"]["board"] = toml_edit::value(2);
            Ok::<(), ()>(())
        })
        .unwrap()
        .unwrap();
        let edited = std::fs::read_to_string(&file).unwrap();
        assert_eq!(edited, original.replace("board = 1", "board = 2"));
        let refused = edit(&file, |document| {
            document["people"]["ann"]["name"] = toml_edit::value(3);
            Ok::<(), ()>(())
        });
        assert!(refused.is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), edited);
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

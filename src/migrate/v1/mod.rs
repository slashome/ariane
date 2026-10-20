//! Migration 0 → 1: builds the version-1 header of a legacy item (ADR-0009).
//!
//! Three starting points are handled:
//!
//! - a YAML front-matter with French keys, as in the pharmakon node: keys are
//!   renamed (`titre` → `title`, `statut` → `status`, `jalon` → `milestone`,
//!   `depend` → `depends_on`) and every other key is kept in place. A
//!   front-matter that strict YAML rejects (an unquoted title containing `: `)
//!   is read line by line instead;
//! - a front-matter holding only `node`, followed by a prose header;
//! - no front-matter at all, only a prose header.
//!
//! In the last two cases [`prose::extract`](crate::migrate::v1::prose::extract) reads the legacy prose header. Then
//! the missing pieces are filled in: `id` from `ids`, `title` from the first
//! `# ` heading, `status` mapped onto the node's workflow, `created` from the
//! commit that added the file. Keys are written in the canonical order of
//! [`CORE_KEYS`](crate::schema::CORE_KEYS), custom keys after them. When a value cannot be decided, the
//! file is left to a human with the reasons, never guessed silently.

/// Allocation of item ids.
mod ids;
/// Reading of the legacy prose headers.
pub mod prose;
/// The legacy status and priority vocabularies.
mod vocabulary;

use std::path::Path;
use std::process::Command;

use yaml_serde::{Mapping, Value};

use crate::item::{self, Document, Header, Kind};
use crate::node::{Category, Effective};
use crate::schema::{self, CORE_KEYS, ValueType};
use crate::tree::{Item, NodePath, Tree};

use super::{Migration, Step};
use ids::Allocations;
use prose::StatusValue;

/// French keys of pharmakon's front-matter and their English names (SPEC.md §1.2).
const RENAMED: [(&str, &str); 4] = [
    ("titre", "title"),
    ("statut", "status"),
    ("jalon", "milestone"),
    ("depend", "depends_on"),
];

/// The migration from version 0 to version 1. It borrows the tree (lifetime
/// `'a`) to read node workflows and the repository root, and holds the ids
/// allocated for the whole scope up front.
pub struct ZeroToOne<'a> {
    tree: &'a Tree,
    ids: Allocations,
}

impl<'a> ZeroToOne<'a> {
    /// Prepares the migration of `scope`, allocating every id it will need.
    pub fn new(tree: &'a Tree, scope: Option<&NodePath>) -> ZeroToOne<'a> {
        ZeroToOne {
            tree,
            ids: Allocations::plan(tree, scope),
        }
    }
}

/// The migration as seen by the engine.
impl Migration for ZeroToOne<'_> {
    /// Version 0, the files without an `ariane` key.
    fn source(&self) -> u64 {
        0
    }

    /// Builds the version-1 text of one item: reads its header (tolerantly when
    /// needed), renames French keys, extracts the prose header when there is no
    /// structured one, fills in `id`, `title`, `status` and `created`, then renders
    /// the header in canonical order above the body.
    ///
    /// Returns `Err` with every reason collected when one value cannot be decided.
    fn apply(&self, item: &Item, text: &str) -> Result<Step, Vec<String>> {
        let document = Document::parse(text);
        let header = match &document.header {
            Header::Missing => Mapping::new(),
            Header::Malformed(error) => match flat_front_matter(text) {
                Some(mapping) => mapping,
                None => {
                    return Err(vec![format!("the front-matter is not valid YAML: {error}")]);
                }
            },
            Header::Present(mapping) => mapping.clone(),
        };
        let mut draft = Draft::default();
        if matches!(document.header, Header::Malformed(_)) {
            draft.notes.push(
                "the front-matter was not valid YAML; read as flat `key: value` lines".to_owned(),
            );
        }
        let mut fields = draft.rename(&header, raw_front_matter(text));
        let structured = fields.keys().any(|key| key.as_str() != Some("node"));
        let mut body = document.body.clone();
        let mut extracted_status = None;
        if !structured {
            let extraction = prose::extract(item.kind, &document.body);
            for (key, value) in extraction.fields {
                if !fields.contains_key(key.as_str()) {
                    fields.insert(Value::from(key), value);
                }
            }
            draft.inferred.extend(extraction.inferred);
            draft.notes.extend(extraction.leftovers);
            extracted_status = extraction.status;
            body = extraction.body;
        }
        draft.check_node(&fields, &item.node);
        let id = text_of(&fields, "id")
            .map(str::to_owned)
            .or_else(|| self.ids.id_of(&item.relative).map(str::to_owned));
        let title = draft.title(&fields, &body, id.as_deref());
        let effective = self.tree.configs.effective(&item.node);
        let status = draft.status(&fields, item, extracted_status, &effective);
        let created = match text_of(&fields, "created") {
            Some(created) => Some(created.to_owned()),
            None => added_on(&self.tree.root, &item.relative).inspect(|_| {
                draft.inferred.push("created".to_owned());
            }),
        };
        if !draft.reasons.is_empty() {
            return Err(draft.reasons);
        }
        fields.insert("ariane".into(), Value::from(item::SCHEMA_VERSION));
        fields.insert("node".into(), Value::from(item.node.as_str()));
        if let Some(id) = id {
            fields.insert("id".into(), Value::from(id));
        }
        if let Some(title) = title {
            fields.insert("title".into(), Value::from(title));
        }
        match status {
            Some(status) => {
                fields.insert("status".into(), Value::from(status));
            }
            None => {
                fields.shift_remove("status");
            }
        }
        if let Some(created) = created {
            fields.insert("created".into(), Value::from(created));
        }
        let rendered =
            item::render(&ordered(fields), &body).map_err(|error| vec![error.to_string()])?;
        Ok(Step {
            text: rendered,
            inferred: draft.inferred,
            notes: draft.notes,
        })
    }
}

/// What the migration of one item has collected so far.
#[derive(Default)]
struct Draft {
    /// Why a human must decide; non-empty means the file is not migrated.
    reasons: Vec<String>,
    /// Keys whose value was guessed.
    inferred: Vec<String>,
    /// Remarks for the reviewer.
    notes: Vec<String>,
}

/// The steps of migration 0 → 1 that can fail or guess.
impl Draft {
    /// Copies the header with French keys renamed to English. A key present in both
    /// languages needs a human. `release` and `milestone` are kept as text, using
    /// the raw text of the file so that `1.0` stays `"1.0"` and does not become `1`.
    fn rename(&mut self, header: &Mapping, raw: Option<&str>) -> Mapping {
        let mut fields = Mapping::new();
        for (key, value) in header {
            let Some(name) = key.as_str() else {
                self.reasons
                    .push(format!("the front-matter has a non-text key: {key:?}"));
                continue;
            };
            let target = RENAMED
                .iter()
                .find(|(french, _)| *french == name)
                .map_or(name, |(_, english)| english);
            if target != name && header.contains_key(target) {
                self.reasons
                    .push(format!("both `{name}` and `{target}` are present"));
                continue;
            }
            let value = if schema::keys_of_type(ValueType::Name).any(|key| key == target) {
                as_name(value, raw.and_then(|raw| raw_scalar(raw, name)))
            } else {
                value.clone()
            };
            fields.insert(Value::from(target), value);
        }
        fields
    }

    /// Records a reason when the header's `node` disagrees with where the file sits.
    fn check_node(&mut self, fields: &Mapping, node: &NodePath) {
        let Some(declared) = fields.get("node") else {
            return;
        };
        if declared.as_str() != Some(node.as_str()) {
            self.reasons.push(format!(
                "the header says node `{}` but the file lives in `{node}`",
                declared.as_str().unwrap_or("?")
            ));
        }
    }

    /// The title: the `title` key, or else the first `# ` heading with a leading
    /// `<id> —` removed. Records a reason when there is neither.
    fn title(&mut self, fields: &Mapping, body: &str, id: Option<&str>) -> Option<String> {
        if let Some(title) = text_of(fields, "title") {
            return Some(title.to_owned());
        }
        let heading = body
            .lines()
            .find_map(|line| line.strip_prefix("# "))
            .map(str::trim)
            .filter(|heading| !heading.is_empty());
        let Some(heading) = heading else {
            self.reasons
                .push("no title: no `title` key and no `# ` heading".to_owned());
            return None;
        };
        let without_id = id
            .and_then(|id| heading.strip_prefix(id))
            .and_then(|rest| rest.trim_start().strip_prefix(['—', '–', '-', ':']))
            .map(str::trim)
            .filter(|rest| !rest.is_empty());
        Some(without_id.unwrap_or(heading).to_owned())
    }

    /// The status: the one already written; or the extracted one mapped onto the
    /// node's workflow (a category becomes the first workflow status of that
    /// category, a reflexion keeps open, trial or settled); or, for an archived
    /// item, a closed status marked as inferred. A ressource gets none. Records a
    /// reason when no status can be decided.
    fn status(
        &mut self,
        fields: &Mapping,
        item: &Item,
        extracted: Option<StatusValue>,
        effective: &Effective,
    ) -> Option<String> {
        if item.kind == Kind::Ressource {
            if fields.contains_key("status") {
                self.notes
                    .push("a ressource carries a status; left for doctor to report".to_owned());
                return text_of(fields, "status").map(str::to_owned);
            }
            return None;
        }
        if let Some(status) = text_of(fields, "status") {
            return Some(status.to_owned());
        }
        let resolved = match (item.kind, extracted) {
            (Kind::Reflexion, Some(StatusValue::Open)) => Some("open".to_owned()),
            (Kind::Reflexion, Some(StatusValue::Trial)) => Some("trial".to_owned()),
            (Kind::Reflexion, Some(StatusValue::Settled)) => Some("settled".to_owned()),
            (Kind::Reflexion, Some(StatusValue::Category(category))) => {
                self.inferred.push("status".to_owned());
                Some(
                    if category.is_closed() {
                        "settled"
                    } else {
                        "open"
                    }
                    .to_owned(),
                )
            }
            (_, Some(StatusValue::Category(category))) => {
                let status = first_of(effective, category);
                if status.is_none() {
                    self.reasons.push(format!(
                        "the node workflow has no status in category `{}`",
                        category.id()
                    ));
                }
                status
            }
            (_, Some(StatusValue::Open)) => first_of(effective, Category::Todo).inspect(|_| {
                self.inferred.push("status".to_owned());
            }),
            (_, Some(StatusValue::Trial)) => {
                first_of(effective, Category::InProgress).inspect(|_| {
                    self.inferred.push("status".to_owned());
                })
            }
            (_, Some(StatusValue::Settled)) => first_of(effective, Category::Done).inspect(|_| {
                self.inferred.push("status".to_owned());
            }),
            (_, None) => None,
        };
        if resolved.is_some() {
            return resolved;
        }
        if item.archived {
            let fallback = match item.kind {
                Kind::Reflexion => Some("settled".to_owned()),
                _ => first_of(effective, Category::Done),
            };
            if fallback.is_some() {
                self.inferred.push("status".to_owned());
                return fallback;
            }
        }
        if !self
            .reasons
            .iter()
            .any(|reason| reason.contains("workflow"))
        {
            self.reasons.push("no status found".to_owned());
        }
        None
    }
}

/// The id of the first status of `category` in the workflow.
fn first_of(effective: &Effective, category: Category) -> Option<String> {
    effective
        .workflow
        .iter()
        .find(|status| status.category == category)
        .map(|status| status.id.clone())
}

/// The value of `key` when it is a non-empty string, borrowed from the mapping.
fn text_of<'a>(fields: &'a Mapping, key: &str) -> Option<&'a str> {
    fields.get(key)?.as_str().filter(|text| !text.is_empty())
}

/// A number or boolean turned into the text it was written as, so that names such as release `1.0` stay strings.
fn as_name(value: &Value, raw: Option<&str>) -> Value {
    match value {
        Value::Number(number) => Value::from(raw.map_or_else(|| number.to_string(), str::to_owned)),
        Value::Bool(flag) => Value::from(raw.map_or_else(|| flag.to_string(), str::to_owned)),
        other => other.clone(),
    }
}

/// The fields with the core keys first, in canonical order, then the custom keys in their original order.
fn ordered(mut fields: Mapping) -> Mapping {
    let mut result = Mapping::new();
    for key in CORE_KEYS {
        if let Some(value) = fields.shift_remove(key.name) {
            result.insert(Value::from(key.name), value);
        }
    }
    for (key, value) in fields {
        result.insert(key, value);
    }
    result
}

/// Reads a front-matter that strict YAML rejects, one `key: value` line at a
/// time, splitting on the first `:`. `[a, b]` values become lists. Returns
/// `None` when a line is not a plain lower-case key followed by a value.
pub fn flat_front_matter(text: &str) -> Option<Mapping> {
    let mut mapping = Mapping::new();
    for line in raw_front_matter(text)?.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (key, value) = line.split_once(':')?;
        let is_key = !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
        if !is_key {
            return None;
        }
        let value = value.trim();
        let value = match value
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            Some(list) => Value::Sequence(
                list.split(',')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(Value::from)
                    .collect(),
            ),
            None => Value::from(value),
        };
        mapping.insert(Value::from(key), value);
    }
    Some(mapping)
}

/// The raw text between the opening and closing `---`, unparsed.
fn raw_front_matter(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// The raw text of `key`'s value in the front-matter, when it is a plain scalar (not a list, a map or a quoted string).
fn raw_scalar<'a>(front_matter: &'a str, key: &str) -> Option<&'a str> {
    front_matter.lines().find_map(|line| {
        let value = line.strip_prefix(key)?.strip_prefix(':')?.trim();
        (!value.is_empty() && !value.starts_with(['[', '{', '"', '\''])).then_some(value)
    })
}

/// The date of the commit that added the file, as `YYYY-MM-DD`, asked to
/// `git log --diff-filter=A --follow`. The path is passed to git as a raw path,
/// not as UTF-8 text (ADR-0005). `None` when git fails or knows no such commit.
fn added_on(root: &Path, relative: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "log",
            "--diff-filter=A",
            "--follow",
            "--format=%as",
            "-1",
            "--",
        ])
        .arg(relative)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let date = String::from_utf8(output.stdout).ok()?;
    let date = date.trim();
    (!date.is_empty()).then(|| date.to_owned())
}

/// Unit tests of migration 0 → 1.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// A `node.toml` with a two-status workflow.
    const WORKFLOW: &str = "[[workflow]]\nid = \"to-do\"\nlabel = \"To do\"\ncategory = \"todo\"\n\n[[workflow]]\nid = \"shipped\"\nlabel = \"Shipped\"\ncategory = \"done\"\n";

    /// Runs migration 0 → 1 on one file of the repository.
    fn migrate(repo: &Repo, relative: &str) -> Result<Step, Vec<String>> {
        let tree = Tree::load(repo.root()).unwrap();
        let item = tree
            .items
            .iter()
            .find(|item| item.relative.ends_with(relative))
            .unwrap();
        let text = std::fs::read_to_string(&item.file).unwrap();
        ZeroToOne::new(&tree, None).apply(item, &text)
    }

    #[test]
    fn renames_french_keys_orders_the_header_and_keeps_custom_keys_in_place() {
        let repo = Repo::new();
        repo.write("game/node.toml", WORKFLOW).write(
            "game/tasks/T004-forge.md",
            "---\nnode: game\nid: T004\ntitre: Forger une lame\nstatut: to-do\ntype: code\nparent: T001\ndepend: [T002, T003]\nmod: Smithing\nrelease: 1.0\njalon: 0.3\nrefs: [DR 0002]\ntests: [blade_forged]\n---\n\n# T004 — Forger une lame\n\nBody.\n",
        );
        let step = migrate(&repo, "T004-forge.md").unwrap();
        assert_eq!(
            step.text,
            "---\nariane: 1\nnode: game\nid: T004\ntitle: Forger une lame\nstatus: to-do\ntype: code\nparent: T001\ndepends_on:\n- T002\n- T003\nrelease: '1.0'\nmilestone: '0.3'\nrefs:\n- DR 0002\nmod: Smithing\ntests:\n- blade_forged\n---\n\n# T004 — Forger une lame\n\nBody.\n"
        );
        assert!(step.inferred.is_empty());
    }

    #[test]
    fn a_prose_item_gets_an_allocated_id_and_its_heading_as_title() {
        let repo = Repo::new();
        repo.write(
            "game/archives/tasks/old.md",
            "# Polir la garde\n\nDone long ago.\n",
        );
        let step = migrate(&repo, "old.md").unwrap();
        let document = Document::parse(&step.text);
        assert_eq!(document.text("id"), Some("T001"));
        assert_eq!(document.text("title"), Some("Polir la garde"));
        assert_eq!(document.text("status"), Some("done"));
        assert_eq!(step.inferred, vec!["status"]);
    }

    #[test]
    fn a_live_task_without_any_status_needs_a_human() {
        let repo = Repo::new();
        repo.write("game/tasks/new.md", "# Tremper l'acier\n");
        let reasons = migrate(&repo, "new.md").unwrap_err();
        assert_eq!(reasons, vec!["no status found"]);
    }

    #[test]
    fn a_header_disagreeing_with_the_location_needs_a_human() {
        let repo = Repo::new();
        repo.write(
            "game/tasks/T001-x.md",
            "---\nnode: other\nid: T001\ntitre: X\nstatut: todo\n---\n# X\n",
        );
        let reasons = migrate(&repo, "T001-x.md").unwrap_err();
        assert!(reasons[0].contains("lives in `game`"));
    }

    #[test]
    fn a_title_with_a_spaced_colon_is_read_as_a_flat_line() {
        let repo = Repo::new();
        repo.write(
            "game/tasks/T002-voix.md",
            "---\nnode: game\nid: T002\ntitre: Voix : selon le rôle\nstatut: todo\ndepend: [T001]\n---\n# Voix\n",
        );
        let step = migrate(&repo, "T002-voix.md").unwrap();
        let document = Document::parse(&step.text);
        assert_eq!(document.text("title"), Some("Voix : selon le rôle"));
        assert_eq!(document.text("status"), Some("todo"));
        assert_eq!(step.notes.len(), 1);
    }

    #[test]
    fn a_reflexion_on_trial_migrates_to_trial_with_a_reflexion_id() {
        let repo = Repo::new();
        repo.write(
            "game/reflexions/forme.md",
            "# Une forme à l'épreuve\n\n> Ouverte le 2030-01-02. **Statut : 🟡 en essai**\n\nTexte.\n",
        );
        let step = migrate(&repo, "forme.md").unwrap();
        let document = Document::parse(&step.text);
        assert_eq!(document.text("status"), Some("trial"));
        assert_eq!(document.text("id"), Some("R001"));
        assert!(!step.inferred.contains(&"status".to_owned()));
    }

    #[test]
    fn a_ressource_gets_no_status() {
        let repo = Repo::new();
        repo.write("game/ressources/site.md", "# Un site\n");
        let step = migrate(&repo, "site.md").unwrap();
        assert_eq!(Document::parse(&step.text).text("status"), None);
    }

    #[test]
    fn duplicate_french_and_english_keys_need_a_human() {
        let repo = Repo::new();
        repo.write(
            "game/tasks/T001-x.md",
            "---\nid: T001\ntitre: A\ntitle: B\nstatus: todo\n---\n",
        );
        let reasons = migrate(&repo, "T001-x.md").unwrap_err();
        assert_eq!(reasons, vec!["both `titre` and `title` are present"]);
    }
}

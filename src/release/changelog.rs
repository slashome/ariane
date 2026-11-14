//! The raw changelog entry of a release (S043, ADR-0029, SPEC.md §1.5).
//!
//! The entry lists the finished work of the release, archived or not: tasks,
//! and stories or epics not split into tasks, whose status is in the `done`
//! category and whose effective release is the release. It is written in the
//! format every node shares:
//!
//! ```text
//! ## 0.10.0
//!
//! ### Added
//!
//! - **Company page**: about page of the organisation; parent and subsidiaries. (T014, T015)
//!
//! ### Fixed
//!
//! - **Company page**: the tabs of certified results are clickable. (T031)
//! ```
//!
//! The title is the release's name, without a date. A line per subject: the
//! subject of a task is its story, else its epic, else the task itself; the
//! subject of a story or an epic is itself. The work of one subject is merged
//! on its line, its ids at the end; lines come in the order of their subjects:
//! epics, stories, then work of its own, each in the manual order. A section is written only when it has
//! lines: `Fixed` holds the work whose `type` is `bug` or `fix`, `Added`
//! everything else. No `type` names a removal yet, so no section holds them.
//! The headings follow `[changelog] language`. Nothing is written to a file:
//! the text is printed, for the release's pull request or for a human to
//! commit.

use serde::Serialize;

use super::{Listed, listed, shown_id};
use crate::node::Language;
use crate::tree::{Item, NodePath, Tree};

/// The `type` values whose work is a correction.
pub const FIX_TYPES: [&str; 2] = ["bug", "fix"];

/// The kind of a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    /// New work.
    Added,
    /// Corrections: work whose `type` is `bug` or `fix`.
    Fixed,
}

impl SectionKind {
    /// The heading of the section in `language`.
    pub fn heading(self, language: Language) -> &'static str {
        match (self, language) {
            (SectionKind::Added, Language::En) => "Added",
            (SectionKind::Fixed, Language::En) => "Fixed",
            (SectionKind::Added, Language::Fr) => "Nouveautés",
            (SectionKind::Fixed, Language::Fr) => "Corrections",
        }
    }
}

/// One finished piece of work of the release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    /// `<node>#<id>`.
    pub reference: String,
    /// The id the line shows: bare in the release's node, else `<node>#<id>`.
    pub id: String,
    /// Its title.
    pub title: String,
    /// Whether it sits under `archives/`.
    pub archived: bool,
}

/// One line: a subject and the work of the release it groups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Line {
    /// The title of the story, the epic or the work itself.
    pub subject: String,
    /// The item that gives the subject, `<node>#<id>`.
    pub subject_reference: String,
    /// Its work, in order.
    pub changes: Vec<Change>,
}

/// One section of the entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    /// What it holds.
    pub kind: SectionKind,
    /// Its heading in the entry's language.
    pub heading: String,
    /// Its lines, in the order of their first piece of work.
    pub lines: Vec<Line>,
}

/// The changelog entry of a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// The node that declares the release list.
    pub node: NodePath,
    /// The release's name, the entry's title.
    pub release: String,
    /// The language of its headings.
    pub language: Language,
    /// The sections that have lines, `Added` first.
    pub sections: Vec<Section>,
    /// The entry in Markdown, as printed.
    pub markdown: String,
}

/// The changelog entry of the release `name` of the list `node` declares.
pub fn entry(tree: &Tree, node: &NodePath, name: &str) -> Entry {
    let language = tree.configs.effective(node).changelog.language;
    let work = listed(tree, node);
    let mut sections: Vec<Section> = Vec::new();
    for kind in [SectionKind::Added, SectionKind::Fixed] {
        let mut lines: Vec<Line> = Vec::new();
        for listed in work.iter().filter(|listed| {
            listed.release == Some(name)
                && listed.is_done()
                && listed.is_work()
                && section_of(listed.item) == kind
        }) {
            let subject = subject(listed);
            let reference = subject.reference().unwrap_or_default();
            let change = Change {
                reference: listed.item.reference().unwrap_or_default(),
                id: shown_id(listed.item, node),
                title: title(listed.item),
                archived: listed.item.archived,
            };
            match lines
                .iter_mut()
                .find(|line| line.subject_reference == reference)
            {
                Some(line) => line.changes.push(change),
                None => lines.push(Line {
                    subject: title(subject),
                    subject_reference: reference,
                    changes: vec![change],
                }),
            }
        }
        let place = |reference: &str| {
            work.iter()
                .position(|listed| listed.item.reference().as_deref() == Some(reference))
                .unwrap_or(usize::MAX)
        };
        lines.sort_by_cached_key(|line| place(&line.subject_reference));
        if !lines.is_empty() {
            sections.push(Section {
                kind,
                heading: kind.heading(language).to_owned(),
                lines,
            });
        }
    }
    let markdown = render(name, language, &sections);
    Entry {
        node: node.clone(),
        release: name.to_owned(),
        language,
        sections,
        markdown,
    }
}

/// The section a finished piece of work goes to, by its own `type`.
fn section_of(item: &Item) -> SectionKind {
    match item.document.text("type") {
        Some(kind) if FIX_TYPES.contains(&kind) => SectionKind::Fixed,
        _ => SectionKind::Added,
    }
}

/// The item whose title is the subject of `listed`: a task's story, else its
/// epic, else the work itself.
fn subject<'a>(listed: &Listed<'a>) -> &'a Item {
    match listed.parent {
        Some(parent) if listed.item.kind == crate::item::Kind::Task => parent,
        _ => listed.item,
    }
}

/// The item's title, else its id.
fn title(item: &Item) -> String {
    item.document
        .text("title")
        .or_else(|| item.id())
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// `text` closed by a full stop, unless it already ends with a punctuation
/// mark that ends a sentence.
fn closed_sentence(text: &str) -> String {
    let trimmed = text.trim_end();
    if trimmed.ends_with(['.', '?', '!', '…']) {
        trimmed.to_owned()
    } else {
        format!("{trimmed}.")
    }
}

/// The entry in Markdown.
fn render(name: &str, language: Language, sections: &[Section]) -> String {
    let colon = match language {
        Language::En => ": ",
        Language::Fr => " : ",
    };
    let mut out = format!("## {name}\n");
    for section in sections {
        out.push_str(&format!("\n### {}\n\n", section.heading));
        for line in &section.lines {
            let ids: Vec<&str> = line
                .changes
                .iter()
                .map(|change| change.id.as_str())
                .collect();
            let titles: Vec<&str> = line
                .changes
                .iter()
                .filter(|change| change.reference != line.subject_reference)
                .map(|change| change.title.trim_end_matches('.').trim_end())
                .collect();
            let body = if titles.is_empty() {
                let subject = line.subject.trim_end_matches('.');
                let stop = if subject.ends_with(['?', '!', '…']) {
                    ""
                } else {
                    "."
                };
                format!("**{subject}**{stop}")
            } else {
                format!(
                    "**{}**{colon}{}",
                    line.subject,
                    closed_sentence(&titles.join("; "))
                )
            };
            out.push_str(&format!("- {body} ({})\n", ids.join(", ")));
        }
    }
    out
}

/// Unit tests of the changelog entry.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeFile;
    use crate::tree::fixture::Repo;

    /// An item of `node` with `id`, `title`, `status` and extra header lines.
    fn item(node: &str, id: &str, title: &str, status: &str, extra: &str) -> String {
        format!(
            "---\nariane: 2\nnode: {node}\nid: {id}\ntitle: '{title}'\nstatus: {status}\n{extra}---\n# {id}\n"
        )
    }

    /// `p` declares `0.1` and `0.2`; an epic spans both; a story of `0.1` has
    /// two done tasks, one of them archived, a bug and an open task; a story
    /// not split is done in `0.1`; a task without parent is done; `p/c`
    /// inherits the list.
    fn sample(node_toml: &str) -> Repo {
        let repo = Repo::new();
        repo.write("p/node.toml", node_toml)
            .write(
                "p/epics/E1-e.md",
                &item("p", "E1", "Company page", "todo", ""),
            )
            .write(
                "p/stories/S1-a.md",
                &item(
                    "p",
                    "S1",
                    "About page",
                    "todo",
                    "parent: E1\nrelease: '0.1'\n",
                ),
            )
            .write(
                "p/tasks/T1-a.md",
                &item("p", "T1", "Show the founders.", "done", "parent: S1\n"),
            )
            .write(
                "p/archives/tasks/T2-b.md",
                &item("p", "T2", "Show the subsidiaries", "done", "parent: S1\n"),
            )
            .write(
                "p/tasks/T3-c.md",
                &item(
                    "p",
                    "T3",
                    "Tabs are clickable",
                    "done",
                    "parent: S1\ntype: bug\n",
                ),
            )
            .write(
                "p/tasks/T4-d.md",
                &item("p", "T4", "Not yet", "todo", "parent: S1\n"),
            )
            .write(
                "p/tasks/T5-e.md",
                &item("p", "T5", "Later", "done", "parent: S1\nrelease: '0.2'\n"),
            )
            .write(
                "p/stories/S2-b.md",
                &item("p", "S2", "Search", "done", "parent: E1\nrelease: '0.1'\n"),
            )
            .write(
                "p/tasks/T6-f.md",
                &item(
                    "p",
                    "T6",
                    "Is it done?",
                    "done",
                    "release: '0.1'\ntype: fix\n",
                ),
            )
            .write(
                "p/tasks/T7-g.md",
                &item("p", "T7", "Dropped", "dropped", "release: '0.1'\n"),
            )
            .write(
                "p/c/tasks/T1-c.md",
                &item("p/c", "T1", "Child work", "done", "parent: p#S1\n"),
            );
        repo
    }

    const LIST: &str = "[[release]]\nname = \"0.1\"\n\n[[release]]\nname = \"0.2\"\n";

    #[test]
    fn the_entry_groups_the_done_work_by_subject_in_sections_with_ids_at_line_end() {
        let repo = sample(LIST);
        let tree = Tree::load(repo.root()).unwrap();
        let entry = entry(&tree, &NodePath::new("p"), "0.1");
        assert_eq!(
            entry.markdown,
            "## 0.1\n\n### Added\n\n- **About page**: Show the founders; Show the subsidiaries; Child work. (T1, T2, p/c#T1)\n- **Search**. (S2)\n\n### Fixed\n\n- **About page**: Tabs are clickable. (T3)\n- **Is it done?** (T6)\n"
        );
        assert_eq!(entry.language, Language::En);
        assert_eq!(entry.sections[0].lines[0].subject_reference, "p#S1");
        assert!(entry.sections[0].lines[0].changes[1].archived);
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["sections"][1]["kind"], "fixed");
        assert_eq!(json["language"], "en");
    }

    #[test]
    fn the_headings_follow_the_inherited_language_and_an_empty_release_has_only_its_title() {
        let text = format!("[changelog]\nlanguage = \"fr\"\naudience = \"user\"\n\n{LIST}");
        assert!(toml::from_str::<NodeFile>(&text).is_ok());
        let repo = sample(&text);
        let tree = Tree::load(repo.root()).unwrap();
        let entry = entry(&tree, &NodePath::new("p"), "0.1");
        assert!(
            entry
                .markdown
                .starts_with("## 0.1\n\n### Nouveautés\n\n- **About page** : Show the founders; "),
            "{}",
            entry.markdown
        );
        assert!(entry.markdown.contains("\n### Corrections\n\n"));
        let later = super::entry(&tree, &NodePath::new("p"), "0.2");
        assert_eq!(
            later.markdown,
            "## 0.2\n\n### Nouveautés\n\n- **About page** : Later. (T5)\n"
        );
        let none = super::entry(&tree, &NodePath::new("p"), "9.9");
        assert_eq!(none.markdown, "## 9.9\n");
        assert!(none.sections.is_empty());
    }
}

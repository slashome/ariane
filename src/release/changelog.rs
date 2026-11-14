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
//!
//! ### Removed
//!
//! - **Legacy export**. (T040)
//! ```
//!
//! The title is the release's name, without a date. A line per subject: the
//! largest unit the release delivers in full. That is the epic when every
//! piece of work under it, archived included and dropped excluded, is done
//! with the release as its effective release; else the story, on the same
//! test; else the work itself, under its own title. The work of one subject
//! is merged on its line, its ids at the end; lines come in the order of their
//! subjects: epics, stories, then single pieces of work, each in the order of
//! items. A section is written only when it has lines: `Added`, `Fixed` for
//! the work whose `type` is `bug` or `fix`, then `Removed` for the work whose
//! `type` is `removal`; `Added` holds everything else.
//! The headings follow `[changelog] language`. Nothing is written to a file:
//! the text is printed, for the release's pull request or for a human to
//! commit.

use std::collections::HashMap;

use serde::Serialize;

use super::{Listed, listed, shown_id};
use crate::item::Kind;
use crate::node::{Category, Language};
use crate::tree::{Item, NodePath, Tree};

/// The `type` values whose work is a correction.
pub const FIX_TYPES: [&str; 2] = ["bug", "fix"];

/// The `type` values whose work takes something away.
pub const REMOVAL_TYPES: [&str; 1] = ["removal"];

/// The kind of a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    /// New work.
    Added,
    /// Corrections: work whose `type` is `bug` or `fix`.
    Fixed,
    /// Removals: work whose `type` is `removal`.
    Removed,
}

impl SectionKind {
    /// The heading of the section in `language`.
    pub fn heading(self, language: Language) -> &'static str {
        match (self, language) {
            (SectionKind::Added, Language::En) => "Added",
            (SectionKind::Fixed, Language::En) => "Fixed",
            (SectionKind::Removed, Language::En) => "Removed",
            (SectionKind::Added, Language::Fr) => "Nouveautés",
            (SectionKind::Fixed, Language::Fr) => "Corrections",
            (SectionKind::Removed, Language::Fr) => "Retraits",
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
    /// The title of the epic, the story or the work itself.
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
    /// Its lines: epics, stories, then single pieces of work, each in the
    /// order of items.
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
    /// The sections that have lines: `Added`, `Fixed`, then `Removed`.
    pub sections: Vec<Section>,
    /// The entry in Markdown, as printed.
    pub markdown: String,
}

/// The changelog entry of the release `name` of the list `node` declares.
pub fn entry(tree: &Tree, node: &NodePath, name: &str) -> Entry {
    let language = tree.configs.effective(node).changelog.language;
    let work = listed(tree, node);
    let release = Release::new(&work, name);
    let mut sections: Vec<Section> = Vec::new();
    for kind in [SectionKind::Added, SectionKind::Fixed, SectionKind::Removed] {
        let mut lines: Vec<(Rank, Line)> = Vec::new();
        for listed in work.iter().filter(|listed| {
            release.delivers(listed) && listed.is_work() && section_of(listed.item) == kind
        }) {
            let (rank, subject) = release.subject(listed);
            let reference = subject.reference().unwrap_or_default();
            let change = Change {
                reference: listed.item.reference().unwrap_or_default(),
                id: shown_id(listed.item, node),
                title: title(listed.item),
                archived: listed.item.archived,
            };
            match lines
                .iter_mut()
                .find(|(_, line)| line.subject_reference == reference)
            {
                Some((_, line)) => line.changes.push(change),
                None => lines.push((
                    rank,
                    Line {
                        subject: title(subject),
                        subject_reference: reference,
                        changes: vec![change],
                    },
                )),
            }
        }
        lines.sort_by_key(|(rank, _)| *rank);
        if !lines.is_empty() {
            sections.push(Section {
                kind,
                heading: kind.heading(language).to_owned(),
                lines: lines.into_iter().map(|(_, line)| line).collect(),
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

/// The place of a line: the kind of its subject, epics first, then stories,
/// then single pieces of work, and the subject's place in the order of items.
type Rank = (u8, usize);

/// The work of one release, read from the items of its list.
struct Release<'w, 'a> {
    /// The items of the list, in the order of items.
    work: &'w [Listed<'a>],
    /// The release's name.
    name: &'w str,
    /// The place of each item in `work`, by address.
    places: HashMap<*const Item, usize>,
}

impl<'w, 'a> Release<'w, 'a> {
    /// The release `name` of the list whose items are `work`.
    fn new(work: &'w [Listed<'a>], name: &'w str) -> Self {
        let places = work
            .iter()
            .enumerate()
            .map(|(place, listed)| (std::ptr::from_ref(listed.item), place))
            .collect();
        Self { work, name, places }
    }

    /// Tells whether the release delivers `listed`: it is done and its
    /// effective release is the release.
    fn delivers(&self, listed: &Listed<'_>) -> bool {
        listed.release == Some(self.name) && listed.is_done()
    }

    /// The listed record of `item`, when it plans against the list.
    fn find(&self, item: &Item) -> Option<(usize, &'w Listed<'a>)> {
        let place = *self.places.get(&std::ptr::from_ref(item))?;
        Some((place, &self.work[place]))
    }

    /// Tells whether the release delivers all of `unit`: every piece of work
    /// under it, archived included and dropped excluded, is delivered by it.
    fn delivers_all(&self, unit: &Listed<'_>) -> bool {
        unit.children
            .iter()
            .filter(|child| matches!(child.kind, Kind::Task | Kind::Story | Kind::Epic))
            .all(|child| match self.find(child) {
                Some((_, listed)) if listed.category == Some(Category::Dropped) => true,
                Some((_, listed)) if listed.is_work() => self.delivers(listed),
                Some((_, listed)) => self.delivers_all(listed),
                None => false,
            })
    }

    /// The subject of the delivered work `listed`, with the place of its
    /// line: the largest unit above it the release delivers in full, its
    /// epic, else its story, else the work itself.
    fn subject(&self, listed: &'w Listed<'a>) -> (Rank, &'a Item) {
        let mut subject = self.find(listed.item).unwrap_or((usize::MAX, listed));
        let mut above = listed.parent;
        while let Some((place, unit)) = above.and_then(|parent| self.find(parent)) {
            if !matches!(unit.item.kind, Kind::Story | Kind::Epic) {
                break;
            }
            if self.delivers_all(unit) {
                subject = (place, unit);
            }
            above = unit.parent;
        }
        let (place, unit) = subject;
        let kind = match unit.item.kind {
            Kind::Epic => 0,
            Kind::Story => 1,
            _ => 2,
        };
        ((kind, place), unit.item)
    }
}

/// The section a finished piece of work goes to, by its own `type`.
fn section_of(item: &Item) -> SectionKind {
    match item.document.text("type") {
        Some(kind) if FIX_TYPES.contains(&kind) => SectionKind::Fixed,
        Some(kind) if REMOVAL_TYPES.contains(&kind) => SectionKind::Removed,
        _ => SectionKind::Added,
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

    /// `p` declares `0.1` and `0.2`, `p/c` inherits the list.
    ///
    /// - `E1` is delivered in full by `0.1`: its story `S1` has two done
    ///   tasks, one archived, a done bug, a dropped task and a done task of
    ///   `p/c`; its story `S2`, not split, is done.
    /// - `E2` spans both releases: its story `S3` is delivered by `0.1`, its
    ///   story `S4` by `0.2`.
    /// - `S5`, ranked before `S3`, is delivered in part by `0.1`: one task
    ///   done, one open.
    /// - `T6` is a fix and `T13` a removal, both without parent.
    fn sample(node_toml: &str) -> Repo {
        let repo = Repo::new();
        let files = [
            (
                "p/epics/E1-e.md",
                item("p", "E1", "Company page", "todo", "rank: 2\n"),
            ),
            (
                "p/epics/E2-e.md",
                item("p", "E2", "Search", "todo", "rank: 1\n"),
            ),
            (
                "p/stories/S1-a.md",
                item(
                    "p",
                    "S1",
                    "About page",
                    "todo",
                    "parent: E1\nrelease: '0.1'\n",
                ),
            ),
            (
                "p/tasks/T1-a.md",
                item("p", "T1", "Show the founders.", "done", "parent: S1\n"),
            ),
            (
                "p/archives/tasks/T2-b.md",
                item("p", "T2", "Show the subsidiaries", "done", "parent: S1\n"),
            ),
            (
                "p/tasks/T3-c.md",
                item(
                    "p",
                    "T3",
                    "Tabs are clickable",
                    "done",
                    "parent: S1\ntype: bug\n",
                ),
            ),
            (
                "p/tasks/T7-g.md",
                item("p", "T7", "Dropped", "dropped", "parent: S1\n"),
            ),
            (
                "p/c/tasks/T1-c.md",
                item("p/c", "T1", "Child work", "done", "parent: p#S1\n"),
            ),
            (
                "p/stories/S2-b.md",
                item(
                    "p",
                    "S2",
                    "Contact form",
                    "done",
                    "parent: E1\nrelease: '0.1'\n",
                ),
            ),
            (
                "p/stories/S3-c.md",
                item(
                    "p",
                    "S3",
                    "Search by name",
                    "todo",
                    "parent: E2\nrelease: '0.1'\nrank: 2\n",
                ),
            ),
            (
                "p/tasks/T8-h.md",
                item("p", "T8", "Index names", "done", "parent: S3\n"),
            ),
            (
                "p/tasks/T9-i.md",
                item("p", "T9", "Search box", "done", "parent: S3\n"),
            ),
            (
                "p/stories/S4-d.md",
                item(
                    "p",
                    "S4",
                    "Search by city",
                    "todo",
                    "parent: E2\nrelease: '0.2'\n",
                ),
            ),
            (
                "p/tasks/T10-j.md",
                item("p", "T10", "Index cities", "done", "parent: S4\n"),
            ),
            (
                "p/stories/S5-e.md",
                item("p", "S5", "Reviews", "todo", "release: '0.1'\nrank: 1\n"),
            ),
            (
                "p/tasks/T11-k.md",
                item("p", "T11", "Show the stars", "done", "parent: S5\n"),
            ),
            (
                "p/tasks/T12-l.md",
                item("p", "T12", "Not yet", "todo", "parent: S5\n"),
            ),
            (
                "p/tasks/T6-f.md",
                item(
                    "p",
                    "T6",
                    "Is it done?",
                    "done",
                    "release: '0.1'\ntype: fix\n",
                ),
            ),
            (
                "p/tasks/T13-m.md",
                item(
                    "p",
                    "T13",
                    "Legacy export",
                    "done",
                    "release: '0.1'\ntype: removal\n",
                ),
            ),
        ];
        repo.write("p/node.toml", node_toml);
        for (path, text) in &files {
            repo.write(path, text);
        }
        repo
    }

    const LIST: &str = "[[release]]\nname = \"0.1\"\n\n[[release]]\nname = \"0.2\"\n";

    #[test]
    fn each_line_has_the_largest_unit_the_release_delivers_in_full_as_subject() {
        let repo = sample(LIST);
        let tree = Tree::load(repo.root()).unwrap();
        let entry = entry(&tree, &NodePath::new("p"), "0.1");
        assert_eq!(
            entry.markdown,
            "## 0.1\n\n### Added\n\n\
             - **Company page**: Contact form; Show the founders; Show the subsidiaries; Child work. (S2, T1, T2, p/c#T1)\n\
             - **Search by name**: Index names; Search box. (T8, T9)\n\
             - **Show the stars**. (T11)\n\n\
             ### Fixed\n\n\
             - **Company page**: Tabs are clickable. (T3)\n\
             - **Is it done?** (T6)\n\n\
             ### Removed\n\n\
             - **Legacy export**. (T13)\n"
        );
        assert_eq!(entry.language, Language::En);
        assert_eq!(entry.sections[0].lines[0].subject_reference, "p#E1");
        assert_eq!(entry.sections[0].lines[1].subject_reference, "p#S3");
        assert_eq!(entry.sections[0].lines[2].subject_reference, "p#T11");
        assert!(entry.sections[0].lines[0].changes[2].archived);
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["sections"][1]["kind"], "fixed");
        assert_eq!(json["sections"][2]["kind"], "removed");
        assert_eq!(json["language"], "en");
    }

    #[test]
    fn an_epic_spanning_two_releases_gives_story_lines_in_each() {
        let repo = sample(LIST);
        let tree = Tree::load(repo.root()).unwrap();
        let later = entry(&tree, &NodePath::new("p"), "0.2");
        assert_eq!(
            later.markdown,
            "## 0.2\n\n### Added\n\n- **Search by city**: Index cities. (T10)\n"
        );
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
                .starts_with("## 0.1\n\n### Nouveautés\n\n- **Company page** : Contact form; "),
            "{}",
            entry.markdown
        );
        assert!(entry.markdown.contains("\n### Corrections\n\n"));
        assert!(
            entry
                .markdown
                .ends_with("\n### Retraits\n\n- **Legacy export**. (T13)\n")
        );
        let none = super::entry(&tree, &NodePath::new("p"), "9.9");
        assert_eq!(none.markdown, "## 9.9\n");
        assert!(none.sections.is_empty());
    }
}

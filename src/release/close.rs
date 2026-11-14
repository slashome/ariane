//! Closing a release: `ariane release close` and `POST /api/releases/close`
//! (ADR-0029, SPEC.md §1.5 "Closing a release").
//!
//! A release is closed once its work is delivered. Closing refuses while a
//! task of the release, or a story or an epic of it not split yet, is open,
//! and lists them; on request it carries that work over to the next release
//! instead, writing `release` on each item, so the slip stays in git. Then it
//! moves the finished work of the release (done or dropped) under
//! `archives/`, every link to it rewritten the way `ariane fix` rewrites them,
//! and with it each story and epic that is closed and has nothing left outside
//! `archives/`; a story or an epic that still has work elsewhere stays, and the
//! plan says why. Last, it writes `closed = YYYY-MM-DD` on the release's entry
//! of `node.toml`. A dry run computes the same plan and writes nothing; it
//! does not refuse on open work, it reports it. Nothing is committed, and
//! nothing is written to the host repository: the changelog entry is only
//! given back.

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use serde::Serialize;
use toml_edit::value;

use super::changelog::{self, Entry};
use super::declare::{self, Refusal};
use super::{Listed, listed, shown_id};
use crate::fix;
use crate::item::Kind;
use crate::node::{FILE as NODE_FILE, Release};
use crate::tree::{Item, NodePath, Tree};

/// What closing a release asks for.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// The node that declares the release list.
    pub node: &'a NodePath,
    /// The release to close.
    pub name: &'a str,
    /// Moves the open work to the next release instead of refusing.
    pub carry_over: bool,
    /// Computes the plan and writes nothing.
    pub dry_run: bool,
    /// The closing date, `YYYY-MM-DD`.
    pub today: &'a str,
}

/// A piece of work the plan names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Work {
    /// `<node>#<id>`.
    pub reference: String,
    /// The id as the release's node shows it.
    pub id: String,
    /// Its kind.
    pub kind: Kind,
    /// Its title.
    pub title: String,
    /// Its status, as written.
    pub status: Option<String>,
}

/// An item moved under `archives/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Archived {
    /// The item.
    #[serde(flatten)]
    pub work: Work,
    /// Its path before, relative to the repository root.
    pub from: String,
    /// Its path under `archives/`.
    pub to: String,
    /// How many references to it are rewritten.
    pub references: usize,
    /// In how many files.
    pub files: usize,
    /// Files that still mention its old name in prose, to read by hand.
    pub mentioned: Vec<String>,
}

/// A story or an epic of the release left outside `archives/`, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Kept {
    /// The item.
    #[serde(flatten)]
    pub work: Work,
    /// Why it stays.
    pub reason: String,
}

/// The plan of a close, and what it did once applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Closing {
    /// The node that declares the release list.
    pub node: NodePath,
    /// The release.
    pub release: String,
    /// The closing date written, or to be.
    pub closed: String,
    /// Whether nothing was written.
    pub dry_run: bool,
    /// The open work of the release.
    pub open: Vec<Work>,
    /// Whether the open work is carried over.
    pub carry_over: bool,
    /// The release the open work goes to with a carry-over: the next one of
    /// the list neither parking nor closed.
    pub carry_to: Option<String>,
    /// Whether the close is refused as asked: open work and no carry-over.
    pub blocked: bool,
    /// What goes under `archives/`.
    pub archived: Vec<Archived>,
    /// The stories and epics of the release that stay outside `archives/`.
    pub kept: Vec<Kept>,
    /// The changelog entry of the release.
    pub changelog: Entry,
}

/// The work as the plan names it.
fn work(item: &Item, node: &NodePath) -> Work {
    Work {
        reference: item.reference().unwrap_or_default(),
        id: shown_id(item, node),
        kind: item.kind,
        title: item
            .document
            .text("title")
            .or_else(|| item.id())
            .unwrap_or_default()
            .to_owned(),
        status: item.document.text("status").map(str::to_owned),
    }
}

/// The directory of `archives/` an item of `kind` goes to.
fn archive_dir(kind: Kind) -> &'static str {
    match kind {
        Kind::Epic => "epics",
        Kind::Story => "stories",
        _ => "tasks",
    }
}

/// The release `name` of the list `node` declares, refusing an inherited
/// list, an unknown, parking or closed release; with the next release open
/// work can go to.
fn release<'a>(
    tree: &Tree,
    node: &NodePath,
    name: &str,
    releases: &'a [Release],
) -> Result<(&'a Release, Option<&'a Release>), Refusal> {
    declare::check_declarer(tree, node)?;
    let Some(index) = releases.iter().position(|release| release.name == name) else {
        return Err(Refusal::Invalid(format!(
            "no release `{name}` declared in `{node}`"
        )));
    };
    let release = &releases[index];
    if release.parking {
        return Err(Refusal::Invalid(format!(
            "`{name}` is a parking release: it is never closed"
        )));
    }
    if let Some(closed) = &release.closed {
        return Err(Refusal::Invalid(format!(
            "`{name}` was already closed on {closed}"
        )));
    }
    let next = releases[index + 1..]
        .iter()
        .find(|release| !release.parking && release.closed.is_none());
    Ok((release, next))
}

/// The plan, and the moves it makes as `(from, to)`.
type Planned<'a> = (Closing, Vec<(PathBuf, PathBuf)>, Vec<&'a Item>);

/// Computes the plan of closing `request.name`: the open work, what goes under
/// `archives/` and what stays, and the changelog entry; also the moves and
/// the open items. Writes nothing.
fn plan<'a>(tree: &'a Tree, request: &Request) -> Result<Result<Planned<'a>, Refusal>> {
    let node = request.node;
    let name = request.name;
    let (_, releases) = declare::effective_list(tree, node);
    let next = match release(tree, node, name, &releases) {
        Ok((_, next)) => next.map(|release| release.name.clone()),
        Err(refusal) => return Ok(Err(refusal)),
    };
    if request.carry_over && next.is_none() {
        return Ok(Err(Refusal::Invalid(format!(
            "no release after `{name}` to carry its open work over to: add one first"
        ))));
    }
    let work_list = listed(tree, node);
    let in_release = |listed: &Listed| listed.release == Some(name);
    let open: Vec<&Item> = work_list
        .iter()
        .filter(|listed| {
            in_release(listed) && !listed.item.archived && listed.is_work() && !listed.is_closed()
        })
        .map(|listed| listed.item)
        .collect();
    let mut archiving: Vec<&Item> = work_list
        .iter()
        .filter(|listed| {
            in_release(listed) && !listed.item.archived && listed.is_work() && listed.is_closed()
        })
        .map(|listed| listed.item)
        .collect();
    let mut kept = Vec::new();
    for kind in [Kind::Story, Kind::Epic] {
        for listed in work_list
            .iter()
            .filter(|listed| listed.item.kind == kind && !listed.item.archived && !listed.is_work())
        {
            let moving: HashSet<*const Item> = archiving
                .iter()
                .map(|item| std::ptr::from_ref(*item))
                .collect();
            let goes = |child: &&Item| moving.contains(&std::ptr::from_ref(*child));
            if !in_release(listed) && !listed.children.iter().any(goes) {
                continue;
            }
            let outside: Vec<String> = listed
                .children
                .iter()
                .filter(|child| !child.archived && !goes(child))
                .map(|child| shown_id(child, node))
                .collect();
            let reason = if !outside.is_empty() {
                Some(format!(
                    "work left outside archives: {}",
                    outside.join(", ")
                ))
            } else if !listed.is_closed() {
                Some(format!(
                    "all its work is archived, but its status `{}` is not closed",
                    listed.item.document.text("status").unwrap_or_default()
                ))
            } else {
                None
            };
            match reason {
                Some(reason) => kept.push(Kept {
                    work: work(listed.item, node),
                    reason,
                }),
                None => archiving.push(listed.item),
            }
        }
    }
    let mut moves = Vec::new();
    for item in &archiving {
        let Some(file_name) = item.file.file_name() else {
            continue;
        };
        let target = tree
            .node_dir(&item.node)
            .join("archives")
            .join(archive_dir(item.kind))
            .join(file_name);
        if target.exists() {
            return Ok(Err(Refusal::Invalid(format!(
                "{} already exists: {} cannot be archived",
                relative(tree, &target),
                shown_id(item, node)
            ))));
        }
        moves.push((item.file.clone(), target));
    }
    let (references, _) = fix::plan_relocation(tree, &moves)?;
    let archived = archiving
        .iter()
        .zip(&moves)
        .zip(references)
        .map(|((item, (from, to)), found)| Archived {
            work: work(item, node),
            from: relative(tree, from),
            to: relative(tree, to),
            references: found.rewritten.values().sum(),
            files: found.rewritten.len(),
            mentioned: found.mentioned,
        })
        .collect();
    let closing = Closing {
        node: node.clone(),
        release: name.to_owned(),
        closed: request.today.to_owned(),
        dry_run: request.dry_run,
        open: open.iter().map(|item| work(item, node)).collect(),
        carry_over: request.carry_over,
        carry_to: next,
        blocked: !open.is_empty() && !request.carry_over,
        archived,
        kept,
        changelog: changelog::entry(tree, node, name),
    };
    Ok(Ok((closing, moves, open)))
}

/// `path` relative to the repository root.
fn relative(tree: &Tree, path: &std::path::Path) -> String {
    path.strip_prefix(&tree.root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// Closes the release `request.name` of the list `request.node` declares, or
/// only plans it on a dry run.
///
/// # Errors
///
/// Fails when a file cannot be read, written or moved; refuses an inherited
/// list, an unknown, parking or closed release, a carry-over without a next
/// release, a file already under `archives/`, and, unless it is a dry run or
/// a carry-over, a release with open work.
pub fn close(tree: &Tree, request: &Request) -> Result<Result<Closing, Refusal>> {
    let (closing, moves, open) = match plan(tree, request)? {
        Ok(planned) => planned,
        Err(refusal) => return Ok(Err(refusal)),
    };
    if request.dry_run {
        return Ok(Ok(closing));
    }
    if closing.blocked {
        return Ok(Err(Refusal::Open {
            release: closing.release,
            items: closing.open.into_iter().map(|work| work.id).collect(),
        }));
    }
    if let Some(next) = closing.carry_to.as_deref().filter(|_| request.carry_over) {
        for item in &open {
            if let Err(refusal) = declare::assign(tree, item, Some(next))? {
                return Ok(Err(refusal));
            }
        }
    }
    let (_, texts) = fix::plan_relocation(tree, &moves)?;
    fix::relocate(&texts, &moves)?;
    let date: toml_edit::Datetime = request.today.parse()?;
    let written = crate::node::edit(&tree.node_dir(request.node).join(NODE_FILE), |document| {
        let entry = document
            .get_mut("release")
            .and_then(toml_edit::Item::as_array_of_tables_mut)
            .and_then(|entries| {
                entries.iter_mut().find(|entry| {
                    entry.get("name").and_then(toml_edit::Item::as_str) == Some(request.name)
                })
            });
        match entry {
            Some(entry) => {
                entry.insert("closed", value(date));
                Ok(())
            }
            None => Err(Refusal::Invalid(format!(
                "no release `{}` in the node.toml of `{}`",
                request.name, request.node
            ))),
        }
    })?;
    if let Err(refusal) = written {
        return Ok(Err(refusal));
    }
    Ok(Ok(closing))
}

/// Unit tests of closing a release.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// An item of `p` with `id`, `status` and extra header lines.
    fn item(id: &str, status: &str, extra: &str) -> String {
        format!(
            "---\nariane: 2\nnode: p\nid: {id}\ntitle: {id} title\nstatus: {status}\n{extra}---\n# {id}\n"
        )
    }

    const NODE: &str = "# plan\n[[release]]\nname = \"0.1\"\n\n[[release]]\nname = \"0.2\"\nclosed = 2026-10-01\n\n[[release]]\nname = \"0.3\"\n\n[[release]]\nname = \"later\"\nparking = true\n";

    /// `p` declares `0.1`, a closed `0.2`, `0.3` and a parking `later`. Epic
    /// `E1` spans `0.1` and `0.3`; story `S1` (in `0.1`) has two done tasks
    /// and a dropped one, one linked from `S1`; story `S2` of `0.3` has a
    /// task done in `0.1`; epic `E2` and its story `S3` are done in `0.1`;
    /// story `S4` is open with all its tasks done in `0.1`; `T9` is an open
    /// task of `0.1`, `S5` an open story of `0.1` not split.
    fn sample() -> Repo {
        let repo = Repo::new();
        repo.write("p/node.toml", NODE)
            .write("p/epics/E1-e.md", &item("E1", "todo", "release: '0.1'\n"))
            .write(
                "p/stories/S1-s.md",
                &format!(
                    "{}\nSee [T1](../tasks/T1-a.md).\n",
                    item("S1", "done", "parent: E1\n")
                ),
            )
            .write(
                "p/tasks/T1-a.md",
                &format!(
                    "{}\nNext: [S2](../stories/S2-s.md#scope), `T5-e.md`.\n",
                    item("T1", "done", "parent: S1\n")
                ),
            )
            .write(
                "p/tasks/T2-b.md",
                &item("T2", "done", "parent: S1\ntype: bug\n"),
            )
            .write("p/tasks/T3-c.md", &item("T3", "dropped", "parent: S1\n"))
            .write(
                "p/stories/S2-s.md",
                &item("S2", "todo", "parent: E1\nrelease: '0.3'\n"),
            )
            .write(
                "p/tasks/T4-d.md",
                &item("T4", "done", "parent: S2\nrelease: '0.1'\n"),
            )
            .write("p/tasks/T5-e.md", &item("T5", "todo", "parent: S2\n"))
            .write("p/epics/E2-e.md", &item("E2", "done", ""))
            .write(
                "p/stories/S3-s.md",
                &item("S3", "done", "parent: E2\nrelease: '0.1'\n"),
            )
            .write("p/tasks/T6-f.md", &item("T6", "done", "parent: S3\n"))
            .write("p/stories/S4-s.md", &item("S4", "todo", ""))
            .write(
                "p/tasks/T7-g.md",
                &item("T7", "done", "parent: S4\nrelease: '0.1'\n"),
            )
            .write(
                "p/README.md",
                "Links: [T1](tasks/T1-a.md), [T6](./tasks/T6-f.md).\n",
            );
        repo
    }

    /// `sample` with open work in `0.1`: a task and a story not split.
    fn with_open_work() -> Repo {
        let repo = sample();
        repo.write("p/tasks/T9-z.md", &item("T9", "todo", "parent: S1\n"))
            .write(
                "p/stories/S5-s.md",
                &item("S5", "in_progress", "release: '0.1'\n"),
            );
        repo
    }

    fn tree(repo: &Repo) -> Tree {
        Tree::load(repo.root()).unwrap()
    }

    fn request<'a>(
        node: &'a NodePath,
        name: &'a str,
        carry_over: bool,
        dry_run: bool,
    ) -> Request<'a> {
        Request {
            node,
            name,
            carry_over,
            dry_run,
            today: "2026-11-14",
        }
    }

    fn read(repo: &Repo, relative: &str) -> String {
        std::fs::read_to_string(repo.root().join("tree").join(relative)).unwrap()
    }

    fn exists(repo: &Repo, relative: &str) -> bool {
        repo.root().join("tree").join(relative).exists()
    }

    fn ids(works: impl IntoIterator<Item = String>) -> Vec<String> {
        let mut ids: Vec<String> = works.into_iter().collect();
        ids.sort();
        ids
    }

    /// The errors `doctor` finds in the tree.
    fn errors(repo: &Repo) -> Vec<String> {
        crate::doctor::diagnose(&tree(repo), None, false)
            .into_iter()
            .filter(|finding| serde_json::to_value(finding.severity).unwrap() == "error")
            .map(|finding| serde_json::to_string(&finding).unwrap())
            .collect()
    }

    #[test]
    fn an_unknown_a_parking_a_closed_or_an_inherited_release_is_refused() {
        let repo = sample();
        repo.write(
            "p/c/tasks/T1-c.md",
            &item("T1", "todo", "").replace("node: p", "node: p/c"),
        );
        let tree = tree(&repo);
        let p = NodePath::new("p");
        for (name, expected) in [
            ("9.9", "no release `9.9`"),
            ("later", "parking release"),
            ("0.2", "already closed on 2026-10-01"),
        ] {
            match close(&tree, &request(&p, name, false, true)).unwrap() {
                Err(Refusal::Invalid(message)) => assert!(message.contains(expected), "{message}"),
                other => panic!("{name} should be refused, got {other:?}"),
            }
        }
        let c = NodePath::new("p/c");
        assert!(matches!(
            close(&tree, &request(&c, "0.1", false, true)).unwrap(),
            Err(Refusal::Inherited { .. })
        ));
        assert_eq!(read(&repo, "p/node.toml"), NODE);
    }

    #[test]
    fn open_work_refuses_the_close_and_is_listed_and_a_dry_run_reports_it() {
        let repo = with_open_work();
        let tree = tree(&repo);
        let p = NodePath::new("p");
        let Err(Refusal::Open { release, items }) =
            close(&tree, &request(&p, "0.1", false, false)).unwrap()
        else {
            panic!("0.1 has open work");
        };
        assert_eq!(release, "0.1");
        assert_eq!(ids(items), ["S5", "T9"]);
        let refusal = Refusal::Open {
            release: "0.1".to_owned(),
            items: vec!["T9".to_owned()],
        };
        assert!(refusal.to_string().contains("carry it over"));
        assert_eq!(read(&repo, "p/node.toml"), NODE);
        assert!(exists(&repo, "p/tasks/T1-a.md"));
        let dry = close(&tree, &request(&p, "0.1", false, true))
            .unwrap()
            .unwrap();
        assert!(dry.blocked);
        assert_eq!(dry.carry_to.as_deref(), Some("0.3"));
        assert_eq!(ids(dry.open.into_iter().map(|work| work.id)), ["S5", "T9"]);
        assert_eq!(read(&repo, "p/node.toml"), NODE);
    }

    #[test]
    fn a_carry_over_moves_the_open_work_to_the_next_open_release_on_the_item_itself() {
        let repo = with_open_work();
        let p = NodePath::new("p");
        let closing = close(&tree(&repo), &request(&p, "0.1", true, false))
            .unwrap()
            .unwrap();
        assert_eq!(closing.carry_to.as_deref(), Some("0.3"));
        assert!(!closing.blocked);
        assert!(read(&repo, "p/tasks/T9-z.md").contains("release: '0.3'\n"));
        assert!(read(&repo, "p/stories/S5-s.md").contains("release: '0.3'\n"));
        assert!(exists(&repo, "p/archives/tasks/T1-a.md"));
        assert!(
            exists(&repo, "p/stories/S1-s.md"),
            "S1 still has T9 outside archives"
        );
        let kept: Vec<(String, String)> = closing
            .kept
            .iter()
            .map(|kept| (kept.work.id.clone(), kept.reason.clone()))
            .collect();
        assert!(kept.contains(&("S1".to_owned(), "work left outside archives: T9".to_owned())));
        assert!(errors(&repo).is_empty(), "{:?}", errors(&repo));
    }

    #[test]
    fn a_carry_over_needs_a_next_release() {
        let repo = Repo::new();
        repo.write(
            "p/node.toml",
            "[[release]]\nname = \"1.0\"\n\n[[release]]\nname = \"later\"\nparking = true\n",
        )
        .write("p/tasks/T1-a.md", &item("T1", "todo", "release: '1.0'\n"));
        let p = NodePath::new("p");
        match close(&tree(&repo), &request(&p, "1.0", true, true)).unwrap() {
            Err(Refusal::Invalid(message)) => {
                assert!(message.contains("no release after"), "{message}")
            }
            other => panic!("refused expected, got {other:?}"),
        }
    }

    #[test]
    fn the_finished_work_is_archived_with_its_links_rewritten_and_the_release_closed() {
        let repo = sample();
        let p = NodePath::new("p");
        let dry = close(&tree(&repo), &request(&p, "0.1", false, true))
            .unwrap()
            .unwrap();
        assert!(exists(&repo, "p/tasks/T1-a.md"));
        assert_eq!(read(&repo, "p/node.toml"), NODE);
        let closing = close(&tree(&repo), &request(&p, "0.1", false, false))
            .unwrap()
            .unwrap();
        assert_eq!(dry.archived, closing.archived);
        assert_eq!(
            ids(closing
                .archived
                .iter()
                .map(|archived| archived.work.id.clone())),
            ["E2", "S1", "S3", "T1", "T2", "T3", "T4", "T6", "T7"]
        );
        for path in [
            "p/archives/tasks/T1-a.md",
            "p/archives/tasks/T3-c.md",
            "p/archives/tasks/T4-d.md",
            "p/archives/stories/S1-s.md",
            "p/archives/stories/S3-s.md",
            "p/archives/epics/E2-e.md",
        ] {
            assert!(exists(&repo, path), "{path}");
        }
        assert!(read(&repo, "p/archives/stories/S1-s.md").contains("See [T1](../tasks/T1-a.md)."));
        let t1_text = read(&repo, "p/archives/tasks/T1-a.md");
        assert!(
            t1_text.contains("Next: [S2](../../stories/S2-s.md#scope), `T5-e.md`."),
            "{t1_text}"
        );
        assert_eq!(
            read(&repo, "p/README.md"),
            "Links: [T1](archives/tasks/T1-a.md), [T6](./archives/tasks/T6-f.md).\n"
        );
        let t1 = closing
            .archived
            .iter()
            .find(|archived| archived.work.id == "T1")
            .unwrap();
        assert_eq!((t1.references, t1.files), (3, 3));
        assert_eq!(t1.from, "tree/p/tasks/T1-a.md");
        assert_eq!(t1.to, "tree/p/archives/tasks/T1-a.md");
        assert!(exists(&repo, "p/epics/E1-e.md"), "E1 spans 0.1 and 0.3");
        assert!(exists(&repo, "p/stories/S2-s.md"));
        assert!(exists(&repo, "p/stories/S4-s.md"));
        let reasons: Vec<(&str, &str)> = closing
            .kept
            .iter()
            .map(|kept| (kept.work.id.as_str(), kept.reason.as_str()))
            .collect();
        assert_eq!(
            reasons,
            [
                ("S2", "work left outside archives: T5"),
                (
                    "S4",
                    "all its work is archived, but its status `todo` is not closed"
                ),
                ("E1", "work left outside archives: S2"),
            ]
        );
        assert_eq!(
            read(&repo, "p/node.toml"),
            NODE.replace("name = \"0.1\"\n", "name = \"0.1\"\nclosed = 2026-11-14\n")
        );
        assert!(
            closing
                .changelog
                .markdown
                .starts_with("## 0.1\n\n### Added\n")
        );
        assert!(errors(&repo).is_empty(), "{:?}", errors(&repo));
        assert!(matches!(
            close(&tree(&repo), &request(&p, "0.1", false, true)).unwrap(),
            Err(Refusal::Invalid(_))
        ));
        let changelog = changelog::entry(&tree(&repo), &p, "0.1");
        assert_eq!(changelog.markdown, closing.changelog.markdown);
    }

    #[test]
    fn a_closed_release_takes_no_more_work_and_doctor_warns_on_open_work_left_in_it() {
        let repo = sample();
        repo.write("p/tasks/T8-h.md", &item("T8", "todo", "release: '0.2'\n"))
            .write("p/stories/S6-s.md", &item("S6", "todo", "release: '0.2'\n"))
            .write("p/tasks/T10-i.md", &item("T10", "done", "parent: S6\n"));
        let tree = tree(&repo);
        let warned: Vec<(String, String)> = crate::doctor::diagnose(&tree, None, false)
            .into_iter()
            .filter(|finding| {
                serde_json::to_value(finding.check).unwrap() == "open_in_closed_release"
            })
            .map(|finding| (finding.path.unwrap_or_default(), finding.message))
            .collect();
        assert_eq!(
            warned,
            [(
                "tree/p/tasks/T8-h.md".to_owned(),
                "open work in the release `0.2`, closed on 2026-10-01".to_owned()
            )]
        );
        let task = tree
            .items
            .iter()
            .find(|item| item.id() == Some("T5"))
            .unwrap();
        match declare::assign(&tree, task, Some("0.2")).unwrap() {
            Err(Refusal::Invalid(message)) => assert!(message.contains("closed on 2026-10-01")),
            other => panic!("a closed release takes no more work, got {other:?}"),
        }
        let marks = declare::Marks {
            parking: Some(true),
            target: None,
        };
        assert!(matches!(
            declare::set(&tree, &NodePath::new("p"), "0.2", marks).unwrap(),
            Err(Refusal::Invalid(_))
        ));
        assert_eq!(read(&repo, "p/node.toml"), NODE);
    }
}

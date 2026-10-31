//! The manual order of items (SPEC.md §1.3, ADR-0022).
//!
//! An item may carry `rank`, a sparse integer written when it is moved. Items
//! are ordered per node and per kind: ranks of two nodes or two kinds are never
//! compared. Within one node and kind, ranked items come first by ascending
//! `rank`, then the unranked ones by descending `priority`; ties fall back to
//! the natural order of ids. An archived item keeps its `rank` in its file, but
//! the order ignores it.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use yaml_serde::Value;

use crate::cli::RankArgs;
use crate::item::{self, Document, Header, Kind, render};
use crate::schema::LEVELS;
use crate::show;
use crate::tasks::Row;
use crate::tree::{Item, NodePath, Tree};

/// What the order of one item depends on.
#[derive(Debug, Clone, Copy, Default)]
pub struct Placement<'a> {
    /// The `rank` of the header, `None` when absent or when the item is archived.
    pub rank: Option<i64>,
    /// The `priority` of the header.
    pub priority: Option<&'a str>,
    /// The id of the header.
    pub id: Option<&'a str>,
}

impl<'a> Placement<'a> {
    /// The placement of a listed item.
    pub fn of(row: &Row<'a>) -> Placement<'a> {
        Placement {
            rank: row.rank.filter(|_| !row.archived),
            priority: row.priority,
            id: row.id,
        }
    }
}

/// Compares two items of the same node and kind: ranked before unranked,
/// ranked by ascending `rank`, unranked by descending `priority`, then ids in
/// natural order.
pub fn compare(a: &Placement, b: &Placement) -> Ordering {
    let by_rank = match (a.rank, b.rank) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => level(b.priority).cmp(&level(a.priority)),
    };
    by_rank.then_with(|| compare_ids(a.id, b.id))
}

/// Sorts listed items: nodes, and kinds within a node, stay in the order they
/// first appear in; the items of each node and kind follow [`compare`].
pub fn sort(rows: &mut [Row]) {
    let mut nodes: HashMap<NodePath, usize> = HashMap::new();
    let mut kinds: HashMap<(NodePath, Kind), usize> = HashMap::new();
    for row in rows.iter() {
        let next = nodes.len();
        nodes.entry(row.node.clone()).or_insert(next);
        let next = kinds.len();
        kinds.entry((row.node.clone(), row.kind)).or_insert(next);
    }
    let group = |row: &Row| (nodes[row.node], kinds[&(row.node.clone(), row.kind)]);
    rows.sort_by(|a, b| {
        group(a)
            .cmp(&group(b))
            .then_with(|| compare(&Placement::of(a), &Placement::of(b)))
    });
}

/// Weight of a `priority`: `high` above `medium` above `low` above none.
fn level(priority: Option<&str>) -> Option<usize> {
    priority.and_then(|value| LEVELS.iter().position(|candidate| *candidate == value))
}

/// Compares ids in natural order, `T9` before `T10`; an item without id comes last.
fn compare_ids(a: Option<&str>, b: Option<&str>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => natural_key(a).cmp(&natural_key(b)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Splits an id into its letters, the number that follows them and the id itself.
fn natural_key(id: &str) -> (&str, Option<u64>, &str) {
    let digits = id.trim_start_matches(|c: char| !c.is_ascii_digit());
    let letters = &id[..id.len() - digits.len()];
    (letters, digits.parse().ok(), id)
}

/// Spacing between the ranks a move gives.
pub const GAP: i64 = 1000;
/// Smallest spacing a re-spaced window leaves between two neighbours.
const MIN_STEP: i64 = 16;

/// Where `ariane rank` places an item.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// Right before this item.
    Before(&'a Item),
    /// Right after this item.
    After(&'a Item),
    /// First of all.
    Top,
    /// After the last ranked item.
    Bottom,
    /// Back among the unranked items: the key is removed.
    Clear,
}

/// One rank a move rewrites: `None` removes the key.
#[derive(Debug)]
pub struct Change<'a> {
    /// The item whose header changes.
    pub item: &'a Item,
    /// Its rank before the move.
    pub from: Option<i64>,
    /// Its rank after the move.
    pub to: Option<i64>,
}

impl<'a> Placement<'a> {
    /// The placement of an item of the tree.
    pub fn of_item(item: &'a Item) -> Placement<'a> {
        Placement {
            rank: rank_of(item),
            priority: item.document.text("priority"),
            id: item.id(),
        }
    }
}

/// The rank the order reads for `item`: its `rank`, unless it is archived.
fn rank_of(item: &Item) -> Option<i64> {
    item.document.integer("rank").filter(|_| !item.archived)
}

/// The ranks to write so that `moved` sits at `target` in the order of its
/// node and kind.
///
/// Only `moved` changes in the usual case: it takes the middle of the gap
/// between its new neighbours, the first rank minus [`GAP`] at the top, the
/// last plus [`GAP`] at the bottom. A drop below an unranked item ranks every
/// unranked item of the node and kind at once, in their current order, the
/// moved item at its place among them. When the gap is exhausted, a window
/// of neighbours around the drop point is re-spaced, widened until it holds
/// enough room. An item already between its new neighbours keeps its rank.
///
/// # Errors
///
/// Fails when `moved` or the target is archived, a ressource or without id,
/// when the target belongs to another node or kind, or is `moved` itself.
pub fn plan<'a>(tree: &'a Tree, moved: &'a Item, target: Target<'a>) -> Result<Vec<Change<'a>>> {
    movable(moved)?;
    let current = rank_of(moved);
    if let Target::Clear = target {
        return Ok(current
            .map(|from| Change {
                item: moved,
                from: Some(from),
                to: None,
            })
            .into_iter()
            .collect());
    }
    let mut sequence: Vec<&Item> = tree
        .items
        .iter()
        .filter(|item| {
            item.node == moved.node
                && item.kind == moved.kind
                && !item.archived
                && item.id().is_some()
                && !std::ptr::eq(*item, moved)
        })
        .collect();
    sequence.sort_by(|a, b| compare(&Placement::of_item(a), &Placement::of_item(b)));
    let position = match target {
        Target::Top => 0,
        Target::Bottom => sequence
            .iter()
            .filter(|item| rank_of(item).is_some())
            .count(),
        Target::Before(other) | Target::After(other) => {
            let index = index_of(&sequence, moved, other)?;
            if matches!(target, Target::After(_)) {
                index + 1
            } else {
                index
            }
        }
        Target::Clear => unreachable!("handled above"),
    };
    sequence.insert(position, moved);
    let mut ranks: Vec<Option<i64>> = sequence.iter().map(|item| rank_of(item)).collect();
    ranks[position] = None;
    if ranks[..position].iter().any(Option::is_none) {
        let mut last = None;
        for slot in &mut ranks {
            let given = slot.unwrap_or_else(|| last.map_or(GAP, |previous| previous + GAP));
            *slot = Some(given);
            last = Some(given);
        }
        return Ok(changes(&sequence, ranks, moved, current));
    }
    let end = (position + 1..ranks.len())
        .find(|&index| ranks[index].is_none())
        .unwrap_or(ranks.len());
    let previous = position.checked_sub(1).and_then(|index| ranks[index]);
    let next = (position + 1 < end).then(|| ranks[position + 1]).flatten();
    let fits = |rank: i64| previous.is_none_or(|p| p < rank) && next.is_none_or(|n| rank < n);
    ranks[position] = match (current, previous, next) {
        (Some(rank), _, _) if fits(rank) => Some(rank),
        (_, None, None) => Some(GAP),
        (_, Some(p), None) => Some(p + GAP),
        (_, None, Some(n)) => Some(n - GAP),
        (_, Some(p), Some(n)) if n - p > 1 => Some(p + (n - p) / 2),
        _ => {
            respace(&mut ranks, position, end);
            ranks[position]
        }
    };
    Ok(changes(&sequence, ranks, moved, current))
}

/// The ranks of `sequence` that differ from `ranks`, `moved` compared with its rank before the move.
fn changes<'a>(
    sequence: &[&'a Item],
    ranks: Vec<Option<i64>>,
    moved: &Item,
    current: Option<i64>,
) -> Vec<Change<'a>> {
    sequence
        .iter()
        .zip(ranks)
        .filter_map(|(item, to)| {
            let from = if std::ptr::eq(*item, moved) {
                current
            } else {
                rank_of(item)
            };
            (from != to).then_some(Change { item, from, to })
        })
        .collect()
}

/// Fails unless `item` can take a rank: a live item with an id, not a ressource.
fn movable(item: &Item) -> Result<()> {
    let name = item
        .reference()
        .unwrap_or_else(|| item.relative.display().to_string());
    if item.id().is_none() {
        bail!("{name} has no versioned header; run `ariane migrate` first");
    }
    if item.archived {
        bail!("{name} is archived: the order ignores it");
    }
    if item.kind == Kind::Ressource {
        bail!("{name} is a ressource: ressources are not ordered");
    }
    Ok(())
}

/// The index of `other` in the order of `moved`'s node and kind.
fn index_of(sequence: &[&Item], moved: &Item, other: &Item) -> Result<usize> {
    let name = other.reference().unwrap_or_default();
    if std::ptr::eq(other, moved) {
        bail!("an item cannot be placed relative to itself");
    }
    movable(other)?;
    if other.node != moved.node || other.kind != moved.kind {
        bail!(
            "{name} is not in the order of {}: ranks are per node and per kind",
            moved.reference().unwrap_or_default()
        );
    }
    sequence
        .iter()
        .position(|item| std::ptr::eq(*item, other))
        .ok_or_else(|| anyhow!("{name} is not in the order of its node"))
}

/// Re-spaces the ranks of a window around `position`, within the ranked
/// prefix `..end`, widening it two items on each side until the ranks that
/// bound it leave at least [`MIN_STEP`] between its items.
fn respace(ranks: &mut [Option<i64>], position: usize, end: usize) {
    let (mut low, mut high) = (position, position);
    loop {
        low = low.saturating_sub(2);
        high = (high + 2).min(end - 1);
        let below = low.checked_sub(1).and_then(|index| ranks[index]);
        let above = (high + 1 < end).then(|| ranks[high + 1]).flatten();
        let count = i64::try_from(high - low + 1).unwrap_or(i64::MAX);
        let start_and_step = match (below, above) {
            (Some(b), Some(a)) if (a - b) / (count + 1) >= MIN_STEP => {
                Some((b, (a - b) / (count + 1)))
            }
            (Some(b), None) => Some((b, GAP)),
            (None, Some(a)) => Some((a - GAP * (count + 1), GAP)),
            (None, None) => Some((0, GAP)),
            _ => None,
        };
        if let Some((start, step)) = start_and_step {
            for (offset, slot) in ranks[low..=high].iter_mut().enumerate() {
                let offset = i64::try_from(offset).unwrap_or(i64::MAX);
                *slot = Some(start + step * (offset + 1));
            }
            return;
        }
    }
}

/// Writes each change into its item's header, the key at its canonical place.
///
/// # Errors
///
/// Fails when a file cannot be read, has no readable header, or cannot be written.
pub fn write(changes: &[Change]) -> Result<()> {
    for change in changes {
        let file = &change.item.file;
        let text =
            std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        let document = Document::parse(&text);
        let Header::Present(mut header) = document.header else {
            bail!("{} has no readable front-matter", file.display());
        };
        item::set_key(&mut header, "rank", change.to.map(Value::from));
        std::fs::write(file, render(&header, &document.body)?)
            .with_context(|| format!("writing {}", file.display()))?;
    }
    Ok(())
}

/// Runs `ariane rank`: places the item, writes the ranks that change and
/// prints them, one line per item.
///
/// # Errors
///
/// Fails when a reference names no single item, the move is refused, or a
/// file cannot be rewritten.
pub fn run(tree: &Tree, args: &RankArgs, here: Option<&NodePath>) -> Result<ExitCode> {
    let moved = show::find(tree, &args.reference, here)?;
    let relative = |raw: &str| show::find(tree, raw, Some(&moved.node));
    let target = if let Some(raw) = &args.before {
        Target::Before(relative(raw)?)
    } else if let Some(raw) = &args.after {
        Target::After(relative(raw)?)
    } else if args.top {
        Target::Top
    } else if args.bottom {
        Target::Bottom
    } else {
        Target::Clear
    };
    let changes = plan(tree, moved, target)?;
    write(&changes)?;
    if changes.is_empty() {
        println!(
            "{} is already in place",
            moved.reference().unwrap_or_default()
        );
    }
    let shown = |rank: Option<i64>| rank.map_or_else(|| "-".to_owned(), |rank| rank.to_string());
    for change in &changes {
        println!(
            "{}  {} -> {}",
            change.item.reference().unwrap_or_default(),
            shown(change.from),
            shown(change.to)
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// Unit tests of the order of items.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::{Selection, rows};
    use crate::tree::Tree;
    use crate::tree::fixture::Repo;

    fn placement<'a>(rank: Option<i64>, priority: Option<&'a str>, id: &'a str) -> Placement<'a> {
        Placement {
            rank,
            priority,
            id: Some(id),
        }
    }

    #[test]
    fn ranked_items_come_first_by_ascending_rank() {
        let first = placement(Some(-1000), None, "T9");
        let second = placement(Some(1500), Some("low"), "T1");
        let unranked = placement(None, Some("high"), "T2");
        assert_eq!(compare(&first, &second), Ordering::Less);
        assert_eq!(compare(&second, &unranked), Ordering::Less);
    }

    #[test]
    fn unranked_items_follow_priority_then_ids_in_natural_order() {
        let high = placement(None, Some("high"), "T10");
        let low = placement(None, Some("low"), "T2");
        let none = placement(None, None, "T1");
        assert_eq!(compare(&high, &low), Ordering::Less);
        assert_eq!(compare(&low, &none), Ordering::Less);
        assert_eq!(
            compare(&placement(None, None, "T9"), &placement(None, None, "T10")),
            Ordering::Less
        );
    }

    #[test]
    fn equal_ranks_fall_back_to_ids() {
        let a = placement(Some(2000), None, "T3");
        let b = placement(Some(2000), None, "T12");
        assert_eq!(compare(&a, &b), Ordering::Less);
    }

    #[test]
    fn listed_items_are_ordered_per_node_and_kind_and_archives_ignore_their_rank() {
        let repo = Repo::new();
        let task = |node: &str, id: &str, extra: &str| {
            format!(
                "---\nariane: 1\nnode: {node}\nid: {id}\ntitle: {id}\nstatus: todo\n{extra}---\n"
            )
        };
        repo.write("p/tasks/T1-a.md", &task("p", "T1", "priority: high\n"))
            .write("p/tasks/T2-b.md", &task("p", "T2", "rank: 2000\n"))
            .write("p/tasks/T3-c.md", &task("p", "T3", "rank: 1000\n"))
            .write("p/a/tasks/T1-d.md", &task("p/a", "T1", "rank: -5\n"))
            .write("p/archives/tasks/T4-e.md", &task("p", "T4", "rank: 1\n"));
        let tree = Tree::load(repo.root()).unwrap();
        let listed = rows(
            &tree,
            None,
            &Selection {
                archived: true,
                ..Selection::default()
            },
        );
        let order: Vec<String> = listed
            .iter()
            .map(|row| row.reference.clone().unwrap())
            .collect();
        let live: Vec<&str> = order
            .iter()
            .map(String::as_str)
            .filter(|reference| reference.starts_with("p#"))
            .collect();
        assert_eq!(live, ["p#T3", "p#T2", "p#T1", "p#T4"]);
        assert!(order.contains(&"p/a#T1".to_owned()));
    }

    fn node(tasks: &[(&str, Option<i64>)]) -> Repo {
        let repo = Repo::new();
        for (id, rank) in tasks {
            let rank = rank
                .map(|rank| format!("rank: {rank}\n"))
                .unwrap_or_default();
            repo.write(
                &format!("p/tasks/{id}-x.md"),
                &format!("---\nariane: 1\nnode: p\nid: {id}\ntitle: {id}\nstatus: todo\n{rank}---\n# {id}\n"),
            );
        }
        repo
    }

    fn find<'a>(tree: &'a Tree, id: &str) -> &'a Item {
        show::find(tree, &format!("p#{id}"), None).unwrap()
    }

    fn moves(
        repo: &Repo,
        id: &str,
        target: impl Fn(&Tree) -> Target,
    ) -> Vec<(String, Option<i64>)> {
        let tree = Tree::load(repo.root()).unwrap();
        let changes = plan(&tree, find(&tree, id), target(&tree)).unwrap();
        changes
            .iter()
            .map(|change| (change.item.id().unwrap().to_owned(), change.to))
            .collect()
    }

    fn order(repo: &Repo) -> Vec<String> {
        let tree = Tree::load(repo.root()).unwrap();
        rows(&tree, None, &Selection::default())
            .iter()
            .map(|row| row.id.unwrap().to_owned())
            .collect()
    }

    #[test]
    fn a_move_between_two_ranked_items_takes_the_middle_and_writes_one_file() {
        let repo = node(&[("T1", Some(1000)), ("T2", Some(2000)), ("T3", Some(3000))]);
        assert_eq!(
            moves(&repo, "T3", |tree| Target::Before(find(tree, "T2"))),
            [("T3".to_owned(), Some(1500))]
        );
    }

    #[test]
    fn the_second_above_the_first_writes_only_the_moved_item() {
        let repo = node(&[("T1", Some(1000)), ("T2", Some(2000))]);
        assert_eq!(
            moves(&repo, "T2", |_| Target::Top),
            [("T2".to_owned(), Some(0))]
        );
        assert_eq!(
            moves(&repo, "T1", |_| Target::Bottom),
            [("T1".to_owned(), Some(3000))]
        );
    }

    #[test]
    fn an_item_already_in_place_keeps_its_rank() {
        let repo = node(&[("T1", Some(1000)), ("T2", Some(2000))]);
        assert!(moves(&repo, "T1", |_| Target::Top).is_empty());
        assert!(moves(&repo, "T2", |tree| Target::After(find(tree, "T1"))).is_empty());
    }

    #[test]
    fn an_exhausted_gap_respaces_only_a_window_of_neighbours() {
        let repo = node(&[
            ("T1", Some(1000)),
            ("T2", Some(1001)),
            ("T3", Some(1002)),
            ("T4", Some(1003)),
            ("T5", Some(9000)),
            ("T6", Some(20000)),
            ("T7", None),
        ]);
        let changes = moves(&repo, "T7", |tree| Target::After(find(tree, "T2")));
        assert!(changes.len() < 7, "{changes:?}");
        assert!(!changes.iter().any(|(id, _)| id == "T6"));
        let tree = Tree::load(repo.root()).unwrap();
        write(&plan(&tree, find(&tree, "T7"), Target::After(find(&tree, "T2"))).unwrap()).unwrap();
        assert_eq!(order(&repo), ["T1", "T2", "T7", "T3", "T4", "T5", "T6"]);
    }

    #[test]
    fn a_drop_below_an_unranked_item_ranks_the_whole_node_and_kind() {
        let repo = node(&[
            ("T1", None),
            ("T2", None),
            ("T3", None),
            ("T4", None),
            ("T5", None),
        ]);
        assert_eq!(
            moves(&repo, "T4", |tree| Target::Before(find(tree, "T3"))),
            [
                ("T1".to_owned(), Some(1000)),
                ("T2".to_owned(), Some(2000)),
                ("T4".to_owned(), Some(3000)),
                ("T3".to_owned(), Some(4000)),
                ("T5".to_owned(), Some(5000)),
            ]
        );
    }

    #[test]
    fn a_drop_above_every_unranked_item_writes_only_the_moved_item() {
        let repo = node(&[("T1", Some(1000)), ("T2", None), ("T3", None)]);
        assert_eq!(
            moves(&repo, "T3", |_| Target::Top),
            [("T3".to_owned(), Some(0))]
        );
        assert_eq!(
            moves(&repo, "T3", |tree| Target::Before(find(tree, "T2"))),
            [("T3".to_owned(), Some(2000))]
        );
    }

    #[test]
    fn equal_ranks_are_separated_when_an_item_is_dropped_between_them() {
        let repo = node(&[("T1", Some(1000)), ("T2", Some(1000)), ("T3", Some(5000))]);
        let tree = Tree::load(repo.root()).unwrap();
        write(&plan(&tree, find(&tree, "T3"), Target::After(find(&tree, "T1"))).unwrap()).unwrap();
        assert_eq!(order(&repo), ["T1", "T3", "T2"]);
    }

    #[test]
    fn writing_puts_the_rank_after_priority_and_clearing_removes_it() {
        let repo = Repo::new();
        repo.write(
            "p/tasks/T1-x.md",
            "---\nariane: 1\nnode: p\nid: T1\ntitle: X\nstatus: todo\npriority: high\ncreated: 2026-10-31\nowner: me\n---\n# X\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        write(&plan(&tree, find(&tree, "T1"), Target::Top).unwrap()).unwrap();
        let text = std::fs::read_to_string(&find(&tree, "T1").file).unwrap();
        assert!(
            text.contains("priority: high\nrank: 1000\ncreated: 2026-10-31\nowner: me\n"),
            "{text}"
        );
        let tree = Tree::load(repo.root()).unwrap();
        write(&plan(&tree, find(&tree, "T1"), Target::Clear).unwrap()).unwrap();
        let text = std::fs::read_to_string(&find(&tree, "T1").file).unwrap();
        assert!(!text.contains("rank"), "{text}");
    }

    #[test]
    fn a_move_across_nodes_or_kinds_or_onto_an_archive_is_refused() {
        let repo = node(&[("T1", Some(1000))]);
        repo.write(
            "q/tasks/T2-x.md",
            "---\nariane: 1\nnode: q\nid: T2\ntitle: T2\nstatus: todo\n---\n",
        )
        .write(
            "p/stories/S1-x.md",
            "---\nariane: 1\nnode: p\nid: S1\ntitle: S1\nstatus: todo\n---\n",
        )
        .write(
            "p/archives/tasks/T3-x.md",
            "---\nariane: 1\nnode: p\nid: T3\ntitle: T3\nstatus: done\n---\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        let moved = find(&tree, "T1");
        for other in ["q#T2", "p#S1", "p#T3", "p#T1"] {
            let other = show::find(&tree, other, None).unwrap();
            assert!(plan(&tree, moved, Target::Before(other)).is_err());
        }
        assert!(plan(&tree, find(&tree, "T3"), Target::Top).is_err());
    }

    #[test]
    fn hundreds_of_moves_keep_the_wanted_order_and_rewrite_few_files() {
        let ids: Vec<String> = (1..=40).map(|n| format!("T{n}")).collect();
        let tasks: Vec<(&str, Option<i64>)> = ids.iter().map(|id| (id.as_str(), None)).collect();
        let repo = node(&tasks);
        let mut wanted = order(&repo);
        let mut seed: u64 = 42;
        let mut next = |bound: usize| {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            usize::try_from(seed >> 33).unwrap() % bound
        };
        let mut written = 0;
        for _ in 0..300 {
            let moved = wanted[next(wanted.len())].clone();
            let near_top = next(4) > 0;
            let others: Vec<String> = wanted.iter().filter(|id| **id != moved).cloned().collect();
            let anchor = others[if near_top {
                next(5)
            } else {
                next(others.len())
            }]
            .clone();
            let tree = Tree::load(repo.root()).unwrap();
            let changes = plan(
                &tree,
                find(&tree, &moved),
                Target::Before(find(&tree, &anchor)),
            )
            .unwrap();
            assert!(changes.len() <= 40, "{changes:?}");
            written += changes.len();
            write(&changes).unwrap();
            wanted.retain(|id| *id != moved);
            let at = wanted.iter().position(|id| *id == anchor).unwrap();
            wanted.insert(at, moved);
            assert_eq!(order(&repo), wanted);
        }
        assert!(written < 300 * 2, "{written} files written for 300 moves");
    }
}

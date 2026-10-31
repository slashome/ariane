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

use crate::item::Kind;
use crate::schema::LEVELS;
use crate::tasks::Row;
use crate::tree::NodePath;

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
}

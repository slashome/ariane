//! Builds the [`Input`] of a forecast from a loaded tree.
//!
//! The forecast of a node runs on the calendar's carrier of that node, over
//! every item below the carrier (ADR-0026 §4); without a carrier, on the node
//! itself and its descendants. Hours come from [`crate::estimate`], effective
//! releases from [`crate::release`], the manual order from [`crate::rank`]:
//! nothing is recomputed here.

use std::cmp::Ordering;
use std::collections::HashMap;

use jiff::Timestamp;
use jiff::civil::Date;

use crate::calendar::Calendar;
use crate::closed;
use crate::estimate::{self, Basis};
use crate::history::History;
use crate::item::Kind;
use crate::node::{Category, Effective, Stage};
use crate::rank::{self, Placement};
use crate::relations::Index;
use crate::release;
use crate::tree::{Item, NodePath, Tree};

use super::input::{Capacity, GroupSpec, Input, Planned, ReleaseSpec, State, Unit};

/// The effective configuration of each node, read once.
struct Configs<'a> {
    /// The tree whose files are combined.
    tree: &'a Tree,
    /// The configurations read so far.
    cache: HashMap<NodePath, Effective>,
}

impl Configs<'_> {
    /// The effective configuration of `node`.
    fn of(&mut self, node: &NodePath) -> &Effective {
        self.cache
            .entry(node.clone())
            .or_insert_with(|| self.tree.configs.effective(node))
    }
}

/// The input of the forecast of `node`: its carrier's sequence when
/// `calendar` gives it one, else its own, starting at `now`. `history` gives
/// the approximate closing date of finished work without `closed`.
pub fn gather(
    tree: &Tree,
    calendar: Option<&Calendar>,
    history: Option<&History>,
    node: &NodePath,
    now: Timestamp,
) -> Input {
    let carried = calendar.and_then(|calendar| {
        calendar
            .carrier(node)
            .map(|(path, entry)| (calendar, path.clone(), entry))
    });
    let scope = carried
        .as_ref()
        .map_or_else(|| node.clone(), |(_, path, _)| path.clone());
    let capacity = carried.as_ref().map(|(calendar, _, entry)| Capacity {
        timezone: calendar.timezone.clone(),
        focus: entry.focus,
        blocks: entry.blocks.clone(),
        days_off: calendar
            .days_off
            .iter()
            .chain(&entry.days_off)
            .copied()
            .collect(),
    });
    let mut paths: Vec<&NodePath> = tree.nodes.iter().map(|node| &node.path).collect();
    paths.sort();
    let nodes: HashMap<&NodePath, usize> = paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| (path, index))
        .collect();
    let node_rank = |path: &NodePath| nodes.get(path).copied().unwrap_or(usize::MAX);
    let mut configs = Configs {
        tree,
        cache: HashMap::new(),
    };
    let (releases, release_index) = releases(tree, &scope, &mut configs, &node_rank);
    let index = Index::new(tree);
    let work: Vec<&Item> = tree
        .items_within(Some(&scope))
        .filter(|item| matches!(item.kind, Kind::Task | Kind::Story | Kind::Epic))
        .filter(|item| item.id().is_some())
        .collect();
    let units_items: Vec<(&Item, estimate::Estimate)> = work
        .iter()
        .filter_map(|item| Some((*item, estimate::effective(tree, item)?)))
        .filter(|(item, estimate)| item.kind == Kind::Task || estimate.basis != Basis::Tasks)
        .collect();
    let mut group_items: Vec<&Item> = work
        .iter()
        .copied()
        .filter(|item| item.kind != Kind::Task)
        .collect();
    for (item, _) in &units_items {
        let (story, epic) = containers(&index, item);
        group_items.extend(story);
        group_items.extend(epic);
    }
    group_items.sort_by(|a, b| by_place(a, b, &node_rank));
    group_items.dedup_by(|a, b| std::ptr::eq(*a, *b));
    let groups: Vec<GroupSpec> = group_items
        .iter()
        .enumerate()
        .filter_map(|(order, item)| {
            let (_, epic) = containers(&index, item);
            Some(GroupSpec {
                reference: item.reference()?,
                kind: item.kind,
                title: title(item),
                order,
                target: date(item, "target"),
                epic: epic
                    .filter(|epic| !std::ptr::eq(*epic, *item))
                    .and_then(Item::reference),
            })
        })
        .collect();
    let mut ordered: Vec<&(&Item, estimate::Estimate)> = units_items.iter().collect();
    ordered.sort_by(|a, b| by_place(a.0, b.0, &node_rank));
    let units = ordered
        .into_iter()
        .enumerate()
        .filter_map(|(order, (item, estimate))| {
            unit(
                tree,
                &index,
                &mut configs,
                &release_index,
                history,
                item,
                estimate,
                order,
            )
        })
        .collect();
    Input {
        scope,
        carrier: carried.map(|(_, path, _)| path),
        releases,
        groups,
        units,
        capacity,
        now,
    }
}

/// Compares two items by the order of their node paths, a parent before its
/// children, then by the manual order of ADR-0022.
fn by_place(a: &Item, b: &Item, node_rank: &impl Fn(&NodePath) -> usize) -> Ordering {
    node_rank(&a.node)
        .cmp(&node_rank(&b.node))
        .then_with(|| (a.kind as u8).cmp(&(b.kind as u8)))
        .then_with(|| rank::compare(&Placement::of_item(a), &Placement::of_item(b)))
}

/// The releases of every list that applies below `scope`, parking ones left
/// out, in delivery order: by place in their list, lists of different nodes
/// interleaved in the order of their node paths. Also gives the index of each `(list, name)`.
fn releases(
    tree: &Tree,
    scope: &NodePath,
    configs: &mut Configs,
    node_rank: &impl Fn(&NodePath) -> usize,
) -> (Vec<ReleaseSpec>, HashMap<(NodePath, String), usize>) {
    let mut entries: Vec<(usize, usize, ReleaseSpec)> = Vec::new();
    let mut seen: Vec<NodePath> = Vec::new();
    for node in tree.nodes.iter().filter(|node| node.path.is_within(scope)) {
        let effective = configs.of(&node.path);
        let (Some(list), Some(owner)) = (&effective.releases, &effective.releases_from) else {
            continue;
        };
        if seen.contains(owner) {
            continue;
        }
        seen.push(owner.clone());
        for (position, release) in list.iter().enumerate() {
            if release.parking {
                continue;
            }
            entries.push((
                position,
                node_rank(owner),
                ReleaseSpec {
                    name: release.name.clone(),
                    declared_by: owner.clone(),
                    target: release.target.as_deref().and_then(|text| text.parse().ok()),
                },
            ));
        }
    }
    entries.sort_by_key(|(position, owner, _)| (*position, *owner));
    let index = entries
        .iter()
        .enumerate()
        .map(|(index, (_, _, spec))| ((spec.declared_by.clone(), spec.name.clone()), index))
        .collect();
    (
        entries.into_iter().map(|(_, _, spec)| spec).collect(),
        index,
    )
}

/// The story and the epic `item` belongs to: for a task, its story and that
/// story's epic; for a story, itself and its epic; for an epic, itself.
fn containers<'a>(index: &Index<'a>, item: &'a Item) -> (Option<&'a Item>, Option<&'a Item>) {
    let parent = |child: &Item, kind: Kind| {
        child
            .document
            .text("parent")
            .and_then(|raw| index.resolve(raw, &child.node))
            .filter(|parent| parent.kind == kind)
    };
    match item.kind {
        Kind::Task => {
            let story = parent(item, Kind::Story);
            (story, story.and_then(|story| parent(story, Kind::Epic)))
        }
        Kind::Story => (Some(item), parent(item, Kind::Epic)),
        Kind::Epic => (None, Some(item)),
        _ => (None, None),
    }
}

/// One piece of work, `order` being its place among all of them.
#[allow(
    clippy::too_many_arguments,
    reason = "each argument is a distinct input the unit is built from"
)]
fn unit(
    tree: &Tree,
    index: &Index,
    configs: &mut Configs,
    release_index: &HashMap<(NodePath, String), usize>,
    history: Option<&History>,
    item: &Item,
    estimate: &estimate::Estimate,
    order: usize,
) -> Option<Unit> {
    let reference = item.reference()?;
    let (story, epic) = containers(index, item);
    let effective = configs.of(&item.node).clone();
    let status = item.document.text("status");
    let category = status.and_then(|status| tree.configs.category(&item.node, status));
    let stage = status.and_then(|status| tree.configs.stage(&item.node, status));
    let state = match category {
        Some(Category::Done) => {
            let written = date(item, closed::KEY);
            let approximated = written
                .is_none()
                .then(|| approximate(history?, item, &effective))
                .flatten();
            State::Done {
                closed: written.or(approximated),
                approximate: written.is_none() && approximated.is_some(),
            }
        }
        Some(Category::Dropped) => State::Dropped,
        Some(Category::Waiting) => State::Waiting,
        Some(Category::InProgress) if stage == Some(Stage::Testing) => State::Testing,
        Some(Category::InProgress) => State::InProgress,
        _ => State::Open,
    };
    let release = match release::effective(tree, item, &effective) {
        None => Planned::Unplanned,
        Some(found) if found.parking => Planned::Parking(found.name.to_owned()),
        Some(found) => effective
            .releases_from
            .as_ref()
            .filter(|_| found.position.is_some())
            .and_then(|owner| release_index.get(&(owner.clone(), found.name.to_owned())))
            .map_or_else(
                || Planned::Undeclared(found.name.to_owned()),
                |position| Planned::Release(*position),
            ),
    };
    let finished = matches!(state, State::Done { .. });
    let own: Vec<&Item> = [Some(item), story, epic].into_iter().flatten().collect();
    let mut depends_on: Vec<String> = Vec::new();
    for holder in &own {
        for raw in holder.document.texts("depends_on") {
            let Some(target) = index.resolve(raw, &holder.node) else {
                continue;
            };
            let closed = target
                .document
                .text("status")
                .and_then(|status| tree.configs.category(&target.node, status))
                .is_some_and(Category::is_closed);
            if closed || own.iter().any(|mine| std::ptr::eq(*mine, target)) {
                continue;
            }
            if let Some(reference) = target.reference()
                && !depends_on.contains(&reference)
            {
                depends_on.push(reference);
            }
        }
    }
    Some(Unit {
        reference,
        node: item.node.clone(),
        kind: item.kind,
        title: title(item),
        state,
        release,
        story: story.and_then(Item::reference),
        epic: epic.and_then(Item::reference),
        order,
        hours: if finished {
            estimate.hours
        } else {
            estimate.remaining_hours
        },
        test_hours: if finished {
            estimate.test_hours
        } else {
            estimate.remaining_test_hours
        },
        tester: estimate.tester,
        basis: estimate.basis,
        depends_on,
        target: date(item, "target"),
    })
}

/// The approximate closing date of a finished `item` without `closed`, from
/// the statuses git saw it go through (ADR-0027).
fn approximate(history: &History, item: &Item, effective: &Effective) -> Option<Date> {
    let is_done = |status: &str| {
        effective
            .status(status)
            .is_some_and(|status| status.category == Category::Done)
    };
    closed::approximate(history.transitions(&item.relative), is_done)?
        .parse()
        .ok()
}

/// The date the header writes under `key`, when it is a valid `YYYY-MM-DD`.
fn date(item: &Item, key: &str) -> Option<Date> {
    item.document.text(key)?.parse().ok()
}

/// The item's title, else its id.
fn title(item: &Item) -> String {
    item.document
        .text("title")
        .or_else(|| item.id())
        .unwrap_or_default()
        .to_owned()
}

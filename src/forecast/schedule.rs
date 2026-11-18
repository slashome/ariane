//! Orders the work and places it in sequence, in focused hours from now
//! (ADR-0028 §2 and §3).
//!
//! The work of the declared releases is sorted once: the work the owner is on
//! now (in progress or in testing) first, whatever its release, then release,
//! then epic, then story, then the work itself, each level in its manual
//! order, the work without epic or story after the groups. Grouping is a preference,
//! dependencies are constraints: the scheduler always takes the first piece of
//! work in that order whose dependencies are delivered, passing over the ones
//! that are not, so a group can show a gap. When nothing is ready because the
//! dependencies left loop on each other, the loop and every piece of work
//! waiting on it stay unscheduled.
//!
//! A release waits for the releases it depends on (ADR-0030): none of its
//! work starts before they are delivered, closed or with all their work
//! delivered, and the releases they depend on in turn, up to a closed one.
//! Work stuck behind a release that cannot be delivered is stuck like work
//! behind a task.
//!
//! Work handed to someone takes none of the owner's hours and ignores its
//! dependencies: work in a `waiting` status, and work in testing that others
//! test, are validated from now for their testing hours.

use std::collections::{HashMap, HashSet};

use crate::estimate::Tester;

use super::input::{Input, Planned, State, Unit};

/// When a piece of work happens, in focused hours from now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// Its place in the owner's sequence; `None` for work handed to someone,
    /// which takes no hours of the owner.
    pub sequence: Option<usize>,
    /// When the owner starts it.
    pub start: f64,
    /// When its hours of work end.
    pub work_end: f64,
    /// When the owner is done with it: after the testing when the owner tests.
    pub owner_end: f64,
    /// When someone else validates it, from the end of the work, without the
    /// owner's hours.
    pub validation: Option<(f64, f64)>,
    /// When it is delivered: tested and validated.
    pub delivered: f64,
}

/// What the scheduler decided.
#[derive(Debug, Default)]
pub struct Schedule {
    /// The placed work, by index in [`Input::units`].
    pub placed: HashMap<usize, Placed>,
    /// Work caught in a dependency cycle.
    pub cycles: Vec<usize>,
    /// Work waiting, directly or not, on a cycle.
    pub blocked: Vec<usize>,
    /// Dependencies that point to open work the scheduler never places, such
    /// as work of a parking release: (dependent, dependency), ignored.
    pub dangling: Vec<(usize, String)>,
}

/// Whether a unit is scheduled: open and planned in a declared release.
pub fn schedulable(unit: &Unit) -> bool {
    unit.state.is_open() && matches!(unit.release, Planned::Release(_))
}

/// The order the scheduler takes a unit in: started work first, then the
/// order of [`order_key`].
fn sequence_key(input: &Input, index: usize) -> (bool, (usize, usize, usize, usize, usize)) {
    (
        !input.units[index].state.is_started(),
        order_key(input, index),
    )
}

/// The order of a unit as the report shows it: release, epic, story, its own
/// place, then its index.
pub fn order_key(input: &Input, index: usize) -> (usize, usize, usize, usize, usize) {
    let unit = &input.units[index];
    let release = match unit.release {
        Planned::Release(position) => position,
        _ => usize::MAX,
    };
    let group = |reference: &Option<String>| {
        reference
            .as_deref()
            .and_then(|reference| {
                input
                    .groups
                    .iter()
                    .find(|group| group.reference == reference)
            })
            .map_or(usize::MAX, |group| group.order)
    };
    (
        release,
        group(&unit.epic),
        group(&unit.story),
        unit.order,
        index,
    )
}

/// Places every schedulable unit of `input`.
pub fn run(input: &Input) -> Schedule {
    let mut schedule = Schedule::default();
    let mut pending: Vec<usize> = (0..input.units.len())
        .filter(|index| schedulable(&input.units[*index]))
        .collect();
    pending.sort_by_key(|index| sequence_key(input, *index));
    let dependencies = resolve(input, &pending, &mut schedule.dangling);
    for index in &pending {
        let unit = &input.units[*index];
        if is_handed_over(unit) {
            schedule.placed.insert(
                *index,
                Placed {
                    sequence: None,
                    start: 0.0,
                    work_end: 0.0,
                    owner_end: 0.0,
                    validation: Some((0.0, unit.test_hours)),
                    delivered: unit.test_hours,
                },
            );
        }
    }
    pending.retain(|index| !schedule.placed.contains_key(index));
    let mut gates = Gates::new(input, &pending, &schedule.placed);
    let mut clock = 0.0_f64;
    let mut sequence = 0;
    while !pending.is_empty() {
        let ready: Vec<(usize, f64)> = pending
            .iter()
            .enumerate()
            .filter_map(|(slot, index)| {
                let delivered = dependencies[index]
                    .iter()
                    .map(|dependency| {
                        schedule
                            .placed
                            .get(dependency)
                            .map(|placed| placed.delivered)
                    })
                    .try_fold(0.0_f64, |latest, delivered| Some(latest.max(delivered?)))?;
                let opened = gates.opened(&input.units[*index])?;
                Some((slot, delivered.max(opened)))
            })
            .collect();
        let Some(&(slot, ready_at)) = ready
            .iter()
            .find(|(_, ready_at)| *ready_at <= clock)
            .or_else(|| {
                ready
                    .iter()
                    .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            })
        else {
            break;
        };
        let index = pending.remove(slot);
        let unit = &input.units[index];
        let start = clock.max(ready_at);
        let work_end = start + unit.hours;
        let placed = match unit.tester {
            Tester::Owner => Placed {
                sequence: Some(sequence),
                start,
                work_end,
                owner_end: work_end + unit.test_hours,
                validation: None,
                delivered: work_end + unit.test_hours,
            },
            Tester::Others => Placed {
                sequence: Some(sequence),
                start,
                work_end,
                owner_end: work_end,
                validation: (unit.test_hours > 0.0)
                    .then_some((work_end, work_end + unit.test_hours)),
                delivered: work_end + unit.test_hours,
            },
        };
        clock = placed.owner_end;
        sequence += 1;
        gates.place(unit, placed.delivered);
        schedule.placed.insert(index, placed);
    }
    let stuck: HashSet<usize> = pending.iter().copied().collect();
    let waits = waits_with_releases(input, &dependencies, &gates, &stuck);
    for index in pending {
        if reaches(index, index, &waits, &stuck) {
            schedule.cycles.push(index);
        } else {
            schedule.blocked.push(index);
        }
    }
    schedule
}

/// When each release is delivered, as the scheduler places its work.
struct Gates {
    /// The releases each release waits for, directly or through the releases
    /// it depends on, up to a closed one.
    waits: Vec<Vec<usize>>,
    /// The work of each release the owner still has to place.
    remaining: Vec<usize>,
    /// When the work placed in each release is delivered, at the latest.
    delivered: Vec<f64>,
}

impl Gates {
    /// The gates of the releases of `input`, `pending` being the work left to
    /// the owner and `placed` the work handed to someone.
    fn new(input: &Input, pending: &[usize], placed: &HashMap<usize, Placed>) -> Gates {
        let count = input.releases.len();
        let mut waits: Vec<Vec<usize>> = Vec::with_capacity(count);
        for release in &input.releases {
            let mut all = Vec::new();
            for dependency in &release.depends_on {
                all.push(*dependency);
                if input.releases[*dependency].closed.is_none() {
                    all.extend(waits[*dependency].iter().copied());
                }
            }
            all.sort_unstable();
            all.dedup();
            waits.push(all);
        }
        let mut gates = Gates {
            waits,
            remaining: vec![0; count],
            delivered: vec![0.0; count],
        };
        for index in pending {
            if let Planned::Release(release) = input.units[*index].release {
                gates.remaining[release] += 1;
            }
        }
        for (index, placed) in placed {
            if let Planned::Release(release) = input.units[*index].release {
                gates.delivered[release] = gates.delivered[release].max(placed.delivered);
            }
        }
        gates
    }

    /// When the releases `unit`'s release waits for are all delivered; `None`
    /// while one of them still has work to place.
    fn opened(&self, unit: &Unit) -> Option<f64> {
        let Planned::Release(release) = unit.release else {
            return Some(0.0);
        };
        self.waits[release]
            .iter()
            .try_fold(0.0_f64, |latest, wait| {
                (self.remaining[*wait] == 0).then(|| latest.max(self.delivered[*wait]))
            })
    }

    /// Records that `unit` is placed, delivered at `delivered`.
    fn place(&mut self, unit: &Unit, delivered: f64) {
        if let Planned::Release(release) = unit.release {
            self.remaining[release] -= 1;
            self.delivered[release] = self.delivered[release].max(delivered);
        }
    }
}

/// What each stuck unit waits for among the stuck units: its own
/// dependencies, and the stuck work of the releases its release waits for.
fn waits_with_releases(
    input: &Input,
    dependencies: &HashMap<usize, Vec<usize>>,
    gates: &Gates,
    stuck: &HashSet<usize>,
) -> HashMap<usize, Vec<usize>> {
    let mut by_release: HashMap<usize, Vec<usize>> = HashMap::new();
    for index in stuck {
        if let Planned::Release(release) = input.units[*index].release {
            by_release.entry(release).or_default().push(*index);
        }
    }
    stuck
        .iter()
        .map(|index| {
            let mut waits = dependencies[index].clone();
            if let Planned::Release(release) = input.units[*index].release {
                for wait in &gates.waits[release] {
                    waits.extend(by_release.get(wait).into_iter().flatten().copied());
                }
            }
            (*index, waits)
        })
        .collect()
}

/// Tells whether a unit is with someone else, taking none of the owner's
/// hours: waiting, or in testing that others do.
fn is_handed_over(unit: &Unit) -> bool {
    match unit.state {
        State::Waiting => true,
        State::Testing => unit.tester == Tester::Others,
        _ => false,
    }
}

/// The scheduled units each scheduled unit waits for. A reference to a task
/// names it; to a story or an epic, every unit inside it. A reference to work
/// that is finished, or to a story or epic with no open work, is met; one to
/// open work that is never scheduled is recorded in `dangling` and ignored.
fn resolve(
    input: &Input,
    pending: &[usize],
    dangling: &mut Vec<(usize, String)>,
) -> HashMap<usize, Vec<usize>> {
    let mut members: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, unit) in input.units.iter().enumerate() {
        let mut names = vec![unit.reference.as_str()];
        names.extend(unit.story.as_deref());
        names.extend(unit.epic.as_deref());
        names.dedup();
        for name in names {
            members.entry(name).or_default().push(index);
        }
    }
    let known_group = |reference: &str| {
        input
            .groups
            .iter()
            .any(|group| group.reference == reference)
    };
    let mut dependencies = HashMap::new();
    for index in pending {
        let unit = &input.units[*index];
        let mut waits = Vec::new();
        for reference in &unit.depends_on {
            let Some(targets) = members.get(reference.as_str()) else {
                if !known_group(reference) {
                    dangling.push((*index, reference.clone()));
                }
                continue;
            };
            let open: Vec<usize> = targets
                .iter()
                .copied()
                .filter(|target| *target != *index && input.units[*target].state.is_open())
                .collect();
            if open
                .iter()
                .any(|target| !schedulable(&input.units[*target]))
            {
                dangling.push((*index, reference.clone()));
            }
            waits.extend(
                open.into_iter()
                    .filter(|target| schedulable(&input.units[*target])),
            );
        }
        waits.sort_unstable();
        waits.dedup();
        dependencies.insert(*index, waits);
    }
    dependencies
}

/// Tells whether `target` can be reached from `from` through dependencies
/// among the `stuck` units, in one step or more.
fn reaches(
    from: usize,
    target: usize,
    dependencies: &HashMap<usize, Vec<usize>>,
    stuck: &HashSet<usize>,
) -> bool {
    let mut seen = HashSet::new();
    let mut stack: Vec<usize> = dependencies.get(&from).cloned().unwrap_or_default();
    while let Some(current) = stack.pop() {
        if current == target {
            return true;
        }
        if !stuck.contains(&current) || !seen.insert(current) {
            continue;
        }
        stack.extend(dependencies.get(&current).into_iter().flatten().copied());
    }
    false
}

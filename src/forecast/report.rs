//! Turns the schedule into the [`Forecast`]: moments, groups and warnings.
//!
//! The schedule covers the whole scope; the report keeps the work of the
//! node asked, the input's focus, and of its descendants. Every summary is
//! computed from that work alone, so the span of a release, an epic or a
//! story is the span of the focus's part of it, placed where the pooled
//! sequence puts it. A release is kept when it holds some of that work or
//! when its list concerns the focus; warnings keep the items of the focus
//! and the releases kept.

use std::collections::{BTreeMap, HashMap};

use jiff::Timestamp;
use jiff::civil::Date;

use crate::estimate::Basis;
use crate::item::Kind;
use crate::tree::NodePath;

use super::input::{GroupSpec, Input, Planned, State, Unit};
use super::schedule::{Placed, Schedule, order_key};
use super::timeline::Timeline;
use super::{
    ClosedDate, DateRange, EpicPlan, Flagged, Forecast, Moment, Phase, Reason, ReleasePlan,
    Segment, Span, Stage, StoryPlan, Summary, Unscheduled, Warning, WarningKind, WorkPlan,
};

/// Converts focused hours into moments, dated when a timeline exists.
struct Clock {
    /// The carrier's blocks laid out from now, if any.
    timeline: Option<Timeline>,
}

impl Clock {
    /// The moment work starting at `hours` starts.
    fn start(&mut self, hours: f64) -> Moment {
        let at = self.timeline.as_mut().and_then(|timeline| {
            let moment = timeline.start_at(hours)?;
            Some(render(timeline, moment))
        });
        Moment {
            hours: round(hours),
            at,
        }
    }

    /// The moment work ending at `hours` ends.
    fn end(&mut self, hours: f64) -> Moment {
        let at = self.timeline.as_mut().and_then(|timeline| {
            let moment = timeline.end_at(hours)?;
            Some(render(timeline, moment))
        });
        Moment {
            hours: round(hours),
            at,
        }
    }

    /// The day work ending at `hours` ends, in the calendar's time zone.
    fn end_date(&mut self, hours: f64) -> Option<Date> {
        let timeline = self.timeline.as_mut()?;
        let moment = timeline.end_at(hours)?;
        Some(moment.to_zoned(timeline.zone().clone()).date())
    }

    /// The stretches of `phase` from `from` to `to`: one per block when dated,
    /// one in hours otherwise.
    fn segments(&mut self, phase: Phase, from: f64, to: f64) -> Vec<Segment> {
        if to <= from {
            return Vec::new();
        }
        let pieces = self
            .timeline
            .as_mut()
            .and_then(|timeline| timeline.pieces(from, to));
        let (Some(pieces), Some(timeline)) = (pieces, self.timeline.as_ref()) else {
            return vec![Segment {
                phase,
                start: Moment {
                    hours: round(from),
                    at: None,
                },
                end: Moment {
                    hours: round(to),
                    at: None,
                },
            }];
        };
        pieces
            .into_iter()
            .map(|piece| Segment {
                phase,
                start: Moment {
                    hours: round(piece.from),
                    at: Some(render(timeline, piece.start)),
                },
                end: Moment {
                    hours: round(piece.to),
                    at: Some(render(timeline, piece.end)),
                },
            })
            .collect()
    }
}

/// `moment` written RFC 3339 in the timeline's time zone.
fn render(timeline: &Timeline, moment: Timestamp) -> String {
    moment
        .to_zoned(timeline.zone().clone())
        .strftime("%Y-%m-%dT%H:%M:%S%:z")
        .to_string()
}

/// `value` rounded to the hundredth.
fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Builds the forecast of `input` from its `schedule`.
pub fn assemble(input: &Input, schedule: &Schedule) -> Forecast {
    let timeline = input
        .capacity
        .as_ref()
        .and_then(|capacity| Timeline::new(capacity, input.now));
    let dated = timeline.is_some();
    let now = match &timeline {
        Some(timeline) => render(timeline, input.now),
        None => input.now.to_string(),
    };
    let timezone = timeline
        .as_ref()
        .and(input.capacity.as_ref())
        .map(|capacity| capacity.timezone.clone());
    let mut clock = Clock { timeline };
    let groups: HashMap<&str, &GroupSpec> = input
        .groups
        .iter()
        .map(|group| (group.reference.as_str(), group))
        .collect();
    let mut works: HashMap<usize, WorkPlan> = HashMap::new();
    for (index, unit) in input.units.iter().enumerate() {
        let placed = schedule.placed.get(&index);
        let shown = placed.is_some() || matches!(unit.state, State::Done { .. });
        if shown && matches!(unit.release, Planned::Release(_)) && in_focus(input, &unit.node) {
            works.insert(index, work_plan(unit, placed, &mut clock));
        }
    }
    let releases: Vec<ReleasePlan> = input
        .releases
        .iter()
        .enumerate()
        .filter_map(|(position, spec)| {
            let mut members: Vec<usize> = works
                .keys()
                .copied()
                .filter(|index| input.units[*index].release == Planned::Release(position))
                .collect();
            if members.is_empty() && !applies(&spec.declared_by, &input.focus) {
                return None;
            }
            members.sort_by_key(|index| order_key(input, *index));
            let context = Context {
                input,
                groups: &groups,
                placed: &schedule.placed,
                works: &works,
            };
            Some(ReleasePlan {
                name: spec.name.clone(),
                declared_by: spec.declared_by.clone(),
                target: spec.target.map(|date| date.to_string()),
                closed_on: spec.closed.map(|date| date.to_string()),
                summary: summarize(input, &members, &schedule.placed, &mut clock),
                epics: epic_plans(&context, &members, &mut clock),
            })
        })
        .collect();
    let (hours, test_hours) = schedule
        .placed
        .keys()
        .map(|index| &input.units[*index])
        .filter(|unit| in_focus(input, &unit.node))
        .fold((0.0, 0.0), |(hours, tests), unit| {
            (hours + unit.hours, tests + unit.test_hours)
        });
    let warnings = warnings(input, schedule, &groups, &mut clock)
        .into_iter()
        .filter_map(|warning| narrow(input, &releases, warning))
        .collect();
    Forecast {
        node: input.focus.clone(),
        carrier: input.carrier.clone(),
        timezone,
        now,
        dated,
        hours: round(hours),
        test_hours: round(test_hours),
        releases,
        unscheduled: unscheduled(input, schedule),
        warnings,
    }
}

/// Tells whether `node` holds work of the node asked: it or one of its
/// descendants.
fn in_focus(input: &Input, node: &NodePath) -> bool {
    node.is_within(&input.focus)
}

/// Tells whether the release list of `owner` concerns `focus`: declared by
/// it, by one of its ancestors or by one of its descendants.
fn applies(owner: &NodePath, focus: &NodePath) -> bool {
    focus.is_within(owner) || owner.is_within(focus)
}

/// `warning` with only the items of the node asked: work of it or of its
/// descendants, and the releases the forecast keeps; `None` when none is left.
fn narrow(input: &Input, releases: &[ReleasePlan], mut warning: Warning) -> Option<Warning> {
    warning
        .items
        .retain(|item| match item.reference.strip_prefix("release ") {
            Some(name) => releases.iter().any(|release| release.name == name),
            None => item
                .reference
                .split_once('#')
                .is_some_and(|(node, _)| in_focus(input, &NodePath::new(node))),
        });
    warning.count = warning.items.len();
    (warning.count > 0).then_some(warning)
}

/// What grouping the work of a release reads.
struct Context<'a> {
    /// The input.
    input: &'a Input,
    /// The stories and epics, by reference.
    groups: &'a HashMap<&'a str, &'a GroupSpec>,
    /// The placed units.
    placed: &'a HashMap<usize, Placed>,
    /// The plan of each shown unit.
    works: &'a HashMap<usize, WorkPlan>,
}

/// The plan of one piece of work.
fn work_plan(unit: &Unit, placed: Option<&Placed>, clock: &mut Clock) -> WorkPlan {
    let (stage, closed) = match unit.state {
        State::Done {
            closed,
            approximate,
        } => (
            Stage::Done,
            closed.map(|date| ClosedDate {
                date: date.to_string(),
                approximate,
            }),
        ),
        State::Waiting => (Stage::Waiting, None),
        State::InProgress => (Stage::InProgress, None),
        State::Testing => (Stage::Testing, None),
        State::Open | State::Dropped => (Stage::Open, None),
    };
    let mut plan = WorkPlan {
        reference: unit.reference.clone(),
        node: unit.node.clone(),
        kind: unit.kind,
        title: unit.title.clone(),
        stage,
        hours: round(unit.hours),
        test_hours: round(unit.test_hours),
        tester: unit.tester,
        basis: unit.basis,
        target: unit.target.map(|date| date.to_string()),
        sequence: None,
        start: None,
        end: None,
        delivered: None,
        segments: Vec::new(),
        validation: None,
        closed,
    };
    if let Some(placed) = placed {
        plan.sequence = placed.sequence;
        plan.start = Some(clock.start(placed.start));
        plan.end = Some(clock.end(placed.owner_end));
        plan.delivered = Some(clock.end(placed.delivered));
        plan.segments = clock.segments(Phase::Work, placed.start, placed.work_end);
        plan.segments
            .extend(clock.segments(Phase::Test, placed.work_end, placed.owner_end));
        plan.validation = placed.validation.map(|(start, end)| Span {
            start: clock.start(start),
            end: clock.end(end),
        });
    }
    plan
}

/// What the units `members` add up to.
fn summarize(
    input: &Input,
    members: &[usize],
    placed: &HashMap<usize, Placed>,
    clock: &mut Clock,
) -> Summary {
    let mut hours = 0.0;
    let mut test_hours = 0.0;
    let mut bounds: Option<(f64, f64)> = None;
    let mut closed: Vec<Date> = Vec::new();
    let mut done = 0;
    let mut open = 0;
    for index in members {
        let unit = &input.units[*index];
        if let State::Done { closed: date, .. } = unit.state {
            done += 1;
            closed.extend(date);
            continue;
        }
        open += 1;
        let Some(placed) = placed.get(index) else {
            continue;
        };
        hours += unit.hours;
        test_hours += unit.test_hours;
        bounds = Some(match bounds {
            Some((start, end)) => (start.min(placed.start), end.max(placed.delivered)),
            None => (placed.start, placed.delivered),
        });
    }
    Summary {
        hours: round(hours),
        test_hours: round(test_hours),
        span: bounds.map(|(start, end)| Span {
            start: clock.start(start),
            end: clock.end(end),
        }),
        closed: closed
            .iter()
            .min()
            .zip(closed.iter().max())
            .map(|(from, to)| DateRange {
                from: from.to_string(),
                to: to.to_string(),
            }),
        done,
        open,
    }
}

/// The distinct values `key` gives the units `members`, in their order.
fn distinct<'a>(
    input: &'a Input,
    members: &[usize],
    key: impl Fn(&'a Unit) -> Option<&'a str>,
) -> Vec<Option<&'a str>> {
    let mut seen = Vec::new();
    for index in members {
        let value = key(&input.units[*index]);
        if !seen.contains(&value) {
            seen.push(value);
        }
    }
    seen
}

/// The work `members` of one release by epic, then by story. `members` is
/// sorted by [`order_key`], so the groups come in their order, the work
/// without epic or story last.
fn epic_plans(context: &Context, members: &[usize], clock: &mut Clock) -> Vec<EpicPlan> {
    let input = context.input;
    distinct(input, members, |unit| unit.epic.as_deref())
        .into_iter()
        .map(|epic| {
            let inside: Vec<usize> = members
                .iter()
                .copied()
                .filter(|index| input.units[*index].epic.as_deref() == epic)
                .collect();
            let stories = distinct(input, &inside, |unit| unit.story.as_deref())
                .into_iter()
                .map(|story| {
                    let work: Vec<usize> = inside
                        .iter()
                        .copied()
                        .filter(|index| input.units[*index].story.as_deref() == story)
                        .collect();
                    let group = story.and_then(|story| context.groups.get(story));
                    StoryPlan {
                        reference: story.map(str::to_owned),
                        title: group.map(|group| group.title.clone()),
                        target: group
                            .and_then(|group| group.target)
                            .map(|date| date.to_string()),
                        summary: summarize(input, &work, context.placed, clock),
                        work: work
                            .iter()
                            .filter_map(|index| context.works.get(index).cloned())
                            .collect(),
                    }
                })
                .collect();
            let group = epic.and_then(|epic| context.groups.get(epic));
            EpicPlan {
                reference: epic.map(str::to_owned),
                title: group.map(|group| group.title.clone()),
                target: group
                    .and_then(|group| group.target)
                    .map(|date| date.to_string()),
                summary: summarize(input, &inside, context.placed, clock),
                stories,
            }
        })
        .collect()
}

/// The open work that is not scheduled, with why.
fn unscheduled(input: &Input, schedule: &Schedule) -> Vec<Unscheduled> {
    let mut entries: Vec<(usize, Reason)> = Vec::new();
    for (index, unit) in input.units.iter().enumerate() {
        if !unit.state.is_open() || !in_focus(input, &unit.node) {
            continue;
        }
        let reason = match &unit.release {
            Planned::Parking(_) => Reason::Parking,
            Planned::Closed(_) => Reason::ClosedRelease,
            Planned::Unplanned => Reason::NoRelease,
            Planned::Undeclared(_) => Reason::UndeclaredRelease,
            Planned::Release(_) if schedule.cycles.contains(&index) => Reason::Cycle,
            Planned::Release(_) if schedule.blocked.contains(&index) => Reason::BlockedByCycle,
            Planned::Release(_) => continue,
        };
        entries.push((index, reason));
    }
    entries.sort_by_key(|(index, _)| order_key(input, *index));
    entries
        .into_iter()
        .map(|(index, reason)| {
            let unit = &input.units[index];
            Unscheduled {
                reference: unit.reference.clone(),
                node: unit.node.clone(),
                kind: unit.kind,
                title: unit.title.clone(),
                reason,
                release: release_name(input, &unit.release),
            }
        })
        .collect()
}

/// The name of a unit's effective release.
fn release_name(input: &Input, planned: &Planned) -> Option<String> {
    match planned {
        Planned::Release(position) => Some(input.releases[*position].name.clone()),
        Planned::Parking(name) | Planned::Closed(name) | Planned::Undeclared(name) => {
            Some(name.clone())
        }
        Planned::Unplanned => None,
    }
}

/// Collects warnings, one item at most once per kind.
#[derive(Default)]
struct Warnings {
    /// The items of each kind, in the order found.
    kinds: BTreeMap<WarningKind, Vec<Flagged>>,
}

impl Warnings {
    /// Flags `reference` under `kind`, unless it already is.
    fn flag(&mut self, kind: WarningKind, reference: &str, detail: Option<String>) {
        let items = self.kinds.entry(kind).or_default();
        if !items.iter().any(|item| item.reference == reference) {
            items.push(Flagged {
                reference: reference.to_owned(),
                detail,
            });
        }
    }

    /// The warnings, kinds in their declared order.
    fn finish(self) -> Vec<Warning> {
        self.kinds
            .into_iter()
            .filter(|(_, items)| !items.is_empty())
            .map(|(kind, items)| Warning {
                kind,
                count: items.len(),
                items,
            })
            .collect()
    }
}

/// Every warning of the forecast (ADR-0028 §5).
fn warnings(
    input: &Input,
    schedule: &Schedule,
    groups: &HashMap<&str, &GroupSpec>,
    clock: &mut Clock,
) -> Vec<Warning> {
    let mut found = Warnings::default();
    let mut order: Vec<usize> = (0..input.units.len()).collect();
    order.sort_by_key(|index| order_key(input, *index));
    for index in &order {
        let unit = &input.units[*index];
        if schedule.placed.contains_key(index) {
            estimates(unit, &mut found);
        }
        if unit.state.is_open() {
            hierarchy(unit, groups, &mut found);
        }
    }
    for index in &schedule.cycles {
        found.flag(
            WarningKind::DependencyCycle,
            &input.units[*index].reference,
            None,
        );
    }
    for (index, dependency) in &schedule.dangling {
        found.flag(
            WarningKind::UnscheduledDependency,
            &input.units[*index].reference,
            Some(format!("depends on {dependency}, which is not scheduled")),
        );
    }
    let ends = Ends::new(input, schedule, clock);
    missed_targets(input, groups, &ends, &order, &mut found);
    contradictory_targets(input, groups, &order, &mut found);
    found.finish()
}

/// Flags the guessed hours of scheduled work.
fn estimates(unit: &Unit, found: &mut Warnings) {
    match unit.kind {
        Kind::Task if unit.basis == Basis::Default => {
            found.flag(WarningKind::Unsized, &unit.reference, None);
        }
        Kind::Story | Kind::Epic => {
            let detail = match unit.basis {
                Basis::Default => "no size: counted at the default size",
                _ => "weighed by its provisional size",
            };
            found.flag(
                WarningKind::Provisional,
                &unit.reference,
                Some(detail.to_owned()),
            );
        }
        _ => {}
    }
}

/// Flags open work outside the hierarchy or outside the declared releases.
fn hierarchy(unit: &Unit, groups: &HashMap<&str, &GroupSpec>, found: &mut Warnings) {
    if unit.kind == Kind::Task && unit.story.is_none() {
        found.flag(WarningKind::WithoutStory, &unit.reference, None);
    }
    if let Some(story) = unit.story.as_deref()
        && groups.get(story).is_some_and(|group| group.epic.is_none())
    {
        found.flag(WarningKind::WithoutEpic, story, None);
    }
    match &unit.release {
        Planned::Unplanned => found.flag(WarningKind::WithoutRelease, &unit.reference, None),
        Planned::Undeclared(name) => found.flag(
            WarningKind::UndeclaredRelease,
            &unit.reference,
            Some(format!(
                "release `{name}` is declared in no applicable list"
            )),
        ),
        Planned::Release(_) | Planned::Parking(_) | Planned::Closed(_) => {}
    }
}

/// The day each piece of work ends: its closing date when finished, its
/// delivery when scheduled and dated; unknown otherwise.
struct Ends {
    /// By unit index.
    days: HashMap<usize, Date>,
    /// The units whose end is unknown although they are scheduled: the
    /// forecast is not dated, or the work lies beyond the horizon.
    unknown: Vec<usize>,
}

impl Ends {
    /// The ends of every unit of `input`.
    fn new(input: &Input, schedule: &Schedule, clock: &mut Clock) -> Ends {
        let mut days = HashMap::new();
        let mut unknown = Vec::new();
        for (index, unit) in input.units.iter().enumerate() {
            if let State::Done {
                closed: Some(date), ..
            } = unit.state
            {
                days.insert(index, date);
            } else if let Some(placed) = schedule.placed.get(&index) {
                match clock.end_date(placed.delivered) {
                    Some(date) => {
                        days.insert(index, date);
                    }
                    None => unknown.push(index),
                }
            }
        }
        Ends { days, unknown }
    }

    /// The last end of the units `inside` accepts; `None` when one of them
    /// has an unknown end or none has an end.
    fn last(&self, input: &Input, inside: impl Fn(&Unit) -> bool) -> Option<Date> {
        let mut last = None;
        for (index, unit) in input.units.iter().enumerate() {
            if !inside(unit) {
                continue;
            }
            if self.unknown.contains(&index) {
                return None;
            }
            if let Some(day) = self.days.get(&index) {
                last = Some(last.map_or(*day, |last: Date| last.max(*day)));
            }
        }
        last
    }
}

/// Flags the targets the forecast misses: of work, stories, epics, releases.
fn missed_targets(
    input: &Input,
    groups: &HashMap<&str, &GroupSpec>,
    ends: &Ends,
    order: &[usize],
    found: &mut Warnings,
) {
    let missed = |target: Date, end: Date| {
        (end > target).then(|| format!("target {target}, forecast {end}"))
    };
    for index in order {
        let unit = &input.units[*index];
        if let (Some(target), Some(end)) = (unit.target, ends.days.get(index))
            && let Some(detail) = missed(target, *end)
        {
            found.flag(WarningKind::MissedTarget, &unit.reference, Some(detail));
        }
    }
    let mut ordered: Vec<&&GroupSpec> = groups.values().collect();
    ordered.sort_by_key(|group| (group.kind == Kind::Story, group.order));
    for group in ordered {
        let Some(target) = group.target else {
            continue;
        };
        let reference = group.reference.as_str();
        let end = ends.last(input, |unit| {
            unit.story.as_deref() == Some(reference) || unit.epic.as_deref() == Some(reference)
        });
        if let Some(detail) = end.and_then(|end| missed(target, end)) {
            found.flag(WarningKind::MissedTarget, reference, Some(detail));
        }
    }
    for (position, release) in input.releases.iter().enumerate() {
        let Some(target) = release.target else {
            continue;
        };
        let end = ends.last(input, |unit| unit.release == Planned::Release(position));
        if let Some(detail) = end.and_then(|end| missed(target, end)) {
            found.flag(
                WarningKind::MissedTarget,
                &format!("release {}", release.name),
                Some(detail),
            );
        }
    }
}

/// Flags targets later than the target of what contains them: a task's
/// story or epic, a story's epic, a piece of work's release.
fn contradictory_targets(
    input: &Input,
    groups: &HashMap<&str, &GroupSpec>,
    order: &[usize],
    found: &mut Warnings,
) {
    let later = |own: Date, other: Option<Date>, whose: &str| {
        let other = other?;
        (own > other).then(|| format!("target {own} is later than {whose}'s {other}"))
    };
    for index in order {
        let unit = &input.units[*index];
        let Some(target) = unit.target else {
            continue;
        };
        let containers = [unit.story.as_deref(), unit.epic.as_deref()];
        for container in containers.into_iter().flatten() {
            if container == unit.reference {
                continue;
            }
            let other = groups.get(container).and_then(|group| group.target);
            if let Some(detail) = later(target, other, container) {
                found.flag(
                    WarningKind::ContradictoryTarget,
                    &unit.reference,
                    Some(detail),
                );
            }
        }
        if let Planned::Release(position) = unit.release {
            let release = &input.releases[position];
            let whose = format!("release {}", release.name);
            if let Some(detail) = later(target, release.target, &whose) {
                found.flag(
                    WarningKind::ContradictoryTarget,
                    &unit.reference,
                    Some(detail),
                );
            }
        }
    }
    let mut stories: Vec<&&GroupSpec> = groups
        .values()
        .filter(|group| group.kind == Kind::Story)
        .collect();
    stories.sort_by_key(|group| group.order);
    for story in stories {
        let (Some(target), Some(epic)) = (story.target, story.epic.as_deref()) else {
            continue;
        };
        let other = groups.get(epic).and_then(|group| group.target);
        if let Some(detail) = later(target, other, epic) {
            found.flag(
                WarningKind::ContradictoryTarget,
                &story.reference,
                Some(detail),
            );
        }
    }
}

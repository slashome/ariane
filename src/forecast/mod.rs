//! The forecast: when each release should be delivered (ADR-0028, SPEC.md §3).
//!
//! This is the only place the forecast is computed; `ariane roadmap` and the
//! interface read what it returns, and nothing it computes is ever written to
//! a file. It is split in four steps:
//!
//! - [`gather`] reads the tree, the node's releases, the estimates and the
//!   calendar into an [`Input`] of plain data;
//! - the scheduler orders the work and places it in sequence, in focused hours
//!   from now, the same with or without a calendar;
//! - a timeline turns those hours into moments of the carrier's blocks, times
//!   `focus`, days off skipped, in the calendar's time zone;
//! - the report groups the result by release, epic and story, and counts its
//!   warnings.
//!
//! [`compute`] runs the last three on any [`Input`], with `now` given, so the
//! engine is tested on fixtures without a clock or a file system; [`build`]
//! and [`load`] add the first.

mod gather;
pub mod input;
mod report;
mod schedule;
mod timeline;

#[cfg(test)]
mod tests;

use std::path::Path;

use anyhow::Result;
use jiff::Timestamp;
use serde::Serialize;

use crate::calendar::{self, Calendar};
use crate::estimate::{Basis, Tester};
use crate::history::History;
use crate::item::Kind;
use crate::tree::{NodePath, Tree};

pub use gather::gather;
pub use input::Input;

/// The forecast of one node.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Forecast {
    /// The node the sequence runs on: the calendar's carrier of the node
    /// asked, else the node asked. Its descendants share the sequence.
    pub node: NodePath,
    /// The node whose blocks give the dates; `None` without a calendar.
    pub carrier: Option<NodePath>,
    /// The calendar's time zone, when the forecast is dated.
    pub timezone: Option<String>,
    /// The moment the forecast starts from, RFC 3339.
    pub now: String,
    /// Whether moments carry dates; without a calendar they only carry hours.
    pub dated: bool,
    /// Hours of work left in the scheduled releases.
    pub hours: f64,
    /// Hours of testing left in them.
    pub test_hours: f64,
    /// The declared releases, in delivery order, parking ones left out.
    pub releases: Vec<ReleasePlan>,
    /// Open work shown but not scheduled, with the reason.
    pub unscheduled: Vec<Unscheduled>,
    /// What makes the forecast uncertain or the plan contradictory, by kind.
    pub warnings: Vec<Warning>,
}

/// What a group of work adds up to.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Summary {
    /// Hours of work left.
    pub hours: f64,
    /// Hours of testing left.
    pub test_hours: f64,
    /// From the start of its first scheduled work to the delivery of its last;
    /// `None` when nothing of it is left to do.
    pub span: Option<Span>,
    /// The closing dates of its finished work, first and last.
    pub closed: Option<DateRange>,
    /// How many pieces of its work are finished.
    pub done: usize,
    /// How many are left.
    pub open: usize,
}

/// One release of the forecast.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReleasePlan {
    /// Its name.
    pub name: String,
    /// The node whose list declares it.
    pub declared_by: NodePath,
    /// Its desired date, `YYYY-MM-DD`.
    pub target: Option<String>,
    /// What it adds up to.
    #[serde(flatten)]
    pub summary: Summary,
    /// Its work by epic, in order; the work without epic last.
    pub epics: Vec<EpicPlan>,
}

/// The fragment of an epic that falls in one release.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EpicPlan {
    /// `<node>#<id>`; `None` for the work without epic.
    pub reference: Option<String>,
    /// Its title.
    pub title: Option<String>,
    /// Its desired date.
    pub target: Option<String>,
    /// What the fragment adds up to.
    #[serde(flatten)]
    pub summary: Summary,
    /// Its work by story, in order; the work without story last.
    pub stories: Vec<StoryPlan>,
}

/// The fragment of a story that falls in one release.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StoryPlan {
    /// `<node>#<id>`; `None` for the work without story.
    pub reference: Option<String>,
    /// Its title.
    pub title: Option<String>,
    /// Its desired date.
    pub target: Option<String>,
    /// What the fragment adds up to.
    #[serde(flatten)]
    pub summary: Summary,
    /// Its work, in order.
    pub work: Vec<WorkPlan>,
}

/// Where a piece of work stands, for the reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Not started.
    Open,
    /// Being worked on.
    InProgress,
    /// Its work done, its testing left: a status of the `testing` stage.
    Testing,
    /// Handed to someone.
    Waiting,
    /// Finished.
    Done,
}

/// One piece of work: a task, or a story or an epic not split yet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkPlan {
    /// `<node>#<id>`.
    pub reference: String,
    /// The node holding it.
    pub node: NodePath,
    /// `task`, or `story` and `epic` for a block weighed by its own size.
    pub kind: Kind,
    /// Its title.
    pub title: String,
    /// Where it stands.
    pub stage: Stage,
    /// Hours of work left, 0 in testing; for finished work, the hours it
    /// weighed.
    pub hours: f64,
    /// Hours of testing after it.
    pub test_hours: f64,
    /// Who tests it.
    pub tester: Tester,
    /// Where its hours come from.
    pub basis: Basis,
    /// Its desired date.
    pub target: Option<String>,
    /// Its place in the owner's sequence, from 0; `None` when it takes none of
    /// the owner's hours: finished or handed to someone.
    pub sequence: Option<usize>,
    /// When the owner starts it.
    pub start: Option<Moment>,
    /// When the owner is done with it, testing included when the owner tests.
    pub end: Option<Moment>,
    /// When it is delivered, someone else's validation included.
    pub delivered: Option<Moment>,
    /// The stretches the owner works on it, block by block when dated.
    pub segments: Vec<Segment>,
    /// The validation by someone else, which takes none of the owner's hours.
    pub validation: Option<Span>,
    /// When it closed, for finished work.
    pub closed: Option<ClosedDate>,
}

/// A moment of the forecast.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Moment {
    /// Hours of focused work from now.
    pub hours: f64,
    /// The moment in the calendar, RFC 3339 in its time zone; `None` without
    /// a calendar, or beyond the ten years laid out.
    pub at: Option<String>,
}

/// From one moment to another.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Span {
    /// The first moment.
    pub start: Moment,
    /// The last moment.
    pub end: Moment,
}

/// What a stretch of the owner's hours goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// The work itself.
    Work,
    /// The testing, when the owner tests.
    Test,
}

/// One stretch of the owner's hours on a piece of work.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Segment {
    /// Work or testing.
    pub phase: Phase,
    /// When it starts.
    pub start: Moment,
    /// When it ends.
    pub end: Moment,
}

/// The closing date of finished work.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClosedDate {
    /// `YYYY-MM-DD`.
    pub date: String,
    /// Whether it is approximated from git (ADR-0027).
    pub approximate: bool,
}

/// The first and last closing dates of finished work.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DateRange {
    /// The first, `YYYY-MM-DD`.
    pub from: String,
    /// The last, `YYYY-MM-DD`.
    pub to: String,
}

/// Why open work is not scheduled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Its release is a parking one.
    Parking,
    /// No release on it, its story or its epic.
    NoRelease,
    /// Its release is declared in no applicable list.
    UndeclaredRelease,
    /// It is part of a dependency cycle.
    Cycle,
    /// It waits, directly or not, on a dependency cycle.
    BlockedByCycle,
}

/// Open work the forecast shows but does not schedule.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Unscheduled {
    /// `<node>#<id>`.
    pub reference: String,
    /// The node holding it.
    pub node: NodePath,
    /// Its kind.
    pub kind: Kind,
    /// Its title.
    pub title: String,
    /// Why it is not scheduled.
    pub reason: Reason,
    /// Its effective release, if any.
    pub release: Option<String>,
}

/// What a warning is about (ADR-0028 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningKind {
    /// A scheduled task without `size`, counted at the default size.
    Unsized,
    /// A scheduled story or epic not split yet, weighed by its provisional
    /// size or the default size.
    Provisional,
    /// An open task without story.
    WithoutStory,
    /// A story with open work and without epic.
    WithoutEpic,
    /// Open work without effective release.
    WithoutRelease,
    /// Open work whose release no applicable list declares.
    UndeclaredRelease,
    /// Work forecast, or closed, after its target, or the target of a story,
    /// an epic or a release missed.
    MissedTarget,
    /// A target later than the target of what contains it.
    ContradictoryTarget,
    /// Work caught in a dependency cycle, left unscheduled.
    DependencyCycle,
    /// A dependency on open work that is never scheduled, ignored.
    UnscheduledDependency,
}

/// One item a warning is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Flagged {
    /// `<node>#<id>`, or the release name for a release.
    pub reference: String,
    /// What is wrong with it, when the kind alone does not say.
    pub detail: Option<String>,
}

/// The items of one kind of warning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    /// What the warning is about.
    pub kind: WarningKind,
    /// How many items it concerns.
    pub count: usize,
    /// The items.
    pub items: Vec<Flagged>,
}

/// Computes the forecast of `input`.
pub fn compute(input: &Input) -> Forecast {
    let schedule = schedule::run(input);
    report::assemble(input, &schedule)
}

/// The forecast of `node` in `tree`, from `now`: on its carrier's blocks when
/// `calendar` gives it one, else in relative hours. `history` dates finished
/// work without `closed`.
pub fn build(
    tree: &Tree,
    calendar: Option<&Calendar>,
    history: Option<&History>,
    node: &NodePath,
    now: Timestamp,
) -> Forecast {
    compute(&gather(tree, calendar, history, node, now))
}

/// The forecast of `node` in the content repository at `root`, whose tree is
/// `tree`, from now: the personal calendar is read (SPEC.md §2.1), and git's
/// history approximates the closing dates the headers lack.
///
/// # Errors
///
/// Fails when the calendar exists but cannot be read.
pub fn load(root: &Path, tree: &Tree, node: &NodePath) -> Result<Forecast> {
    let personal = calendar::personal_root(root);
    let overview = calendar::overview(&personal, tree)?;
    let history = History::read(root);
    Ok(build(
        tree,
        overview.calendar.as_ref(),
        Some(&history),
        node,
        Timestamp::now(),
    ))
}

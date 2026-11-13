//! What the forecast engine reads: plain data, free of the tree and of the
//! file system, so that [`super::compute`] can run on hand-built fixtures.
//!
//! [`super::gather`] builds it from a loaded tree. Hours and capacity are
//! inputs, not computed here: velocity (ADR-0025) will feed other numbers
//! through the same fields without changing the engine.

use jiff::Timestamp;
use jiff::civil::Date;

use crate::calendar::{Block, DaysOff};
use crate::estimate::{Basis, Tester};
use crate::item::Kind;
use crate::tree::NodePath;

/// Everything the forecast of one node depends on.
#[derive(Debug, Clone)]
pub struct Input {
    /// The node the forecast runs on: the calendar's carrier of the node asked,
    /// else the node asked; every task below it shares one sequence.
    pub scope: NodePath,
    /// The node whose blocks give the capacity, when the calendar has one.
    pub carrier: Option<NodePath>,
    /// The releases that can be scheduled, in delivery order, parking ones
    /// left out.
    pub releases: Vec<ReleaseSpec>,
    /// The stories and epics the work belongs to, and the unsplit ones that
    /// are work themselves.
    pub groups: Vec<GroupSpec>,
    /// The pieces of work: tasks, and stories or epics not split yet.
    pub units: Vec<Unit>,
    /// The owner's hours on the carrier; `None` gives relative hours only.
    pub capacity: Option<Capacity>,
    /// The moment the forecast starts from.
    pub now: Timestamp,
}

/// One release of a declared list.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseSpec {
    /// The name items write in `release`.
    pub name: String,
    /// The node whose `node.toml` declares the list.
    pub declared_by: NodePath,
    /// The desired date of the `[[release]]` entry.
    pub target: Option<Date>,
}

/// A story or an epic, as a group of work.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupSpec {
    /// `<node>#<id>`.
    pub reference: String,
    /// `Kind::Story` or `Kind::Epic`.
    pub kind: Kind,
    /// The item's title.
    pub title: String,
    /// Its place among the groups: the order of node paths, then the
    /// manual order of ADR-0022.
    pub order: usize,
    /// Its desired date.
    pub target: Option<Date>,
    /// The epic of a story, as `<node>#<id>`.
    pub epic: Option<String>,
}

/// Where a piece of work is planned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Planned {
    /// A release of [`Input::releases`], by index.
    Release(usize),
    /// A parking release, never scheduled.
    Parking(String),
    /// A release no applicable list declares, which cannot be ordered.
    Undeclared(String),
    /// No release on the item, its story or its epic.
    Unplanned,
}

/// Where a piece of work stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Not started: `backlog`, `todo`, `blocked`, or an unknown status.
    Open,
    /// Being worked on: its whole remaining hours still count.
    InProgress,
    /// Handed to someone: no hours of the owner, a delay of its testing.
    Waiting,
    /// Finished, drawn at its closing date when one is known.
    Done {
        /// The day it closed, `None` when neither the header nor git says.
        closed: Option<Date>,
        /// Whether that day is approximated from git (ADR-0027).
        approximate: bool,
    },
    /// Abandoned: left out of everything.
    Dropped,
}

impl State {
    /// Tells whether the work still has to be done or delivered.
    pub fn is_open(self) -> bool {
        matches!(self, State::Open | State::InProgress | State::Waiting)
    }
}

/// One piece of work the scheduler places: a task, or a story or an epic
/// without tasks, weighed as one block.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    /// `<node>#<id>`.
    pub reference: String,
    /// The node holding the item.
    pub node: NodePath,
    /// `Kind::Task`, or `Kind::Story` and `Kind::Epic` for a block.
    pub kind: Kind,
    /// The item's title.
    pub title: String,
    /// Where it stands.
    pub state: State,
    /// Its effective release.
    pub release: Planned,
    /// Its story, as `<node>#<id>`: its own reference for a story block.
    pub story: Option<String>,
    /// Its epic, as `<node>#<id>`: its own reference for an epic block.
    pub epic: Option<String>,
    /// Its place among the work of its node, manual order of ADR-0022.
    pub order: usize,
    /// Hours of focused work left; for finished work, the hours it weighed.
    pub hours: f64,
    /// Hours of testing after it.
    pub test_hours: f64,
    /// Who tests it.
    pub tester: Tester,
    /// Where its hours come from.
    pub basis: Basis,
    /// The open work it waits for, as `<node>#<id>` of tasks, stories or
    /// epics: its own `depends_on`, its story's and its epic's.
    pub depends_on: Vec<String>,
    /// Its desired date.
    pub target: Option<Date>,
}

/// The owner's hours on the carrier node (ADR-0026).
#[derive(Debug, Clone, PartialEq)]
pub struct Capacity {
    /// The IANA time zone the blocks are written in.
    pub timezone: String,
    /// The share of the blocks that is focused work.
    pub focus: f64,
    /// The weekly blocks.
    pub blocks: Vec<Block>,
    /// Every day off that applies: the calendar's and the carrier's.
    pub days_off: Vec<DaysOff>,
}

//! `calendar.toml`: the owner's work calendar (ADR-0026, SPEC.md §2).
//!
//! The calendar says when the owner works on what: weekly blocks per node, a
//! `focus` coefficient per node for the share of those hours that is focused
//! work, days off for every node or for one, and the time zone the blocks are
//! written in. It describes a person, not a project, so it lives at the root of
//! the owner's **personal** content repository, versioned with it, and never in
//! a shared one: [`personal_root`] says which repository that is, and a
//! `calendar.toml` found anywhere else is reported by `doctor`, never read.
//!
//! The schema is declared once, in [`CALENDAR_KEYS`], and published by
//! `ariane schema --json`. [`check`] reads a file against it and returns the
//! valid part of the calendar with every problem found, so that `doctor`,
//! `ariane calendar edit` and the interface judge a file the same way. The
//! module never rewrites the file: `ariane calendar edit` saves exactly the
//! text the owner wrote, comments and layout included.
//!
//! A node's forecast runs on the nearest node, itself or an ancestor, whose
//! table carries blocks ([`Calendar::carrier`]); its descendants share those
//! hours. A missing file is not an error: the roadmap then has no dates.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize, Serializer};

use crate::cli::{CalendarArgs, CalendarCommand};
use crate::schema::{Requirement, is_date};
use crate::tree::{NodePath, Tree};

/// Name of the file, at the root of the personal content repository.
pub const FILE: &str = "calendar.toml";

/// The schema version this CLI reads and writes.
pub const VERSION: i64 = 1;

/// The days a block's `days` names, in the order of the week.
pub const WEEKDAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// The `focus` of a node whose table does not set one.
pub const DEFAULT_FOCUS: f64 = 1.0;

/// The time zone a new calendar is written in when the system's has no IANA name.
const FALLBACK_TIMEZONE: &str = "UTC";

/// One key of `calendar.toml`, as `ariane schema --json` publishes it.
#[derive(Debug, Clone, Copy)]
pub struct CalendarKey {
    /// The dotted name; `<path>` stands for a node path, quoted in TOML.
    pub name: &'static str,
    /// Its type in `ariane schema` output.
    pub value: &'static str,
    /// Whether it must be written.
    pub requirement: Requirement,
    /// What it means.
    pub doc: &'static str,
}

/// Every key of `calendar.toml`, in the order they are written.
pub const CALENDAR_KEYS: &[CalendarKey] = &[
    CalendarKey {
        name: "version",
        value: "integer",
        requirement: Requirement::Must,
        doc: "Schema version of the file, `1`. Changed only by `ariane migrate`.",
    },
    CalendarKey {
        name: "timezone",
        value: "string",
        requirement: Requirement::Must,
        doc: "The IANA time zone every block is written in, such as `Europe/Paris`.",
    },
    CalendarKey {
        name: "days_off",
        value: "list<string>",
        requirement: Requirement::May,
        doc: "Days off for every node: dates `YYYY-MM-DD` and inclusive ranges `YYYY-MM-DD..YYYY-MM-DD`.",
    },
    CalendarKey {
        name: "node.<path>.focus",
        value: "number",
        requirement: Requirement::May,
        doc: "The share of the node's blocks that is focused work, above 0 and at most 1; defaults to 1.",
    },
    CalendarKey {
        name: "node.<path>.blocks",
        value: "list<table>",
        requirement: Requirement::May,
        doc: "The node's weekly blocks of work. Its descendants share them unless they declare their own.",
    },
    CalendarKey {
        name: "node.<path>.blocks.days",
        value: "string",
        requirement: Requirement::Must,
        doc: "The days of a block: `mon` to `sun`, ranges and lists, such as `mon-fri`, `tue,thu` or `sat`.",
    },
    CalendarKey {
        name: "node.<path>.blocks.from",
        value: "string",
        requirement: Requirement::Must,
        doc: "When the block starts, `HH:MM`, in `timezone`.",
    },
    CalendarKey {
        name: "node.<path>.blocks.to",
        value: "string",
        requirement: Requirement::Must,
        doc: "When the block ends, `HH:MM` after `from`, `24:00` at the latest: a block never crosses midnight.",
    },
    CalendarKey {
        name: "node.<path>.days_off",
        value: "list<string>",
        requirement: Requirement::May,
        doc: "Days off of this node only, written like the global `days_off`.",
    },
];

/// The file as written, before its values are checked.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCalendar {
    version: i64,
    timezone: String,
    #[serde(default)]
    days_off: Vec<String>,
    #[serde(default)]
    node: BTreeMap<String, RawNode>,
}

/// One `[node."<path>"]` table as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNode {
    #[serde(default)]
    focus: Option<toml::Value>,
    #[serde(default)]
    blocks: Vec<RawBlock>,
    #[serde(default)]
    days_off: Vec<String>,
}

/// One block as written.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBlock {
    days: String,
    from: String,
    to: String,
}

/// A time of day, in minutes since midnight; `24:00` is 1440.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Clock(u16);

/// Written `HH:MM`, as in the file.
impl std::fmt::Display for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.0 / 60, self.0 % 60)
    }
}

impl Serialize for Clock {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// A set of days of the week, Monday first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Weekdays([bool; 7]);

impl Weekdays {
    /// How many days the set holds.
    pub fn count(self) -> usize {
        self.0.iter().filter(|day| **day).count()
    }

    /// The days both sets hold.
    pub fn common(self, other: Weekdays) -> Weekdays {
        let mut both = [false; 7];
        for (index, slot) in both.iter_mut().enumerate() {
            *slot = self.0[index] && other.0[index];
        }
        Weekdays(both)
    }

    /// The names of the days held, Monday first.
    pub fn names(self) -> Vec<&'static str> {
        WEEKDAYS
            .iter()
            .zip(self.0)
            .filter(|(_, held)| *held)
            .map(|(name, _)| *name)
            .collect()
    }
}

impl Serialize for Weekdays {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.names().serialize(serializer)
    }
}

/// One weekly block of work: on `weekdays`, from `from` to `to`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Block {
    /// The days as written, such as `mon-fri`.
    pub days: String,
    /// The days the block covers.
    pub weekdays: Weekdays,
    /// When it starts.
    pub from: Clock,
    /// When it ends, after `from`.
    pub to: Clock,
    /// Hours per week: its length times its number of days.
    pub hours: f64,
}

impl Block {
    /// Tells whether the two blocks share some time on some day, touching
    /// excluded: `09:00–12:00` and `12:00–18:00` do not overlap.
    fn overlaps(&self, other: &Block) -> bool {
        self.weekdays.common(other.weekdays).count() > 0
            && self.from < other.to
            && other.from < self.to
    }

    /// `mon-fri 09:00–18:00`.
    fn label(&self) -> String {
        format!("{} {}–{}", self.days, self.from, self.to)
    }
}

/// A day off, or an inclusive range of days off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaysOff {
    /// The first day off.
    pub from: jiff::civil::Date,
    /// The last day off, `from` itself for a single day.
    pub to: jiff::civil::Date,
}

/// Written as in the file: `2026-12-24`, or `2026-12-24..2027-01-02`.
impl std::fmt::Display for DaysOff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.from == self.to {
            write!(f, "{}", self.from)
        } else {
            write!(f, "{}..{}", self.from, self.to)
        }
    }
}

impl Serialize for DaysOff {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Range {
            from: String,
            to: String,
        }
        Range {
            from: self.from.to_string(),
            to: self.to.to_string(),
        }
        .serialize(serializer)
    }
}

/// The `[node."<path>"]` table of one node, its invalid parts left out.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeCalendar {
    /// The share of the blocks that is focused work, in `]0, 1]`.
    pub focus: f64,
    /// The weekly blocks.
    pub blocks: Vec<Block>,
    /// The node's own days off.
    pub days_off: Vec<DaysOff>,
}

impl NodeCalendar {
    /// Hours of blocks per week.
    pub fn weekly_hours(&self) -> f64 {
        round(self.blocks.iter().map(|block| block.hours).sum())
    }

    /// Hours of focused work per week: the blocks times `focus`.
    pub fn focused_hours(&self) -> f64 {
        round(self.weekly_hours() * self.focus)
    }
}

/// The calendar, its invalid parts left out.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Calendar {
    /// The schema version.
    pub version: i64,
    /// The IANA time zone of the blocks.
    pub timezone: String,
    /// Days off of every node.
    pub days_off: Vec<DaysOff>,
    /// The tables of the nodes, by node path.
    pub nodes: BTreeMap<NodePath, NodeCalendar>,
}

impl Calendar {
    /// The node whose blocks `node` works on: `node` itself, else its nearest
    /// ancestor whose table has blocks; `None` when none has.
    pub fn carrier(&self, node: &NodePath) -> Option<(&NodePath, &NodeCalendar)> {
        node.ancestors_from_root()
            .iter()
            .rev()
            .filter_map(|candidate| self.nodes.get_key_value(candidate))
            .find(|(_, entry)| !entry.blocks.is_empty())
    }
}

/// What a problem of the calendar is about, for `doctor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemKind {
    /// The file does not parse, or a value is out of its format or range.
    Invalid,
    /// Two blocks share time: an hour counts once.
    Overlap,
    /// A `[node."<path>"]` table names no node of the tree, perhaps a node
    /// renamed or moved: its blocks still count.
    UnknownNode,
    /// A node table without blocks: its `focus` and `days_off` are never used.
    Unused,
}

/// One problem found in the calendar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
    /// What it is about.
    pub kind: ProblemKind,
    /// The node table concerned, when there is one.
    pub node: Option<NodePath>,
    /// The other node of an overlap between two nodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<NodePath>,
    /// What is wrong, in one sentence.
    pub message: String,
}

impl Problem {
    /// Tells whether the problem makes the file invalid; an unused table or
    /// an unknown node does not.
    pub fn is_error(&self) -> bool {
        !matches!(self.kind, ProblemKind::Unused | ProblemKind::UnknownNode)
    }

    /// Tells whether the problem concerns `scope` or one of its descendants:
    /// either node of an overlap does.
    pub fn concerns(&self, scope: &NodePath) -> bool {
        [&self.node, &self.other]
            .into_iter()
            .flatten()
            .any(|node| node.is_within(scope))
    }
}

/// A calendar read against its schema: the valid part, and every problem.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Checked {
    /// The calendar without its invalid parts; `None` when the file does not parse.
    pub calendar: Option<Calendar>,
    /// Every problem found.
    pub problems: Vec<Problem>,
}

impl Checked {
    /// The problems that make the file invalid.
    pub fn errors(&self) -> impl Iterator<Item = &Problem> {
        self.problems.iter().filter(|problem| problem.is_error())
    }
}

/// Reads `text` as a calendar, `known` telling which node paths exist.
pub fn check(text: &str, known: impl Fn(&NodePath) -> bool) -> Checked {
    let raw: RawCalendar = match toml::from_str(text) {
        Ok(raw) => raw,
        Err(error) => {
            return Checked {
                calendar: None,
                problems: vec![invalid(None, error.message().trim().to_owned())],
            };
        }
    };
    let mut problems = Vec::new();
    if raw.version > VERSION {
        problems.push(invalid(
            None,
            format!(
                "`version = {}` is newer than this Ariane, which reads version {VERSION}: update the CLI",
                raw.version
            ),
        ));
    } else if raw.version < 1 {
        problems.push(invalid(
            None,
            format!("`version = {}` is not a version", raw.version),
        ));
    }
    if jiff::tz::TimeZone::get(&raw.timezone).is_err() {
        problems.push(invalid(
            None,
            format!(
                "`timezone = \"{}\"` is not an IANA time zone, such as `Europe/Paris`",
                raw.timezone
            ),
        ));
    }
    let days_off = dates(&raw.days_off, None, &mut problems);
    let mut nodes = BTreeMap::new();
    for (path, entry) in raw.node {
        let node = NodePath::new(path.as_str());
        if !known(&node) {
            problems.push(Problem {
                kind: ProblemKind::UnknownNode,
                node: Some(node.clone()),
                other: None,
                message: format!(
                    "`[node.\"{path}\"]` names no node of the tree: was the node renamed or moved? Its blocks still count, against overlaps too"
                ),
            });
        }
        let checked = node_calendar(&node, entry, &mut problems);
        nodes.insert(node, checked);
    }
    overlaps(&nodes, &mut problems);
    Checked {
        calendar: Some(Calendar {
            version: raw.version,
            timezone: raw.timezone,
            days_off,
            nodes,
        }),
        problems,
    }
}

/// Checks one node table, keeping its valid focus, blocks and days off.
fn node_calendar(node: &NodePath, entry: RawNode, problems: &mut Vec<Problem>) -> NodeCalendar {
    let here = format!("[node.\"{}\"]", node.as_str());
    let focus = match &entry.focus {
        None => DEFAULT_FOCUS,
        Some(value) => match focus(value) {
            Some(focus) => focus,
            None => {
                problems.push(invalid(
                    Some(node),
                    format!("{here}: `focus = {value}` is not a number above 0 and at most 1"),
                ));
                DEFAULT_FOCUS
            }
        },
    };
    let mut blocks = Vec::new();
    for raw in &entry.blocks {
        match block(raw) {
            Ok(block) => blocks.push(block),
            Err(reason) => problems.push(invalid(Some(node), format!("{here}: {reason}"))),
        }
    }
    let days_off = dates(&entry.days_off, Some(node), problems);
    if entry.blocks.is_empty() {
        problems.push(Problem {
            kind: ProblemKind::Unused,
            node: Some(node.clone()),
            other: None,
            message: format!(
                "{here} has no blocks: its focus and days off are never used, the node works on the blocks of its nearest ancestor that has some"
            ),
        });
    }
    NodeCalendar {
        focus,
        blocks,
        days_off,
    }
}

/// `focus` when `value` is a number in `]0, 1]`.
fn focus(value: &toml::Value) -> Option<f64> {
    let number = match value {
        toml::Value::Integer(integer) => *integer as f64,
        toml::Value::Float(float) => *float,
        _ => return None,
    };
    (number > 0.0 && number <= 1.0).then_some(number)
}

/// Reads one block, or says why it is invalid.
fn block(raw: &RawBlock) -> Result<Block, String> {
    let weekdays = weekdays(&raw.days)?;
    let from =
        clock(&raw.from, false).map_err(|reason| format!("`from = \"{}\"` {reason}", raw.from))?;
    let to = clock(&raw.to, true).map_err(|reason| format!("`to = \"{}\"` {reason}", raw.to))?;
    if from >= to {
        return Err(format!(
            "the block `{} {}–{}` ends before it starts; a block never crosses midnight, split it in two",
            raw.days, raw.from, raw.to
        ));
    }
    let hours = f64::from(to.0 - from.0) / 60.0 * weekdays.count() as f64;
    Ok(Block {
        days: raw.days.clone(),
        weekdays,
        from,
        to,
        hours: round(hours),
    })
}

/// Reads `mon-fri`, `tue,thu`, `sat` or `mon,wed-fri`.
fn weekdays(spec: &str) -> Result<Weekdays, String> {
    let wrong = |reason: String| {
        format!(
            "`days = \"{spec}\"` {reason}; write days `mon` to `sun`, such as `mon-fri` or `tue,thu`"
        )
    };
    let index = |name: &str| {
        WEEKDAYS
            .iter()
            .position(|day| *day == name)
            .ok_or_else(|| wrong(format!("names `{name}`, which is not a day")))
    };
    let mut set = [false; 7];
    for part in spec.split(',').map(str::trim) {
        if part.is_empty() {
            return Err(wrong("has an empty part".to_owned()));
        }
        let (first, last) = match part.split_once('-') {
            Some((first, last)) => (index(first.trim())?, index(last.trim())?),
            None => {
                let day = index(part)?;
                (day, day)
            }
        };
        if first > last {
            return Err(wrong(format!(
                "has the range `{part}` going backwards; the week starts on `mon`"
            )));
        }
        for slot in &mut set[first..=last] {
            *slot = true;
        }
    }
    Ok(Weekdays(set))
}

/// Reads `HH:MM`; `24:00` only when `end` is set.
fn clock(text: &str, end: bool) -> Result<Clock, &'static str> {
    let bytes = text.as_bytes();
    let well_formed = bytes.len() == 5 && bytes[2] == b':' && text.is_char_boundary(2);
    let parsed = well_formed
        .then(|| (text[..2].parse::<u16>().ok(), text[3..].parse::<u16>().ok()))
        .and_then(|(hours, minutes)| hours.zip(minutes));
    match parsed {
        Some((24, 0)) if end => Ok(Clock(24 * 60)),
        Some((hours, minutes)) if hours < 24 && minutes < 60 => Ok(Clock(hours * 60 + minutes)),
        _ if end => Err("is not an hour `HH:MM`, from `00:00` to `24:00`"),
        _ => Err("is not an hour `HH:MM`, from `00:00` to `23:59`"),
    }
}

/// Reads a list of days off, reporting each invalid entry.
fn dates(entries: &[String], node: Option<&NodePath>, problems: &mut Vec<Problem>) -> Vec<DaysOff> {
    let place = node.map_or_else(String::new, |node| {
        format!("[node.\"{}\"]: ", node.as_str())
    });
    entries
        .iter()
        .filter_map(|entry| match days_off(entry) {
            Ok(days) => Some(days),
            Err(reason) => {
                problems.push(invalid(node, format!("{place}days off `{entry}` {reason}")));
                None
            }
        })
        .collect()
}

/// Reads `YYYY-MM-DD` or `YYYY-MM-DD..YYYY-MM-DD`.
fn days_off(entry: &str) -> Result<DaysOff, &'static str> {
    let date = |text: &str| {
        let text = text.trim();
        is_date(text)
            .then(|| text.parse::<jiff::civil::Date>().ok())
            .flatten()
    };
    let (from, to) = match entry.split_once("..") {
        Some((from, to)) => (date(from), date(to)),
        None => (date(entry), date(entry)),
    };
    match (from, to) {
        (Some(from), Some(to)) if from <= to => Ok(DaysOff { from, to }),
        (Some(_), Some(_)) => Err("ends before it starts"),
        _ => Err("is neither a date `YYYY-MM-DD` nor a range `YYYY-MM-DD..YYYY-MM-DD`"),
    }
}

/// Reports every pair of blocks that share time: of two nodes, an hour counted
/// twice; of one node, blocks that may touch but not overlap.
fn overlaps(nodes: &BTreeMap<NodePath, NodeCalendar>, problems: &mut Vec<Problem>) {
    let blocks: Vec<(&NodePath, &Block)> = nodes
        .iter()
        .flat_map(|(node, entry)| entry.blocks.iter().map(move |block| (node, block)))
        .collect();
    for (index, (node, block)) in blocks.iter().enumerate() {
        for (other_node, other) in &blocks[index + 1..] {
            if !block.overlaps(other) {
                continue;
            }
            let days = block.weekdays.common(other.weekdays).names().join(", ");
            let message = if node == other_node {
                format!(
                    "two blocks of `{}` overlap on {days}: `{}` and `{}`; blocks of one node may touch, not overlap",
                    node.as_str(),
                    block.label(),
                    other.label()
                )
            } else {
                format!(
                    "the blocks `{}` of `{}` and `{}` of `{}` overlap on {days}: an hour counts once",
                    block.label(),
                    node.as_str(),
                    other.label(),
                    other_node.as_str()
                )
            };
            problems.push(Problem {
                kind: ProblemKind::Overlap,
                node: Some((*node).clone()),
                other: (node != other_node).then(|| (*other_node).clone()),
                message,
            });
        }
    }
}

/// An [`ProblemKind::Invalid`] problem.
fn invalid(node: Option<&NodePath>, message: String) -> Problem {
    Problem {
        kind: ProblemKind::Invalid,
        node: node.cloned(),
        other: None,
        message,
    }
}

/// `value` rounded to the hundredth.
fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// The calendar file of the content repository at `root`.
pub fn path(root: &Path) -> PathBuf {
    root.join(FILE)
}

/// The personal content repository, whose `calendar.toml` is the only one read.
///
/// It is the repository the `repo` key of `config.toml` names (`ariane config
/// get repo`, ADR-0013): the one a machine is set up with. When that key is
/// not set, or names no content repository, the repository being worked on,
/// `root`, is the personal one, which is the case of every content repository
/// until shared ones are mounted (ADR-0021).
pub fn personal_root(root: &Path) -> PathBuf {
    let configured = crate::config::load(&crate::config::file())
        .ok()
        .and_then(|loaded| loaded.config.repo_path());
    personal_of(root, configured.as_deref())
}

/// [`personal_root`] with the configured repository given.
pub fn personal_of(root: &Path, configured: Option<&Path>) -> PathBuf {
    let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    configured
        .map(canonical)
        .filter(|path| crate::repo::is_content_repository(path))
        .unwrap_or_else(|| canonical(root))
}

/// Tells whether the content repository at `root` is the personal one.
pub fn is_personal(root: &Path, personal: &Path) -> bool {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf()) == personal
}

/// What `ariane calendar --json` without a node, and `GET /api/calendar`, give.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Overview {
    /// The file read.
    pub file: PathBuf,
    /// Whether it exists; without it the roadmap has no dates.
    pub present: bool,
    /// The calendar, its invalid parts left out; `None` when absent or unparsable.
    pub calendar: Option<Calendar>,
    /// Every problem found.
    pub problems: Vec<Problem>,
}

/// Reads the calendar of the personal repository `personal`, whose nodes
/// `tree` holds.
///
/// # Errors
///
/// Fails when the file exists but cannot be read.
pub fn overview(personal: &Path, tree: &Tree) -> Result<Overview> {
    let file = path(personal);
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Overview {
                file,
                present: false,
                calendar: None,
                problems: Vec::new(),
            });
        }
        Err(error) => return Err(error).with_context(|| format!("reading {}", file.display())),
    };
    let checked = check(&text, |node| known_node(tree, node));
    Ok(Overview {
        file,
        present: true,
        calendar: checked.calendar,
        problems: checked.problems,
    })
}

/// Tells whether `node` is a node of `tree`.
fn known_node(tree: &Tree, node: &NodePath) -> bool {
    tree.nodes.iter().any(|candidate| &candidate.path == node)
}

/// What the calendar gives one node: `ariane calendar <node> --json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Resolved {
    /// The node asked about.
    pub node: NodePath,
    /// The node whose blocks it works on: itself or an ancestor; `None` when
    /// neither has blocks, and the node has no dates.
    pub carrier: Option<NodePath>,
    /// The time zone of the blocks.
    pub timezone: String,
    /// The carrier's focus.
    pub focus: Option<f64>,
    /// The carrier's blocks.
    pub blocks: Vec<Block>,
    /// The days off of every node.
    pub days_off: Vec<DaysOff>,
    /// The carrier's own days off.
    pub node_days_off: Vec<DaysOff>,
    /// Hours of blocks per week.
    pub weekly_hours: f64,
    /// Hours of focused work per week.
    pub focused_hours: f64,
}

/// What `calendar` gives `node`.
pub fn resolve(calendar: &Calendar, node: &NodePath) -> Resolved {
    let carrier = calendar.carrier(node);
    Resolved {
        node: node.clone(),
        carrier: carrier.map(|(path, _)| path.clone()),
        timezone: calendar.timezone.clone(),
        focus: carrier.map(|(_, entry)| entry.focus),
        blocks: carrier.map_or_else(Vec::new, |(_, entry)| entry.blocks.clone()),
        days_off: calendar.days_off.clone(),
        node_days_off: carrier.map_or_else(Vec::new, |(_, entry)| entry.days_off.clone()),
        weekly_hours: carrier.map_or(0.0, |(_, entry)| entry.weekly_hours()),
        focused_hours: carrier.map_or(0.0, |(_, entry)| entry.focused_hours()),
    }
}

/// Runs `ariane calendar`: prints what applies to a node, or edits the file.
///
/// The calendar read is the personal repository's ([`personal_root`]); a
/// `calendar.toml` in `root`, when `root` is another repository, is ignored
/// and said so.
///
/// # Errors
///
/// Fails when the file cannot be read or written, or the editor cannot run.
pub fn run(
    root: &Path,
    tree: &Tree,
    args: &CalendarArgs,
    here: Option<&NodePath>,
) -> Result<ExitCode> {
    let personal = personal_root(root);
    if !is_personal(root, &personal) && path(root).exists() {
        eprintln!(
            "ariane calendar: {} is ignored: only the personal content repository's calendar is read, {}",
            path(root).display(),
            path(&personal).display()
        );
    }
    let loaded;
    let tree = if is_personal(root, &personal) {
        tree
    } else {
        loaded = Tree::load(&personal)?;
        &loaded
    };
    if let Some(CalendarCommand::Edit) = args.command {
        return edit(&personal, tree, &crate::config::command::editor());
    }
    let node = args
        .node
        .as_deref()
        .map(NodePath::new)
        .or_else(|| here.filter(|node| !node.is_home()).cloned());
    show(&overview(&personal, tree)?, node.as_ref(), args.json)
}

/// Prints the calendar, or what applies to `node`.
///
/// # Errors
///
/// Fails only if the JSON cannot be serialized.
fn show(overview: &Overview, node: Option<&NodePath>, json: bool) -> Result<ExitCode> {
    for problem in &overview.problems {
        let level = if problem.is_error() {
            "error"
        } else {
            "warning"
        };
        eprintln!("ariane calendar: {level}: {}", problem.message);
    }
    let Some(calendar) = &overview.calendar else {
        if json {
            println!("{}", serde_json::to_string_pretty(overview)?);
        } else if overview.present {
            println!(
                "{} is invalid; fix it with `ariane calendar edit`",
                overview.file.display()
            );
        } else {
            println!(
                "No calendar: {} does not exist, so the roadmap has no dates.\n`ariane calendar edit` creates it.",
                overview.file.display()
            );
        }
        return Ok(ExitCode::SUCCESS);
    };
    match (node, json) {
        (Some(node), true) => println!(
            "{}",
            serde_json::to_string_pretty(&resolve(calendar, node))?
        ),
        (None, true) => println!("{}", serde_json::to_string_pretty(overview)?),
        (Some(node), false) => print!("{}", describe(&resolve(calendar, node))),
        (None, false) => {
            println!("{}  ({})", overview.file.display(), calendar.timezone);
            let carriers: Vec<&NodePath> = calendar
                .nodes
                .iter()
                .filter(|(_, entry)| !entry.blocks.is_empty())
                .map(|(node, _)| node)
                .collect();
            if carriers.is_empty() {
                println!("\nNo node has blocks yet.");
                print!("{}", days_off_lines(&calendar.days_off, &[], None));
            }
            for node in carriers {
                println!();
                print!("{}", describe(&resolve(calendar, node)));
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The lines `ariane calendar <node>` prints.
fn describe(resolved: &Resolved) -> String {
    let mut text = format!("node        {}\n", resolved.node);
    let Some(carrier) = &resolved.carrier else {
        text.push_str(&format!(
            "blocks      none on {} or its ancestors: it has no dates in the roadmap\n",
            resolved.node
        ));
        text.push_str(&days_off_lines(&resolved.days_off, &[], None));
        return text;
    };
    if carrier == &resolved.node {
        text.push_str("blocks      its own\n");
    } else {
        text.push_str(&format!(
            "blocks      inherited from {carrier}, shared with its other descendants\n"
        ));
    }
    for block in &resolved.blocks {
        text.push_str(&format!(
            "  {:<12} {}–{}  {} h\n",
            block.days, block.from, block.to, block.hours
        ));
    }
    let focus = resolved.focus.unwrap_or(DEFAULT_FOCUS);
    text.push_str(&format!("focus       {focus}  ({})\n", resolved.timezone));
    text.push_str(&days_off_lines(
        &resolved.days_off,
        &resolved.node_days_off,
        Some(carrier),
    ));
    text.push_str(&format!(
        "per week    {} h of blocks × focus {focus} = {} h of focused work\n",
        resolved.weekly_hours, resolved.focused_hours
    ));
    text
}

/// The days-off lines: every node's, then the carrier's own.
fn days_off_lines(global: &[DaysOff], own: &[DaysOff], carrier: Option<&NodePath>) -> String {
    let entries: Vec<String> = global
        .iter()
        .map(|days| format!("{days} (every node)"))
        .chain(own.iter().map(|days| {
            format!(
                "{days} ({})",
                carrier.map_or_else(String::new, ToString::to_string)
            )
        }))
        .collect();
    if entries.is_empty() {
        return "days off    none\n".to_owned();
    }
    let mut text = String::new();
    for (index, entry) in entries.iter().enumerate() {
        let label = if index == 0 { "days off" } else { "" };
        text.push_str(&format!("{label:<12}{entry}\n"));
    }
    text
}

/// What a new calendar starts with: the version and the system's time zone.
pub fn template() -> String {
    let system = jiff::tz::TimeZone::system();
    let timezone = system.iana_name().unwrap_or(FALLBACK_TIMEZONE);
    format!("version = {VERSION}\ntimezone = \"{timezone}\"\n")
}

/// The prefix of the lines `ariane calendar edit` adds above an invalid draft.
const BANNER: &str = "#!";

/// How `ariane calendar edit` ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edited {
    /// The file was written.
    Saved,
    /// Nothing changed, or the owner emptied the draft: the file is as it was.
    Abandoned,
}

/// Runs `ariane calendar edit` with `editor` on the calendar of the personal
/// repository `personal`, whose nodes `tree` holds.
///
/// # Errors
///
/// As [`edit_with`].
fn edit(personal: &Path, tree: &Tree, editor: &str) -> Result<ExitCode> {
    match edit_with(personal, tree, editor)? {
        Edited::Saved => println!("{} saved", path(personal).display()),
        Edited::Abandoned => println!("{} left as it was", path(personal).display()),
    }
    Ok(ExitCode::SUCCESS)
}

/// Opens a draft of the calendar in `editor`, from the file or, when it is
/// missing, from [`template`]; validates the draft when the editor closes.
///
/// A valid draft is saved as written. An invalid one is never saved: the
/// editor opens again on the owner's draft, with the errors in `#!` lines on
/// top, which are taken out again before the draft is read. A draft left
/// unchanged, or emptied, abandons the edit and leaves the file as it was.
///
/// # Errors
///
/// Fails when the editor cannot start or exits with an error, and when a
/// file cannot be read or written; the calendar is then left as it was.
pub fn edit_with(personal: &Path, tree: &Tree, editor: &str) -> Result<Edited> {
    let file = path(personal);
    let original = match std::fs::read_to_string(&file) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).with_context(|| format!("reading {}", file.display())),
    };
    let start = original.clone().unwrap_or_else(template);
    let scratch = scratch_dir();
    std::fs::create_dir_all(&scratch).with_context(|| format!("creating {}", scratch.display()))?;
    let draft_file = scratch.join(FILE);
    let outcome = edit_loop(&draft_file, &file, &start, tree, editor);
    let _ = std::fs::remove_dir_all(&scratch);
    outcome
}

/// A directory of its own for one edit's draft, under the system's temporary
/// directory.
fn scratch_dir() -> PathBuf {
    static EDITS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let edit = EDITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("ariane-calendar-{}-{edit}", std::process::id()))
}

/// The rounds of [`edit_with`] on the draft `draft_file`, until it is saved
/// into `file` or abandoned.
///
/// # Errors
///
/// As [`edit_with`].
fn edit_loop(
    draft_file: &Path,
    file: &Path,
    start: &str,
    tree: &Tree,
    editor: &str,
) -> Result<Edited> {
    let mut draft = start.to_owned();
    loop {
        std::fs::write(draft_file, &draft)
            .with_context(|| format!("writing {}", draft_file.display()))?;
        let status = crate::config::command::open_editor(editor, draft_file)?;
        if !status.success() {
            bail!("the editor `{editor}` exited with {status}; {FILE} left as it was");
        }
        let written = std::fs::read_to_string(draft_file)
            .with_context(|| format!("reading {}", draft_file.display()))?;
        let text = without_banner(&written);
        if text.trim().is_empty() || text == start {
            return Ok(Edited::Abandoned);
        }
        let checked = check(&text, |node| known_node(tree, node));
        let errors: Vec<&Problem> = checked.errors().collect();
        if errors.is_empty() {
            for warning in checked
                .problems
                .iter()
                .filter(|problem| !problem.is_error())
            {
                eprintln!("ariane calendar: warning: {}", warning.message);
            }
            std::fs::write(file, &text).with_context(|| format!("writing {}", file.display()))?;
            return Ok(Edited::Saved);
        }
        for error in &errors {
            eprintln!("ariane calendar: error: {}", error.message);
        }
        draft = with_banner(&text, &errors);
    }
}

/// `text` under `#!` lines that list `errors` and say how to abandon.
fn with_banner(text: &str, errors: &[&Problem]) -> String {
    let mut banner = format!(
        "{BANNER} This calendar is invalid and was not saved. Fix it and save, or empty the file to abandon.\n{BANNER} These lines are removed when the file is read.\n"
    );
    for error in errors {
        banner.push_str(&format!("{BANNER} error: {}\n", error.message));
    }
    format!("{banner}\n{text}")
}

/// `text` without the `#!` lines on top, and the blank line under them.
fn without_banner(text: &str) -> String {
    let mut rest = text;
    let mut stripped = false;
    while rest.starts_with(BANNER) {
        stripped = true;
        rest = rest.split_once('\n').map_or("", |(_, after)| after);
    }
    if stripped {
        rest = rest.strip_prefix('\n').unwrap_or(rest);
    }
    rest.to_owned()
}

/// Unit tests of the calendar: schema, checks, resolution and edition.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// The calendar of ADR-0026, on a tree holding its nodes.
    const SAMPLE: &str = r#"# my week
version = 1
timezone = "Europe/Paris"
days_off = ["2026-12-24..2027-01-02", "2027-05-01"]

[node."cmc"]
focus = 0.6
blocks = [{ days = "mon-fri", from = "09:00", to = "18:00" }]

[node."slashome/pharmakon"]
focus = 1
days_off = ["2026-08-03..2026-08-14"]
blocks = [
  { days = "tue,thu", from = "20:00", to = "23:00" },
  { days = "sat", from = "10:00", to = "13:00" },
]
"#;

    /// Every node path is known.
    fn any(_: &NodePath) -> bool {
        true
    }

    /// The problems `check` finds in `text`, every node known.
    fn problems(text: &str) -> Vec<Problem> {
        check(text, any).problems
    }

    /// The single problem `check` finds in `text`.
    fn only(text: &str) -> Problem {
        let mut found = problems(text);
        assert_eq!(found.len(), 1, "{found:?}");
        found.remove(0)
    }

    /// A version-1 calendar in Paris, followed by `rest`.
    fn with(rest: &str) -> String {
        format!("version = 1\ntimezone = \"Europe/Paris\"\n{rest}")
    }

    #[test]
    fn the_sample_of_the_adr_is_valid_and_priced_per_week() {
        let checked = check(SAMPLE, any);
        assert_eq!(checked.problems, vec![]);
        let calendar = checked.calendar.unwrap();
        assert_eq!(calendar.timezone, "Europe/Paris");
        assert_eq!(calendar.days_off.len(), 2);
        assert_eq!(calendar.days_off[1].to_string(), "2027-05-01");
        let cmc = &calendar.nodes[&NodePath::new("cmc")];
        assert_eq!((cmc.weekly_hours(), cmc.focus), (45.0, 0.6));
        assert_eq!(cmc.focused_hours(), 27.0);
        let pharmakon = &calendar.nodes[&NodePath::new("slashome/pharmakon")];
        assert_eq!(pharmakon.weekly_hours(), 9.0);
        assert_eq!(pharmakon.blocks[0].weekdays.names(), ["tue", "thu"]);
        assert_eq!(pharmakon.days_off[0].to_string(), "2026-08-03..2026-08-14");
    }

    #[test]
    fn a_node_works_on_its_own_blocks_else_on_its_nearest_ancestor_s() {
        let calendar = check(SAMPLE, any).calendar.unwrap();
        let child = resolve(&calendar, &NodePath::new("cmc/cmc-lib"));
        assert_eq!(child.carrier, Some(NodePath::new("cmc")));
        assert_eq!((child.weekly_hours, child.focused_hours), (45.0, 27.0));
        let own = resolve(&calendar, &NodePath::new("slashome/pharmakon"));
        assert_eq!(own.carrier, Some(NodePath::new("slashome/pharmakon")));
        assert_eq!(own.node_days_off.len(), 1);
        let none = resolve(&calendar, &NodePath::new("slashome"));
        assert_eq!((none.carrier, none.focused_hours), (None, 0.0));
        assert_eq!(none.days_off.len(), 2);
    }

    #[test]
    fn a_table_without_blocks_is_skipped_for_the_ancestor_s_and_reported_unused() {
        let text = with(
            "[node.\"cmc\"]\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"10:00\" }]\n[node.\"cmc/lib\"]\nfocus = 0.5\n",
        );
        let checked = check(&text, any);
        assert_eq!(checked.problems.len(), 1);
        assert_eq!(checked.problems[0].kind, ProblemKind::Unused);
        assert_eq!(checked.errors().count(), 0);
        let calendar = checked.calendar.unwrap();
        let resolved = resolve(&calendar, &NodePath::new("cmc/lib/x"));
        assert_eq!(resolved.carrier, Some(NodePath::new("cmc")));
        assert_eq!(resolved.focus, Some(1.0));
    }

    #[test]
    fn blocks_of_two_nodes_that_overlap_are_an_error_and_touching_is_not() {
        let overlap = only(&with(
            "[node.\"a\"]\nblocks = [{ days = \"mon-fri\", from = \"09:00\", to = \"18:00\" }]\n[node.\"b\"]\nblocks = [{ days = \"fri-sun\", from = \"17:00\", to = \"20:00\" }]\n",
        ));
        assert_eq!(overlap.kind, ProblemKind::Overlap);
        assert!(overlap.message.contains("on fri"), "{}", overlap.message);
        assert!(overlap.message.contains("an hour counts once"));
        assert_eq!(
            problems(&with(
                "[node.\"a\"]\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"12:00\" }]\n[node.\"b\"]\nblocks = [{ days = \"mon\", from = \"12:00\", to = \"24:00\" }, { days = \"tue\", from = \"09:00\", to = \"12:00\" }]\n",
            )),
            vec![]
        );
    }

    #[test]
    fn blocks_of_one_node_may_touch_but_not_overlap() {
        let touching = with(
            "[node.\"a\"]\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"12:00\" }, { days = \"mon\", from = \"12:00\", to = \"13:00\" }]\n",
        );
        assert_eq!(problems(&touching), vec![]);
        let overlap = only(&with(
            "[node.\"a\"]\nblocks = [{ days = \"mon,wed\", from = \"09:00\", to = \"12:00\" }, { days = \"wed\", from = \"11:00\", to = \"13:00\" }]\n",
        ));
        assert_eq!(overlap.kind, ProblemKind::Overlap);
        assert!(overlap.message.contains("may touch, not overlap"));
    }

    #[test]
    fn focus_must_be_above_zero_and_at_most_one() {
        for focus in ["0", "1.5", "-0.2", "\"high\""] {
            let problem = only(&with(&format!(
                "[node.\"a\"]\nfocus = {focus}\nblocks = [{{ days = \"mon\", from = \"09:00\", to = \"10:00\" }}]\n"
            )));
            assert_eq!(problem.kind, ProblemKind::Invalid, "{focus}");
            assert!(problem.message.contains("focus"), "{}", problem.message);
        }
    }

    #[test]
    fn hours_days_and_dates_follow_their_formats() {
        let block = |days: &str, from: &str, to: &str| {
            with(&format!(
                "[node.\"a\"]\nblocks = [{{ days = \"{days}\", from = \"{from}\", to = \"{to}\" }}]\n"
            ))
        };
        for (days, from, to) in [
            ("mon", "9:00", "10:00"),
            ("mon", "09:60", "10:00"),
            ("mon", "24:00", "24:00"),
            ("mon", "09:00", "24:30"),
            ("monday", "09:00", "10:00"),
            ("fri-mon", "09:00", "10:00"),
            ("mon,", "09:00", "10:00"),
            ("mon", "10:00", "10:00"),
            ("mon", "20:00", "02:00"),
        ] {
            let problem = only(&block(days, from, to));
            assert_eq!(problem.kind, ProblemKind::Invalid, "{days} {from} {to}");
        }
        assert_eq!(problems(&block("mon, wed-fri", "00:00", "24:00")), vec![]);
        for date in [
            "2026-13-01",
            "2026-12-24..2026-12-01",
            "24/12/2026",
            "2026-12-24..",
        ] {
            let problem = only(&with(&format!("days_off = [\"{date}\"]\n")));
            assert!(problem.message.contains(date), "{}", problem.message);
        }
    }

    #[test]
    fn the_time_zone_must_be_an_iana_name() {
        let problem = only("version = 1\ntimezone = \"Mars/Olympus\"\n");
        assert!(problem.message.contains("IANA"));
        assert_eq!(
            problems("version = 1\ntimezone = \"America/New_York\"\n"),
            vec![]
        );
    }

    #[test]
    fn an_unknown_node_a_wrong_version_and_an_unknown_key_are_reported() {
        let checked = check(
            &with(
                "[node.\"ghost\"]\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"10:00\" }]\n",
            ),
            |node| node.as_str() == "real",
        );
        assert_eq!(checked.problems[0].kind, ProblemKind::UnknownNode);
        assert_eq!(checked.errors().count(), 0);
        let ghost = &checked.calendar.unwrap().nodes[&NodePath::new("ghost")];
        assert_eq!(ghost.weekly_hours(), 1.0);
        assert!(
            only("version = 2\ntimezone = \"UTC\"\n")
                .message
                .contains("update the CLI")
        );
        let unknown = only(&with("hours = 3\n"));
        assert!(unknown.message.contains("hours"), "{}", unknown.message);
        assert_eq!(check("version = 1\n", any).calendar, None);
    }

    #[test]
    fn the_personal_repository_is_the_configured_one_else_the_current_one() {
        let personal = Repo::new();
        let other = Repo::new();
        let canonical = |repo: &Repo| repo.root().canonicalize().unwrap();
        assert_eq!(personal_of(other.root(), None), canonical(&other));
        assert_eq!(
            personal_of(other.root(), Some(personal.root())),
            canonical(&personal)
        );
        assert_eq!(
            personal_of(other.root(), Some(Path::new("/nowhere/at/all"))),
            canonical(&other)
        );
        assert!(is_personal(personal.root(), &canonical(&personal)));
        assert!(!is_personal(other.root(), &canonical(&personal)));
    }

    #[test]
    fn a_missing_file_is_no_calendar_and_no_problem() {
        let repo = Repo::new();
        let tree = Tree::load(repo.root()).unwrap();
        let overview = overview(repo.root(), &tree).unwrap();
        assert!(!overview.present);
        assert_eq!((overview.calendar, overview.problems), (None, vec![]));
    }

    #[test]
    fn the_description_says_where_inherited_blocks_come_from() {
        let calendar = check(SAMPLE, any).calendar.unwrap();
        let text = describe(&resolve(&calendar, &NodePath::new("cmc/cmc-lib")));
        assert!(text.contains("inherited from cmc"), "{text}");
        assert!(text.contains("mon-fri"), "{text}");
        assert!(
            text.contains("45 h of blocks × focus 0.6 = 27 h of focused work"),
            "{text}"
        );
        assert!(
            text.contains("2026-12-24..2027-01-02 (every node)"),
            "{text}"
        );
        let none = describe(&resolve(&calendar, &NodePath::new("atlas")));
        assert!(none.contains("no dates"), "{none}");
    }

    #[test]
    fn the_template_is_a_valid_calendar_in_the_system_time_zone() {
        let text = template();
        assert!(text.starts_with("version = 1\ntimezone = \""), "{text}");
        assert_eq!(problems(&text), vec![]);
    }

    #[test]
    fn the_banner_goes_on_top_of_an_invalid_draft_and_comes_off_again() {
        let error = invalid(None, "bad".to_owned());
        let draft = with_banner("version = 1\n# mine\n", &[&error]);
        assert!(draft.starts_with("#! This calendar is invalid"), "{draft}");
        assert!(draft.contains("#! error: bad\n"), "{draft}");
        assert_eq!(without_banner(&draft), "version = 1\n# mine\n");
        assert_eq!(
            without_banner("# mine\nversion = 1\n"),
            "# mine\nversion = 1\n"
        );
    }

    /// A repository with the nodes `a` and `b`, and an editor script `editor`
    /// that runs `body` with the draft as `$1` and its round, from 1, as `$n`.
    #[cfg(unix)]
    fn editing(body: &str) -> (Repo, Tree, String) {
        use std::os::unix::fs::PermissionsExt;
        let repo = Repo::new();
        repo.write("a/node.toml", "").write("b/node.toml", "");
        let tree = Tree::load(repo.root()).unwrap();
        let script = repo.root().join("editor.sh");
        let rounds = repo.root().join("rounds");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\necho x >> '{}'\nn=$(wc -l < '{}' | tr -d ' ')\n{body}\n",
                rounds.display(),
                rounds.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let editor = script.display().to_string();
        (repo, tree, editor)
    }

    /// How many times the editor of [`editing`] ran.
    #[cfg(unix)]
    fn rounds(repo: &Repo) -> usize {
        std::fs::read_to_string(repo.root().join("rounds"))
            .unwrap_or_default()
            .lines()
            .count()
    }

    #[cfg(unix)]
    #[test]
    fn a_valid_edit_is_saved_as_written_and_a_missing_file_starts_from_the_template() {
        let (repo, tree, editor) = editing(
            "printf '\\n# evenings\\n[node.\"a\"]\\nblocks = [{ days = \"tue\", from = \"20:00\", to = \"23:00\" }]\\n' >> \"$1\"",
        );
        assert_eq!(
            edit_with(repo.root(), &tree, &editor).unwrap(),
            Edited::Saved
        );
        let saved = std::fs::read_to_string(path(repo.root())).unwrap();
        assert!(saved.starts_with(&template()), "{saved}");
        assert!(saved.contains("# evenings\n[node.\"a\"]"), "{saved}");
        assert_eq!(rounds(&repo), 1);
    }

    #[cfg(unix)]
    #[test]
    fn an_invalid_edit_is_never_saved_and_reopens_on_the_draft_with_its_errors() {
        let (repo, tree, editor) = editing(
            "if [ \"$n\" = 1 ]; then printf '[node.\"a\"]\\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"12:00\" }]\\n[node.\"b\"]\\nblocks = [{ days = \"mon\", from = \"11:00\", to = \"13:00\" }]\\n' >> \"$1\"; \
             else grep -q 'error: the blocks' \"$1\" && grep -q 'from = \"11:00\"' \"$1\" && sed 's/11:00/12:00/' \"$1\" > \"$1.new\" && mv \"$1.new\" \"$1\"; fi",
        );
        let original = "# keep\nversion = 1\ntimezone = \"UTC\"\n";
        std::fs::write(path(repo.root()), original).unwrap();
        assert_eq!(
            edit_with(repo.root(), &tree, &editor).unwrap(),
            Edited::Saved
        );
        assert_eq!(rounds(&repo), 2);
        let saved = std::fs::read_to_string(path(repo.root())).unwrap();
        assert!(saved.starts_with(original), "{saved}");
        assert!(!saved.contains("#!"), "{saved}");
        assert!(
            saved.contains("from = \"12:00\", to = \"13:00\""),
            "{saved}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_unknown_node_does_not_stop_an_edit_from_being_saved() {
        let (repo, tree, editor) = editing(
            "printf '[node.\"renamed\"]\nblocks = [{ days = \"tue\", from = \"20:00\", to = \"23:00\" }]\n' >> \"$1\"",
        );
        assert_eq!(
            edit_with(repo.root(), &tree, &editor).unwrap(),
            Edited::Saved
        );
        assert_eq!(rounds(&repo), 1);
        let saved = std::fs::read_to_string(path(repo.root())).unwrap();
        assert!(saved.contains("[node.\"renamed\"]"), "{saved}");
    }

    #[cfg(unix)]
    #[test]
    fn an_overlap_with_an_unknown_node_is_refused() {
        let (repo, tree, editor) = editing(
            "if [ \"$n\" = 1 ]; then printf '[node.\"a\"]\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"12:00\" }]\n[node.\"ghost\"]\nblocks = [{ days = \"mon\", from = \"10:00\", to = \"11:00\" }]\n' >> \"$1\"; else : > \"$1\"; fi",
        );
        assert_eq!(
            edit_with(repo.root(), &tree, &editor).unwrap(),
            Edited::Abandoned
        );
        assert_eq!(rounds(&repo), 2);
        assert!(!path(repo.root()).exists());
    }

    #[test]
    fn an_overlap_names_both_its_nodes_and_concerns_either() {
        let overlap = only(&with(
            "[node.\"a/b\"]\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"12:00\" }]\n[node.\"c\"]\nblocks = [{ days = \"mon\", from = \"11:00\", to = \"13:00\" }]\n",
        ));
        assert_eq!(
            (overlap.node.as_ref(), overlap.other.as_ref()),
            (Some(&NodePath::new("a/b")), Some(&NodePath::new("c")))
        );
        for scope in ["a", "a/b", "c"] {
            assert!(overlap.concerns(&NodePath::new(scope)), "{scope}");
        }
        assert!(!overlap.concerns(&NodePath::new("d")));
    }

    #[cfg(unix)]
    #[test]
    fn an_unchanged_or_emptied_draft_abandons_and_leaves_the_file_as_it_was() {
        let (repo, tree, editor) = editing("true");
        assert_eq!(
            edit_with(repo.root(), &tree, &editor).unwrap(),
            Edited::Abandoned
        );
        assert!(!path(repo.root()).exists());
        let (repo, tree, editor) =
            editing("if [ \"$n\" = 1 ]; then echo 'focus = 3' >> \"$1\"; else : > \"$1\"; fi");
        let original = "version = 1\ntimezone = \"UTC\"\n";
        std::fs::write(path(repo.root()), original).unwrap();
        assert_eq!(
            edit_with(repo.root(), &tree, &editor).unwrap(),
            Edited::Abandoned
        );
        assert_eq!(rounds(&repo), 2);
        assert_eq!(
            std::fs::read_to_string(path(repo.root())).unwrap(),
            original
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_editor_that_fails_leaves_the_file_as_it_was() {
        let (repo, tree, editor) = editing("echo 'oops' > \"$1\"; exit 1");
        assert!(edit_with(repo.root(), &tree, &editor).is_err());
        assert!(!path(repo.root()).exists());
    }
}

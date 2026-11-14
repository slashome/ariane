//! `ariane roadmap`: the forecast of a node, in a terminal or as JSON
//! (ADR-0028, SPEC.md §3).
//!
//! Nothing is computed here: [`crate::forecast`] gives the forecast, run on
//! the calendar's carrier of the node and narrowed to the node and its
//! descendants. This module checks the node, prints the forecast as text, and
//! prints it as the JSON that `GET /api/roadmap?node=<node>` serves.
//!
//! The text lists the releases in delivery order, then their epics, stories
//! and work, indented, each with its hours left (work, plus testing), its span
//! (dates in the calendar's time zone, else hours of focused work from now),
//! what is done and what is open, its target and whether the forecast misses
//! it. A header says which node's blocks give the dates; the unscheduled work
//! and the warnings come last, counted by kind with their items.

use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Result, bail};

use crate::calendar;
use crate::cli::RoadmapArgs;
use crate::forecast::{
    self, EpicPlan, Forecast, Moment, Reason, ReleasePlan, Span, Stage, StoryPlan, Summary,
    Unscheduled, Warning, WorkPlan,
};
use crate::history::History;
use crate::item::Kind;
use crate::tree::{NodePath, Tree};

/// The widest a title column grows before titles are cut.
const LABEL_WIDTH: usize = 64;

/// How many items a warning or an unscheduled reason lists on one line.
const LISTED: usize = 10;

/// How many items a warning lists when each has its own line, with a detail.
const DETAILED: usize = 5;

/// Tells whether `node` is a node of `tree`.
pub fn is_node(tree: &Tree, node: &NodePath) -> bool {
    tree.nodes.iter().any(|candidate| &candidate.path == node)
}

/// The forecast of `node`, from `now`, as `ariane roadmap` and `GET
/// /api/roadmap` give it: on the calendar of the personal repository at
/// `personal`, `history` dating finished work without `closed`.
///
/// # Errors
///
/// Fails when `node` is not a node of `tree`, or when the calendar exists but
/// cannot be read.
pub fn forecast(
    personal: &Path,
    tree: &Tree,
    history: Option<&History>,
    node: &NodePath,
    now: jiff::Timestamp,
) -> Result<Forecast> {
    if !is_node(tree, node) {
        bail!("`{node}` is not a node of the tree");
    }
    forecast::load(personal, tree, history, node, now)
}

/// Runs `ariane roadmap`: the forecast of the node asked, else of the current
/// directory's node, as text or JSON.
///
/// # Errors
///
/// Fails when no node is named outside a node's directory, when the node is
/// unknown, when the calendar cannot be read, or when `--release` names no
/// release of the forecast.
pub fn run(
    root: &Path,
    tree: &Tree,
    args: &RoadmapArgs,
    here: Option<&NodePath>,
) -> Result<ExitCode> {
    let Some(node) = args
        .node
        .as_deref()
        .map(NodePath::new)
        .or_else(|| here.filter(|node| !node.is_home()).cloned())
    else {
        bail!("name a node, `ariane roadmap <node>`, or run it inside a node's directory");
    };
    let history = History::read(root);
    let forecast = forecast(
        &calendar::personal_root(root),
        tree,
        Some(&history),
        &node,
        forecast::now(),
    )?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&forecast)?);
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(name) = args.release.as_deref()
        && !forecast.releases.iter().any(|release| release.name == name)
    {
        let names: Vec<&str> = forecast
            .releases
            .iter()
            .map(|release| release.name.as_str())
            .collect();
        bail!(
            "no release `{name}` in the roadmap of {node}; its releases: {}",
            names.join(", ")
        );
    }
    let shared = shared_with(tree, &forecast);
    print!("{}", render(&forecast, &shared, args.release.as_deref()));
    Ok(ExitCode::SUCCESS)
}

/// The nodes holding work that share the carrier's sequence with the node
/// asked, outside it: empty when the node carries its own blocks or has none.
fn shared_with(tree: &Tree, forecast: &Forecast) -> Vec<NodePath> {
    let Some(carrier) = forecast.carrier.as_ref() else {
        return Vec::new();
    };
    let mut nodes: Vec<NodePath> = tree
        .items_within(Some(carrier))
        .filter(|item| matches!(item.kind, Kind::Task | Kind::Story | Kind::Epic))
        .map(|item| item.node.clone())
        .filter(|node| !node.is_within(&forecast.node))
        .collect();
    nodes.sort();
    nodes.dedup();
    nodes
}

/// One line of the plan: its label, indented, and its columns.
struct Line {
    /// How deep it is: release 0, epic 1, story 2, work 3.
    depth: usize,
    /// The reference and the title.
    label: String,
    /// The hours left, testing added.
    hours: String,
    /// From its start to its delivery.
    span: String,
    /// Where it stands: done and open counts, or the stage of the work.
    state: String,
    /// Its target, and whether the forecast misses it.
    marks: String,
}

/// The text `ariane roadmap` prints for `forecast`, `shared` being the other
/// nodes of the carrier's sequence; with `release`, that release only.
pub fn render(forecast: &Forecast, shared: &[NodePath], release: Option<&str>) -> String {
    let missed: Vec<&str> = forecast
        .warnings
        .iter()
        .filter(|warning| warning.kind == forecast::WarningKind::MissedTarget)
        .flat_map(|warning| &warning.items)
        .map(|item| item.reference.as_str())
        .collect();
    let view = View {
        forecast,
        missed: &missed,
    };
    let mut text = header(forecast, shared);
    let mut lines = Vec::new();
    for plan in forecast
        .releases
        .iter()
        .filter(|plan| release.is_none_or(|name| plan.name == name))
    {
        view.release(plan, &mut lines);
    }
    if lines.is_empty() {
        text.push_str("\nNo release declared for this node: `[[release]]` in its node.toml.\n");
    } else {
        text.push('\n');
        text.push_str(&table(&lines));
    }
    text.push_str(&unscheduled(&view, &forecast.unscheduled));
    text.push_str(&warnings(&view, &forecast.warnings));
    text
}

/// The header: the node, where its dates come from, and what is left.
fn header(forecast: &Forecast, shared: &[NodePath]) -> String {
    let mut text = format!("Roadmap of {}\n", forecast.node);
    let calendar = match (&forecast.carrier, forecast.timezone.as_deref()) {
        (Some(carrier), Some(zone)) if carrier == &forecast.node => {
            format!("its own blocks, {zone}")
        }
        (Some(carrier), Some(zone)) if shared.is_empty() => {
            format!("the blocks of {carrier}, {zone}")
        }
        (Some(carrier), Some(zone)) => format!(
            "the blocks of {carrier}, {zone}, shared with {} in one sequence",
            listed(
                &shared.iter().map(ToString::to_string).collect::<Vec<_>>(),
                4
            )
        ),
        _ => format!(
            "none on {} or its ancestors: hours of focused work from now, no dates (`ariane calendar edit` adds blocks)",
            forecast.node
        ),
    };
    let _ = writeln!(text, "calendar  {calendar}");
    if forecast.dated {
        let _ = writeln!(text, "from      {}", clock(&forecast.now));
    }
    let (done, open) = forecast
        .releases
        .iter()
        .fold((0, 0), |(done, open), release| {
            (done + release.summary.done, open + release.summary.open)
        });
    let _ = writeln!(
        text,
        "left      {} h of work + {} h of testing, {done} done, {open} open",
        forecast.hours, forecast.test_hours
    );
    text
}

/// What the lines of the plan read from the forecast.
struct View<'a> {
    /// The forecast printed.
    forecast: &'a Forecast,
    /// The references whose target the forecast misses, `release <name>`
    /// for a release.
    missed: &'a [&'a str],
}

impl View<'_> {
    /// The lines of one release, its epics, stories and work.
    fn release(&self, plan: &ReleasePlan, lines: &mut Vec<Line>) {
        let mut label = plan.name.clone();
        let homonyms = self
            .forecast
            .releases
            .iter()
            .filter(|release| release.name == plan.name)
            .count();
        if homonyms > 1 {
            let _ = write!(label, "  (declared by {})", plan.declared_by);
        }
        if let Some(closed) = &plan.closed {
            let _ = write!(label, "  (closed {closed})");
        }
        lines.push(self.group(
            0,
            label,
            &plan.summary,
            plan.target.as_deref(),
            &format!("release {}", plan.name),
        ));
        for epic in &plan.epics {
            self.epic(epic, lines);
        }
    }

    /// The lines of one epic fragment; an epic not split yet is one line, and
    /// the work without epic nor story comes right under `(no epic)`.
    fn epic(&self, plan: &EpicPlan, lines: &mut Vec<Line>) {
        if let [story] = plan.stories.as_slice()
            && story.reference.is_none()
            && let [work] = story.work.as_slice()
            && Some(&work.reference) == plan.reference.as_ref()
        {
            lines.push(self.work(1, work));
            return;
        }
        let label = self.titled(
            plan.reference.as_deref(),
            plan.title.as_deref(),
            "(no epic)",
        );
        let reference = plan.reference.clone().unwrap_or_default();
        lines.push(self.group(1, label, &plan.summary, plan.target.as_deref(), &reference));
        for story in &plan.stories {
            if plan.reference.is_none() && story.reference.is_none() {
                lines.extend(story.work.iter().map(|work| self.work(2, work)));
            } else {
                self.story(story, lines);
            }
        }
    }

    /// The lines of one story fragment; a story not split yet is one line.
    fn story(&self, plan: &StoryPlan, lines: &mut Vec<Line>) {
        if let [work] = plan.work.as_slice()
            && Some(&work.reference) == plan.reference.as_ref()
        {
            lines.push(self.work(2, work));
            return;
        }
        let label = self.titled(
            plan.reference.as_deref(),
            plan.title.as_deref(),
            "(no story)",
        );
        let reference = plan.reference.clone().unwrap_or_default();
        lines.push(self.group(2, label, &plan.summary, plan.target.as_deref(), &reference));
        for work in &plan.work {
            lines.push(self.work(3, work));
        }
    }

    /// The line of a release, an epic or a story.
    fn group(
        &self,
        depth: usize,
        label: String,
        summary: &Summary,
        target: Option<&str>,
        reference: &str,
    ) -> Line {
        let total = summary.done + summary.open;
        let mut state = if total == 0 {
            "no work here".to_owned()
        } else {
            format!("{}/{total} done", summary.done)
        };
        if summary.open == 0
            && let Some(closed) = &summary.closed
        {
            let _ = write!(state, ", closed {}", range(&closed.from, &closed.to));
        }
        Line {
            depth,
            label,
            hours: if summary.open == 0 {
                String::new()
            } else {
                hours(summary.hours, summary.test_hours)
            },
            span: summary.span.as_ref().map(span).unwrap_or_default(),
            state,
            marks: self.marks(target, reference),
        }
    }

    /// The line of one piece of work.
    fn work(&self, depth: usize, plan: &WorkPlan) -> Line {
        let label = self.titled(Some(&plan.reference), Some(&plan.title), "");
        let state = match plan.stage {
            Stage::Done => match &plan.closed {
                Some(closed) if closed.approximate => format!("done ~{}", closed.date),
                Some(closed) => format!("done {}", closed.date),
                None => "done".to_owned(),
            },
            Stage::Open => String::new(),
            Stage::InProgress => "in progress".to_owned(),
            Stage::Testing => "testing".to_owned(),
            Stage::Waiting => "waiting".to_owned(),
        };
        let delivered = plan
            .start
            .as_ref()
            .zip(plan.delivered.as_ref())
            .map(|(start, end)| {
                span(&Span {
                    start: start.clone(),
                    end: end.clone(),
                })
            });
        Line {
            depth,
            label,
            hours: if plan.stage == Stage::Done {
                String::new()
            } else {
                hours(plan.hours, plan.test_hours)
            },
            span: delivered.unwrap_or_default(),
            state,
            marks: if plan.kind == Kind::Task {
                self.marks(plan.target.as_deref(), &plan.reference)
            } else {
                let marks = self.marks(plan.target.as_deref(), &plan.reference);
                format!("{} not split {marks}", kind_name(plan.kind))
                    .trim_end()
                    .to_owned()
            },
        }
    }

    /// `T12  Title`, the reference shortened from the node asked.
    fn titled(&self, reference: Option<&str>, title: Option<&str>, none: &str) -> String {
        match (reference, title) {
            (Some(reference), Some(title)) => format!("{}  {title}", self.short(reference)),
            (Some(reference), None) => self.short(reference),
            (None, _) => none.to_owned(),
        }
    }

    /// `reference` as read from the node asked: its id for an item of the
    /// node, `child#id` for a descendant's, the full reference otherwise.
    fn short(&self, reference: &str) -> String {
        let Some((node, id)) = reference.split_once('#') else {
            return reference.to_owned();
        };
        let asked = self.forecast.node.as_str();
        if node == asked {
            return id.to_owned();
        }
        match node
            .strip_prefix(asked)
            .and_then(|rest| rest.strip_prefix('/'))
        {
            Some(child) if !asked.is_empty() => format!("{child}#{id}"),
            _ => reference.to_owned(),
        }
    }

    /// `target 2026-12-01`, with `MISSED` when the forecast misses it.
    fn marks(&self, target: Option<&str>, reference: &str) -> String {
        let missed = !reference.is_empty() && self.missed.contains(&reference);
        match (target, missed) {
            (Some(target), true) => format!("target {target} MISSED"),
            (Some(target), false) => format!("target {target}"),
            (None, true) => "MISSED".to_owned(),
            (None, false) => String::new(),
        }
    }
}

/// The lines as aligned columns, labels indented by depth.
fn table(lines: &[Line]) -> String {
    let label = |line: &Line| {
        let indent = "  ".repeat(line.depth);
        cut(&format!("{indent}{}", line.label), LABEL_WIDTH)
    };
    let labels: Vec<String> = lines.iter().map(label).collect();
    let width = |column: &dyn Fn(&Line) -> &str| {
        lines
            .iter()
            .map(|line| column(line).chars().count())
            .max()
            .unwrap_or(0)
    };
    let label_width = labels
        .iter()
        .map(|label| label.chars().count())
        .max()
        .unwrap_or(0);
    let hours_width = width(&|line| &line.hours);
    let span_width = width(&|line| &line.span);
    let state_width = width(&|line| &line.state);
    let mut text = String::new();
    for (line, label) in lines.iter().zip(&labels) {
        if line.depth == 0 && !text.is_empty() {
            text.push('\n');
        }
        let row = format!(
            "{label:<label_width$}  {:>hours_width$}  {:<span_width$}  {:<state_width$}  {}",
            line.hours, line.span, line.state, line.marks
        );
        text.push_str(row.trim_end());
        text.push('\n');
    }
    text
}

/// The unscheduled work, counted by reason.
fn unscheduled(view: &View, entries: &[Unscheduled]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut reasons: Vec<(String, Vec<String>)> = Vec::new();
    for entry in entries {
        let reason = match (entry.reason, entry.release.as_deref()) {
            (Reason::Parking, Some(release)) => format!("parked in {release}"),
            (Reason::Parking, None) => "parked".to_owned(),
            (Reason::ClosedRelease, Some(release)) => format!("in {release}, closed"),
            (Reason::ClosedRelease, None) => "in a closed release".to_owned(),
            (Reason::NoRelease, _) => "no release".to_owned(),
            (Reason::UndeclaredRelease, Some(release)) => format!("undeclared release {release}"),
            (Reason::UndeclaredRelease, None) => "undeclared release".to_owned(),
            (Reason::Cycle, _) => "in a dependency cycle".to_owned(),
            (Reason::BlockedByCycle, _) => "waiting on a cycle".to_owned(),
        };
        let reference = view.short(&entry.reference);
        match reasons.iter_mut().find(|(known, _)| *known == reason) {
            Some((_, references)) => references.push(reference),
            None => reasons.push((reason, vec![reference])),
        }
    }
    let width = reasons
        .iter()
        .map(|(reason, _)| reason.chars().count())
        .max()
        .unwrap_or(0);
    let mut text = format!("\nUnscheduled ({})\n", entries.len());
    for (reason, references) in &reasons {
        let _ = writeln!(
            text,
            "  {reason:<width$}  {:>3}  {}",
            references.len(),
            listed(references, LISTED)
        );
    }
    text
}

/// The warnings, counted by kind with their items.
fn warnings(view: &View, warnings: &[Warning]) -> String {
    let total: usize = warnings.iter().map(|warning| warning.count).sum();
    if total == 0 {
        return "\nNo warnings.\n".to_owned();
    }
    let width = warnings
        .iter()
        .map(|warning| kind_label(warning).len())
        .max()
        .unwrap_or(0);
    let mut text = format!("\nWarnings ({total})\n");
    for warning in warnings {
        let label = kind_label(warning);
        let references: Vec<String> = warning
            .items
            .iter()
            .map(|item| view.short(&item.reference))
            .collect();
        if warning.items.iter().all(|item| item.detail.is_none()) {
            let _ = writeln!(
                text,
                "  {label:<width$}  {:>3}  {}",
                warning.count,
                listed(&references, LISTED)
            );
            continue;
        }
        let _ = writeln!(text, "  {label:<width$}  {:>3}", warning.count);
        for (item, reference) in warning.items.iter().zip(&references).take(DETAILED) {
            let detail = item.detail.as_deref().unwrap_or_default();
            let _ = writeln!(text, "      {reference}  {detail}");
        }
        if warning.items.len() > DETAILED {
            let _ = writeln!(text, "      … and {} more", warning.items.len() - DETAILED);
        }
    }
    text
}

/// The name of a warning's kind, as the JSON writes it.
fn kind_label(warning: &Warning) -> String {
    serde_json::to_value(warning.kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The name of a kind of work.
fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Epic => "epic",
        Kind::Story => "story",
        _ => "task",
    }
}

/// `entries` joined by commas, the first `limit` only, then how many more.
fn listed(entries: &[String], limit: usize) -> String {
    let mut text = entries
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if entries.len() > limit {
        let _ = write!(text, ", … and {} more", entries.len() - limit);
    }
    text
}

/// `4 h`, or `4 h + 2 h` with testing.
fn hours(work: f64, test: f64) -> String {
    if test > 0.0 {
        format!("{work} h + {test} h")
    } else {
        format!("{work} h")
    }
}

/// `start → end`: dates and times when dated, hours from now otherwise.
fn span(span: &Span) -> String {
    format!("{} → {}", moment(&span.start), moment(&span.end))
}

/// A moment: `2026-11-16 09:00` when dated, `+36 h` otherwise.
fn moment(moment: &Moment) -> String {
    match moment.at.as_deref() {
        Some(at) => clock(at),
        None => format!("+{} h", moment.hours),
    }
}

/// An RFC 3339 moment as `2026-11-16 09:00`.
fn clock(at: &str) -> String {
    match (at.get(..10), at.get(11..16)) {
        (Some(day), Some(time)) => format!("{day} {time}"),
        _ => at.to_owned(),
    }
}

/// `from..to`, or one date when both are the same.
fn range(from: &str, to: &str) -> String {
    if from == to {
        from.to_owned()
    } else {
        format!("{from}..{to}")
    }
}

/// `text` cut to `width` characters, an ellipsis marking the cut.
fn cut(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    let mut kept: String = text.chars().take(width.saturating_sub(1)).collect();
    kept.push('…');
    kept
}

/// Tests of `ariane roadmap`: the forecast it reads, and the text it prints.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// Releases `1.0`, `2.0` and a parking `later`, sizes priced 1 to 16
    /// hours, testing by the owner at half the hours.
    const NODE: &str = "[[release]]\nname = \"1.0\"\ntarget = 2026-11-20\n\n[[release]]\nname = \"2.0\"\n\n[[release]]\nname = \"later\"\nparking = true\n\n[estimates]\nXS = 1\nS = 2\nM = 4\nL = 8\nXL = 16\ntest_ratio = 0.5\n";

    /// Mondays 09:00–13:00 in Paris for node `p`.
    const CALENDAR: &str = "version = 1\ntimezone = \"Europe/Paris\"\n\n[node.\"p\"]\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"13:00\" }]\n";

    /// A Monday morning, before any block.
    fn monday() -> jiff::Timestamp {
        "2026-11-16T07:00:00Z".parse().unwrap()
    }

    /// An item of `node` with `id`, `status` and extra header lines.
    fn item(node: &str, id: &str, status: &str, extra: &str) -> String {
        format!(
            "---\nariane: 2\nnode: {node}\nid: {id}\ntitle: Title of {id}\nstatus: {status}\n{extra}---\n# {id}\n"
        )
    }

    /// Node `p` with an epic, a story, tasks of `p` and of its child `p/q`,
    /// finished and parked work, and the sibling `p/r` with work of its own.
    fn sample() -> Repo {
        let repo = Repo::new();
        repo.write("p/node.toml", NODE)
            .write("p/q/node.toml", "")
            .write("p/r/node.toml", "")
            .write(
                "p/epics/E1-a.md",
                &item("p", "E1", "todo", "release: '1.0'\n"),
            )
            .write(
                "p/stories/S1-a.md",
                &item("p", "S1", "todo", "parent: E1\n"),
            )
            .write(
                "p/tasks/T1-a.md",
                &item("p", "T1", "todo", "parent: S1\nsize: S\nrank: 1\n"),
            )
            .write(
                "p/tasks/T2-b.md",
                &item(
                    "p",
                    "T2",
                    "done",
                    "parent: S1\nsize: XS\nclosed: 2026-11-02\nrank: 2\n",
                ),
            )
            .write(
                "p/q/tasks/T1-a.md",
                &item("p/q", "T1", "todo", "parent: p#S1\nsize: M\n"),
            )
            .write(
                "p/q/tasks/T2-b.md",
                &item("p/q", "T2", "todo", "release: later\n"),
            )
            .write(
                "p/r/tasks/T1-a.md",
                &item("p/r", "T1", "todo", "release: '2.0'\n"),
            );
        repo
    }

    /// The forecast of `node` in `repo`, from [`monday`], the calendar
    /// written as `calendar` in the repository when given.
    fn roadmap(repo: &Repo, node: &str, calendar: Option<&str>) -> Forecast {
        if let Some(calendar) = calendar {
            std::fs::write(repo.root().join(calendar::FILE), calendar).unwrap();
        }
        let tree = Tree::load(repo.root()).unwrap();
        forecast(repo.root(), &tree, None, &NodePath::new(node), monday()).unwrap()
    }

    #[test]
    fn an_unknown_node_is_refused() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let refused = forecast(repo.root(), &tree, None, &NodePath::new("nope"), monday());
        assert!(refused.unwrap_err().to_string().contains("not a node"));
    }

    #[test]
    fn without_calendar_the_text_reads_hours_from_now_and_the_warnings() {
        let repo = sample();
        let forecast = roadmap(&repo, "p", None);
        let text = render(&forecast, &[], None);
        assert!(text.starts_with("Roadmap of p\n"), "{text}");
        assert!(
            text.contains("calendar  none on p or its ancestors"),
            "{text}"
        );
        assert!(text.contains("`ariane calendar edit`"), "{text}");
        assert!(!text.contains("\nfrom "), "{text}");
        assert!(
            text.contains("left      10 h of work + 5 h of testing, 1 done, 3 open"),
            "{text}"
        );
        assert!(text.contains("\n1.0 "), "{text}");
        assert!(text.contains("  E1  Title of E1"), "{text}");
        assert!(text.contains("    S1  Title of S1"), "{text}");
        assert!(text.contains("      T1  Title of T1"), "{text}");
        assert!(text.contains("      q#T1  Title of T1"), "{text}");
        assert!(text.contains("+0 h → +3 h"), "{text}");
        assert!(text.contains("done 2026-11-02"), "{text}");
        assert!(text.contains("1/3 done"), "{text}");
        assert!(text.contains("target 2026-11-20"), "{text}");
        assert!(
            text.contains("Unscheduled (1)\n  parked in later"),
            "{text}"
        );
        assert!(text.contains("Warnings ("), "{text}");
        assert!(text.contains("unsized"), "{text}");
        assert!(text.contains("r#T1"), "{text}");
    }

    #[test]
    fn with_a_calendar_the_text_reads_dates_and_missed_targets() {
        let repo = sample();
        let forecast = roadmap(&repo, "p", Some(CALENDAR));
        let text = render(&forecast, &[], None);
        assert!(
            text.contains("calendar  its own blocks, Europe/Paris"),
            "{text}"
        );
        assert!(text.contains("from      2026-11-16 08:00"), "{text}");
        assert!(
            text.contains("2026-11-16 09:00 → 2026-11-16 12:00"),
            "{text}"
        );
        assert!(text.contains("target 2026-11-20 MISSED"), "{text}");
        assert!(text.contains("missed_target"), "{text}");
        assert!(
            text.contains("release 1.0  target 2026-11-20, forecast"),
            "{text}"
        );
    }

    #[test]
    fn a_child_reads_its_part_of_the_pooled_sequence_and_says_with_whom() {
        let repo = sample();
        let forecast = roadmap(&repo, "p/q", Some(CALENDAR));
        assert_eq!(forecast.node, NodePath::new("p/q"));
        assert_eq!(forecast.carrier, Some(NodePath::new("p")));
        let tree = Tree::load(repo.root()).unwrap();
        let shared = shared_with(&tree, &forecast);
        assert_eq!(shared, [NodePath::new("p"), NodePath::new("p/r")]);
        let text = render(&forecast, &shared, None);
        assert!(
            text.contains(
                "calendar  the blocks of p, Europe/Paris, shared with p, p/r in one sequence"
            ),
            "{text}"
        );
        assert!(text.contains("      T1  Title of T1"), "{text}");
        assert!(text.contains("  p#E1  Title of E1"), "{text}");
        assert!(!text.contains("p#T1"), "{text}");
        assert!(
            text.contains("2026-11-16 12:00 → 2026-11-30 10:00"),
            "{text}"
        );
        assert!(
            text.contains("Unscheduled (1)\n  parked in later    1  T2"),
            "{text}"
        );
        assert!(!text.contains("r#T1"), "{text}");
        assert!(text.contains("\n2.0  "), "{text}");
        assert!(text.contains("no work here"), "{text}");
        let json = serde_json::to_value(&forecast).unwrap();
        assert_eq!(json["node"], "p/q");
        assert_eq!(json["carrier"], "p");
        assert_eq!(json["releases"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn a_release_can_be_read_alone() {
        let repo = sample();
        let forecast = roadmap(&repo, "p", None);
        let text = render(&forecast, &[], Some("2.0"));
        assert!(text.contains("\n2.0 "), "{text}");
        assert!(!text.contains("\n1.0 "), "{text}");
    }

    #[test]
    fn long_lists_are_cut_with_a_count() {
        let entries: Vec<String> = (1..=13).map(|index| format!("T{index}")).collect();
        assert_eq!(
            listed(&entries, LISTED),
            "T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, … and 3 more"
        );
        assert_eq!(cut("abcdef", 4), "abc…");
        assert_eq!(cut("abc", 4), "abc");
    }
}

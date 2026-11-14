//! Fixture tests of the forecast engine: content repositories written to a
//! temporary directory, a calendar given as text, and `now` fixed.

use std::collections::HashMap;
use std::path::PathBuf;

use jiff::Timestamp;

use super::*;
use crate::calendar;
use crate::history::{History, Transition};
use crate::tree::fixture::Repo;

/// Releases `1.0` and `2.0` and a parking `later`, sizes priced 1 to 16
/// hours, no testing.
const NODE: &str = "[[release]]\nname = \"1.0\"\n\n[[release]]\nname = \"2.0\"\n\n[[release]]\nname = \"later\"\nparking = true\n\n[estimates]\nXS = 1\nS = 2\nM = 4\nL = 8\nXL = 16\ntest_ratio = 0\n";

/// Mondays 09:00–13:00 in Paris at half focus, the 23rd of November off.
const PARIS: &str = "version = 1\ntimezone = \"Europe/Paris\"\ndays_off = [\"2026-11-23\"]\n\n[node.\"p\"]\nfocus = 0.5\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"13:00\" }]\n";

/// An item of node `node` with `id`, `status` and extra header lines.
fn item_of(node: &str, id: &str, status: &str, extra: &str) -> String {
    format!(
        "---\nariane: 2\nnode: {node}\nid: {id}\ntitle: {id}\nstatus: {status}\n{extra}---\n# {id}\n"
    )
}

/// An item of node `p`.
fn item(id: &str, status: &str, extra: &str) -> String {
    item_of("p", id, status, extra)
}

/// A repository whose node `p` declares `node_toml`.
fn repo(node_toml: &str) -> Repo {
    let repo = Repo::new();
    repo.write("p/node.toml", node_toml);
    repo
}

/// [`NODE`] with testing at half the hours, by `tester`.
fn tested_by(tester: &str) -> String {
    format!(
        "{}tester = \"{tester}\"\n",
        NODE.replace("test_ratio = 0", "test_ratio = 0.5")
    )
}

/// `node_toml` with the default workflow and a status `a-tester` of the
/// `in_progress` category at the `testing` stage.
fn with_testing(node_toml: &str) -> String {
    let mut text = node_toml.to_owned();
    for category in [
        "backlog",
        "todo",
        "in_progress",
        "waiting",
        "blocked",
        "done",
        "dropped",
    ] {
        text.push_str(&format!(
            "\n[[workflow]]\nid = \"{category}\"\nlabel = \"{category}\"\ncategory = \"{category}\"\n"
        ));
    }
    text.push_str(
        "\n[[workflow]]\nid = \"a-tester\"\nlabel = \"À tester\"\ncategory = \"in_progress\"\nstage = \"testing\"\n",
    );
    text
}

/// The moment `text`, RFC 3339.
fn at(text: &str) -> Timestamp {
    text.parse().unwrap()
}

/// A Monday morning, before any block.
fn monday() -> Timestamp {
    at("2026-11-16T07:00:00Z")
}

/// The forecast of `node` in `repo`, with the calendar `calendar` if any.
fn forecast_of(repo: &Repo, node: &str, calendar: Option<&str>, now: Timestamp) -> Forecast {
    let tree = Tree::load(repo.root()).unwrap();
    let calendar = calendar.map(|text| calendar::check(text, |_| true).calendar.unwrap());
    build(&tree, calendar.as_ref(), None, &NodePath::new(node), now)
}

/// The forecast of node `p` without calendar.
fn relative(repo: &Repo) -> Forecast {
    forecast_of(repo, "p", None, monday())
}

/// Every piece of work of the forecast, in release, epic and story order.
fn works(forecast: &Forecast) -> Vec<&WorkPlan> {
    forecast
        .releases
        .iter()
        .flat_map(|release| &release.epics)
        .flat_map(|epic| &epic.stories)
        .flat_map(|story| &story.work)
        .collect()
}

/// The references of the owner's sequence, in order.
fn sequence(forecast: &Forecast) -> Vec<String> {
    let mut placed: Vec<(usize, String)> = works(forecast)
        .into_iter()
        .filter_map(|work| Some((work.sequence?, work.reference.clone())))
        .collect();
    placed.sort();
    placed.into_iter().map(|(_, reference)| reference).collect()
}

/// The piece of work `reference`.
fn work<'a>(forecast: &'a Forecast, reference: &str) -> &'a WorkPlan {
    works(forecast)
        .into_iter()
        .find(|work| work.reference == reference)
        .unwrap()
}

/// The references a warning of `kind` flags, checking its count.
fn flagged(forecast: &Forecast, kind: WarningKind) -> Vec<String> {
    forecast
        .warnings
        .iter()
        .find(|warning| warning.kind == kind)
        .map(|warning| {
            assert_eq!(warning.count, warning.items.len());
            warning
                .items
                .iter()
                .map(|item| item.reference.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// `(hours, at)` of a moment.
fn moment(moment: &Option<Moment>) -> (f64, Option<&str>) {
    let moment = moment.as_ref().unwrap();
    (moment.hours, moment.at.as_deref())
}

/// The release named `name`.
fn release<'a>(forecast: &'a Forecast, name: &str) -> &'a ReleasePlan {
    forecast
        .releases
        .iter()
        .find(|release| release.name == name)
        .unwrap()
}

/// `(start, end)` hours of a span.
fn hours(span: &Option<Span>) -> (f64, f64) {
    let span = span.as_ref().unwrap();
    (span.start.hours, span.end.hours)
}

#[test]
fn work_follows_releases_then_epics_then_stories_then_rank() {
    let repo = repo(NODE);
    repo.write(
        "p/epics/E1-a.md",
        &item("E1", "todo", "release: '1.0'\nrank: 2000\n"),
    )
    .write(
        "p/epics/E2-b.md",
        &item("E2", "todo", "release: '1.0'\nrank: 1000\n"),
    )
    .write(
        "p/stories/S1-a.md",
        &item("S1", "todo", "parent: E1\nrank: 1\n"),
    )
    .write("p/stories/S2-b.md", &item("S2", "todo", "parent: E2\n"))
    .write(
        "p/stories/S3-c.md",
        &item("S3", "todo", "parent: E1\nrank: 2\n"),
    )
    .write(
        "p/tasks/T1-a.md",
        &item("T1", "todo", "parent: S1\nsize: S\nrank: 20\n"),
    )
    .write(
        "p/tasks/T2-b.md",
        &item("T2", "todo", "parent: S1\nsize: S\nrank: 10\n"),
    )
    .write(
        "p/tasks/T3-c.md",
        &item("T3", "todo", "parent: S2\nsize: S\n"),
    )
    .write(
        "p/tasks/T4-d.md",
        &item("T4", "todo", "parent: S3\nsize: S\n"),
    )
    .write(
        "p/tasks/T5-e.md",
        &item("T5", "todo", "release: '1.0'\nsize: S\n"),
    )
    .write(
        "p/tasks/T6-f.md",
        &item("T6", "todo", "parent: S2\nrelease: '2.0'\nsize: S\n"),
    );
    let forecast = relative(&repo);
    assert_eq!(
        sequence(&forecast),
        ["p#T3", "p#T2", "p#T1", "p#T4", "p#T5", "p#T6"]
    );
    let first = release(&forecast, "1.0");
    let epics: Vec<Option<&str>> = first
        .epics
        .iter()
        .map(|epic| epic.reference.as_deref())
        .collect();
    assert_eq!(epics, [Some("p#E2"), Some("p#E1"), None]);
    let stories: Vec<Option<&str>> = first.epics[1]
        .stories
        .iter()
        .map(|story| story.reference.as_deref())
        .collect();
    assert_eq!(stories, [Some("p#S1"), Some("p#S3")]);
    assert_eq!(first.epics[2].stories[0].reference, None);
    assert_eq!((first.summary.hours, first.summary.open), (10.0, 5));
    assert_eq!(hours(&first.summary.span), (0.0, 10.0));
    let second = release(&forecast, "2.0");
    assert_eq!(second.epics[0].reference.as_deref(), Some("p#E2"));
    assert_eq!(moment(&work(&forecast, "p#T6").start), (10.0, None));
    assert_eq!(forecast.hours, 12.0);
    assert!(!forecast.dated);
}

#[test]
fn dependencies_on_tasks_stories_and_epics_pass_over_work_not_ready() {
    let repo = repo(NODE);
    repo.write(
        "p/epics/E0-a.md",
        &item("E0", "todo", "release: '1.0'\nrank: 1\n"),
    )
    .write(
        "p/epics/E1-b.md",
        &item("E1", "todo", "release: '1.0'\nrank: 2\n"),
    )
    .write(
        "p/stories/S0-a.md",
        &item("S0", "todo", "parent: E0\nrank: 1\n"),
    )
    .write(
        "p/stories/S4-b.md",
        &item("S4", "todo", "parent: E0\nrank: 2\ndepends_on:\n- E1\n"),
    )
    .write(
        "p/stories/S1-c.md",
        &item("S1", "todo", "parent: E1\nrank: 1\n"),
    )
    .write(
        "p/stories/S5-d.md",
        &item("S5", "todo", "parent: E1\nrank: 2\n"),
    )
    .write(
        "p/tasks/T0-a.md",
        &item("T0", "todo", "parent: S0\nsize: S\ndepends_on:\n- S1\n"),
    )
    .write(
        "p/tasks/T7-b.md",
        &item("T7", "todo", "parent: S4\nsize: S\n"),
    )
    .write(
        "p/tasks/T1-c.md",
        &item("T1", "todo", "parent: S1\nsize: S\nrank: 1\n"),
    )
    .write(
        "p/tasks/T2-d.md",
        &item("T2", "todo", "parent: S1\nsize: S\nrank: 2\n"),
    )
    .write(
        "p/tasks/T8-e.md",
        &item("T8", "todo", "parent: S5\nsize: S\n"),
    )
    .write(
        "p/tasks/T5-f.md",
        &item(
            "T5",
            "todo",
            "release: '1.0'\nsize: S\nrank: 5\ndepends_on:\n- T6\n",
        ),
    )
    .write(
        "p/tasks/T6-g.md",
        &item("T6", "todo", "release: '1.0'\nsize: S\nrank: 6\n"),
    )
    .write(
        "p/tasks/T9-h.md",
        &item(
            "T9",
            "todo",
            "release: '1.0'\nsize: S\nrank: 9\ndepends_on:\n- T10\n",
        ),
    )
    .write(
        "p/tasks/T10-i.md",
        &item("T10", "done", "release: '1.0'\nsize: S\n"),
    );
    let forecast = relative(&repo);
    assert_eq!(
        sequence(&forecast),
        [
            "p#T1", "p#T2", "p#T0", "p#T8", "p#T7", "p#T6", "p#T5", "p#T9"
        ]
    );
    let epic = &release(&forecast, "1.0").epics[0];
    assert_eq!(epic.reference.as_deref(), Some("p#E0"));
    assert_eq!(hours(&epic.summary.span), (4.0, 10.0));
    assert!(forecast.unscheduled.is_empty());
    assert!(flagged(&forecast, WarningKind::DependencyCycle).is_empty());
}

#[test]
fn a_dependency_cycle_is_left_unscheduled_with_what_waits_on_it() {
    let repo = repo(NODE);
    repo.write(
        "p/tasks/T1-a.md",
        &item("T1", "todo", "release: '1.0'\ndepends_on:\n- T2\n"),
    )
    .write(
        "p/tasks/T2-b.md",
        &item("T2", "todo", "release: '1.0'\ndepends_on:\n- T1\n"),
    )
    .write(
        "p/tasks/T3-c.md",
        &item("T3", "todo", "release: '1.0'\ndepends_on:\n- T1\n"),
    )
    .write("p/tasks/T4-d.md", &item("T4", "todo", "release: '1.0'\n"));
    let forecast = relative(&repo);
    assert_eq!(sequence(&forecast), ["p#T4"]);
    let reasons: Vec<(&str, Reason)> = forecast
        .unscheduled
        .iter()
        .map(|entry| (entry.reference.as_str(), entry.reason))
        .collect();
    assert_eq!(
        reasons,
        [
            ("p#T1", Reason::Cycle),
            ("p#T2", Reason::Cycle),
            ("p#T3", Reason::BlockedByCycle)
        ]
    );
    assert_eq!(
        flagged(&forecast, WarningKind::DependencyCycle),
        ["p#T1", "p#T2"]
    );
}

#[test]
fn parking_unplanned_and_undeclared_work_is_shown_not_scheduled() {
    let repo = repo(NODE);
    repo.write("p/tasks/T1-a.md", &item("T1", "todo", "release: later\n"))
        .write("p/tasks/T2-b.md", &item("T2", "todo", ""))
        .write("p/tasks/T3-c.md", &item("T3", "todo", "release: '9.9'\n"))
        .write(
            "p/tasks/T4-d.md",
            &item("T4", "todo", "release: '1.0'\nsize: S\ndepends_on:\n- T1\n"),
        )
        .write(
            "p/tasks/T5-e.md",
            &item("T5", "dropped", "release: '1.0'\n"),
        );
    let forecast = relative(&repo);
    assert_eq!(sequence(&forecast), ["p#T4"]);
    assert_eq!(forecast.releases.len(), 2);
    let reasons: Vec<(&str, Reason, Option<&str>)> = forecast
        .unscheduled
        .iter()
        .map(|entry| {
            (
                entry.reference.as_str(),
                entry.reason,
                entry.release.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        reasons,
        [
            ("p#T1", Reason::Parking, Some("later")),
            ("p#T2", Reason::NoRelease, None),
            ("p#T3", Reason::UndeclaredRelease, Some("9.9")),
        ]
    );
    assert_eq!(flagged(&forecast, WarningKind::WithoutRelease), ["p#T2"]);
    assert_eq!(flagged(&forecast, WarningKind::UndeclaredRelease), ["p#T3"]);
    assert_eq!(
        flagged(&forecast, WarningKind::UnscheduledDependency),
        ["p#T4"]
    );
    assert_eq!(release(&forecast, "1.0").summary.open, 1);
}

#[test]
fn a_closed_release_is_delivered_its_finished_work_shown_and_its_open_work_not_scheduled() {
    let repo = repo(&NODE.replace("name = \"1.0\"\n", "name = \"1.0\"\nclosed = 2026-11-10\n"));
    repo.write(
        "p/archives/tasks/T1-a.md",
        &item(
            "T1",
            "done",
            "release: '1.0'\nclosed: 2026-11-09\nsize: S\n",
        ),
    )
    .write(
        "p/tasks/T2-b.md",
        &item("T2", "todo", "release: '1.0'\nsize: S\n"),
    )
    .write(
        "p/tasks/T3-c.md",
        &item("T3", "todo", "release: '2.0'\nsize: S\n"),
    );
    let forecast = relative(&repo);
    assert_eq!(sequence(&forecast), ["p#T3"]);
    let closed = release(&forecast, "1.0");
    assert_eq!(closed.closed.as_deref(), Some("2026-11-10"));
    assert!(closed.summary.span.is_none());
    assert_eq!((closed.summary.done, closed.summary.open), (1, 0));
    assert_eq!(release(&forecast, "2.0").closed, None);
    assert_eq!(forecast.unscheduled.len(), 1);
    assert_eq!(forecast.unscheduled[0].reference, "p#T2");
    assert_eq!(forecast.unscheduled[0].reason, Reason::ClosedRelease);
    assert_eq!(forecast.unscheduled[0].release.as_deref(), Some("1.0"));
}

#[test]
fn an_unsplit_story_or_epic_is_one_block_at_its_provisional_size() {
    let repo = repo(NODE);
    repo.write("p/epics/E1-a.md", &item("E1", "todo", "release: '1.0'\n"))
        .write(
            "p/epics/E2-b.md",
            &item("E2", "todo", "release: '1.0'\nsize: XL\n"),
        )
        .write("p/stories/S2-a.md", &item("S2", "todo", "parent: E2\n"))
        .write(
            "p/stories/S1-b.md",
            &item("S1", "todo", "release: '1.0'\nsize: L\n"),
        )
        .write(
            "p/tasks/T1-a.md",
            &item("T1", "todo", "parent: S2\nsize: S\n"),
        );
    let forecast = relative(&repo);
    assert_eq!(sequence(&forecast), ["p#E1", "p#T1", "p#S1"]);
    let epic = work(&forecast, "p#E1");
    assert_eq!(
        (epic.kind, epic.hours, epic.basis),
        (Kind::Epic, 4.0, Basis::Default)
    );
    let story = work(&forecast, "p#S1");
    assert_eq!(
        (story.kind, story.hours, story.basis),
        (Kind::Story, 8.0, Basis::Provisional)
    );
    assert_eq!(
        flagged(&forecast, WarningKind::Provisional),
        ["p#E1", "p#S1"]
    );
    assert_eq!(forecast.hours, 14.0);
}

#[test]
fn the_owner_tests_with_hours_and_others_validate_while_the_next_task_proceeds() {
    let owner = repo(&tested_by("owner"));
    let others = repo(&tested_by("others"));
    for repo in [&owner, &others] {
        repo.write(
            "p/tasks/T1-a.md",
            &item("T1", "todo", "release: '1.0'\nsize: S\n"),
        )
        .write(
            "p/tasks/T2-b.md",
            &item("T2", "todo", "release: '2.0'\nsize: S\n"),
        );
    }
    let forecast = relative(&owner);
    let first = work(&forecast, "p#T1");
    let phases: Vec<(Phase, f64, f64)> = first
        .segments
        .iter()
        .map(|segment| (segment.phase, segment.start.hours, segment.end.hours))
        .collect();
    assert_eq!(phases, [(Phase::Work, 0.0, 2.0), (Phase::Test, 2.0, 3.0)]);
    assert_eq!(first.validation, None);
    assert_eq!(moment(&work(&forecast, "p#T2").start), (3.0, None));
    assert_eq!(hours(&release(&forecast, "2.0").summary.span).1, 6.0);

    let forecast = relative(&others);
    let first = work(&forecast, "p#T1");
    assert_eq!(first.segments.len(), 1);
    assert_eq!(moment(&first.end), (2.0, None));
    assert_eq!(hours(&first.validation), (2.0, 3.0));
    assert_eq!(moment(&first.delivered), (3.0, None));
    assert_eq!(moment(&work(&forecast, "p#T2").start), (2.0, None));
    assert_eq!(hours(&release(&forecast, "1.0").summary.span), (0.0, 3.0));
    assert_eq!(forecast.test_hours, 2.0);
}

#[test]
fn waiting_work_is_a_delay_and_work_in_progress_counts_all_its_hours() {
    let repo = repo(&tested_by("owner"));
    repo.write(
        "p/tasks/T1-a.md",
        &item("T1", "waiting", "release: '1.0'\nsize: S\nrank: 1\n"),
    )
    .write(
        "p/tasks/T2-b.md",
        &item("T2", "in_progress", "release: '1.0'\nsize: M\nrank: 2\n"),
    );
    let forecast = relative(&repo);
    let waiting = work(&forecast, "p#T1");
    assert_eq!((waiting.stage, waiting.sequence), (Stage::Waiting, None));
    assert!(waiting.segments.is_empty());
    assert_eq!(hours(&waiting.validation), (0.0, 1.0));
    let started = work(&forecast, "p#T2");
    assert_eq!(
        (started.stage, started.sequence, started.hours),
        (Stage::InProgress, Some(0), 4.0)
    );
    assert_eq!(moment(&started.end), (6.0, None));
}

#[test]
fn work_in_testing_counts_only_its_testing_in_the_owners_hours() {
    let repo = repo(&with_testing(&tested_by("owner")));
    repo.write(
        "p/tasks/T1-a.md",
        &item("T1", "a-tester", "release: '1.0'\nsize: M\nrank: 2\n"),
    )
    .write(
        "p/tasks/T2-b.md",
        &item("T2", "todo", "release: '1.0'\nsize: S\nrank: 1\n"),
    );
    let forecast = relative(&repo);
    assert_eq!(sequence(&forecast), ["p#T1", "p#T2"]);
    let testing = work(&forecast, "p#T1");
    assert_eq!(
        (testing.stage, testing.hours, testing.test_hours),
        (Stage::Testing, 0.0, 2.0)
    );
    let phases: Vec<(Phase, f64, f64)> = testing
        .segments
        .iter()
        .map(|segment| (segment.phase, segment.start.hours, segment.end.hours))
        .collect();
    assert_eq!(phases, [(Phase::Test, 0.0, 2.0)]);
    assert_eq!(testing.validation, None);
    assert_eq!(moment(&work(&forecast, "p#T2").start), (2.0, None));
    assert_eq!(forecast.hours, 2.0);
    assert_eq!(forecast.test_hours, 3.0);
}

#[test]
fn work_in_testing_by_others_is_a_validation_from_now() {
    let repo = repo(&with_testing(&tested_by("others")));
    repo.write(
        "p/tasks/T1-a.md",
        &item("T1", "a-tester", "release: '1.0'\nsize: M\n"),
    )
    .write(
        "p/tasks/T2-b.md",
        &item("T2", "todo", "release: '1.0'\nsize: S\n"),
    );
    let forecast = relative(&repo);
    let testing = work(&forecast, "p#T1");
    assert_eq!((testing.stage, testing.sequence), (Stage::Testing, None));
    assert!(testing.segments.is_empty());
    assert_eq!(hours(&testing.validation), (0.0, 2.0));
    assert_eq!(moment(&testing.delivered), (2.0, None));
    assert_eq!(sequence(&forecast), ["p#T2"]);
    assert_eq!(moment(&work(&forecast, "p#T2").start), (0.0, None));
}

#[test]
fn work_in_progress_comes_first_whatever_its_release_dependencies_kept() {
    let repo = repo(&with_testing(NODE));
    repo.write(
        "p/tasks/T1-a.md",
        &item("T1", "todo", "release: '1.0'\nsize: S\nrank: 1\n"),
    )
    .write(
        "p/tasks/T2-b.md",
        &item("T2", "todo", "release: '1.0'\nsize: S\nrank: 2\n"),
    )
    .write(
        "p/tasks/T3-c.md",
        &item("T3", "in_progress", "release: '2.0'\nsize: S\nrank: 1\n"),
    )
    .write(
        "p/tasks/T4-d.md",
        &item("T4", "a-tester", "release: '2.0'\nsize: S\nrank: 0\n"),
    )
    .write(
        "p/tasks/T5-e.md",
        &item("T5", "in_progress", "release: '1.0'\nsize: S\nrank: 9\n"),
    )
    .write(
        "p/tasks/T6-f.md",
        &item(
            "T6",
            "in_progress",
            "release: '1.0'\nsize: S\nrank: 8\ndepends_on: [T2]\n",
        ),
    );
    let forecast = relative(&repo);
    assert_eq!(
        sequence(&forecast),
        ["p#T5", "p#T4", "p#T3", "p#T1", "p#T2", "p#T6"]
    );
    assert_eq!(hours(&release(&forecast, "2.0").summary.span), (2.0, 4.0));
    let first = release(&forecast, "1.0");
    let shown: Vec<&str> = first.epics[0].stories[0]
        .work
        .iter()
        .map(|work| work.reference.as_str())
        .collect();
    assert_eq!(shown, ["p#T1", "p#T2", "p#T6", "p#T5"]);
}

#[test]
fn finished_work_sits_at_its_closing_date_and_takes_no_hours() {
    let repo = repo(NODE);
    repo.write(
        "p/tasks/T1-a.md",
        &item(
            "T1",
            "done",
            "release: '1.0'\nsize: S\nclosed: 2026-11-02\n",
        ),
    )
    .write(
        "p/tasks/T2-b.md",
        &item("T2", "done", "release: '1.0'\nsize: S\n"),
    )
    .write(
        "p/tasks/T3-c.md",
        &item("T3", "done", "release: '1.0'\nsize: S\n"),
    )
    .write(
        "p/tasks/T4-d.md",
        &item("T4", "todo", "release: '1.0'\nsize: S\n"),
    );
    let tree = Tree::load(repo.root()).unwrap();
    let transition = |date: &str, status: &str| Transition {
        date: date.to_owned(),
        status: Some(status.to_owned()),
    };
    let history = History {
        updated: HashMap::new(),
        statuses: HashMap::from([(
            PathBuf::from("tree/p/tasks/T2-b.md"),
            vec![
                transition("2026-10-01", "todo"),
                transition("2026-10-09", "done"),
            ],
        )]),
    };
    let forecast = build(&tree, None, Some(&history), &NodePath::new("p"), monday());
    assert_eq!(sequence(&forecast), ["p#T4"]);
    assert_eq!(moment(&work(&forecast, "p#T4").start), (0.0, None));
    let exact = work(&forecast, "p#T1");
    assert_eq!(
        (exact.stage, exact.sequence, exact.hours),
        (Stage::Done, None, 2.0)
    );
    assert_eq!(
        exact.closed,
        Some(ClosedDate {
            date: "2026-11-02".to_owned(),
            approximate: false
        })
    );
    assert_eq!(
        work(&forecast, "p#T2").closed,
        Some(ClosedDate {
            date: "2026-10-09".to_owned(),
            approximate: true
        })
    );
    assert_eq!(work(&forecast, "p#T3").closed, None);
    let summary = &release(&forecast, "1.0").summary;
    assert_eq!((summary.done, summary.open, summary.hours), (3, 1, 2.0));
    assert_eq!(
        summary.closed,
        Some(DateRange {
            from: "2026-10-09".to_owned(),
            to: "2026-11-02".to_owned()
        })
    );
}

#[test]
fn the_calendar_dates_the_work_block_by_block_with_focus_and_days_off() {
    let node =
        tested_by("owner").replace("name = \"1.0\"\n", "name = \"1.0\"\ntarget = 2026-12-01\n");
    let repo = repo(&node);
    repo.write(
        "p/tasks/T1-a.md",
        &item("T1", "todo", "release: '1.0'\nsize: S\nrank: 1\n"),
    )
    .write(
        "p/tasks/T2-b.md",
        &item(
            "T2",
            "todo",
            "release: '1.0'\nsize: XS\nrank: 2\ntarget: 2026-12-01\n",
        ),
    );
    let forecast = forecast_of(&repo, "p", Some(PARIS), monday());
    assert!(forecast.dated);
    assert_eq!(forecast.timezone.as_deref(), Some("Europe/Paris"));
    assert_eq!(forecast.now, "2026-11-16T08:00:00+01:00");
    assert_eq!(forecast.carrier, Some(NodePath::new("p")));
    let segments: Vec<(Phase, &str, &str)> = work(&forecast, "p#T1")
        .segments
        .iter()
        .map(|segment| {
            (
                segment.phase,
                segment.start.at.as_deref().unwrap(),
                segment.end.at.as_deref().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        segments,
        [
            (
                Phase::Work,
                "2026-11-16T09:00:00+01:00",
                "2026-11-16T13:00:00+01:00"
            ),
            (
                Phase::Test,
                "2026-11-30T09:00:00+01:00",
                "2026-11-30T11:00:00+01:00"
            ),
        ]
    );
    let second = work(&forecast, "p#T2");
    assert_eq!(
        moment(&second.start),
        (3.0, Some("2026-11-30T11:00:00+01:00"))
    );
    assert_eq!(
        moment(&second.delivered),
        (4.5, Some("2026-12-07T10:00:00+01:00"))
    );
    assert_eq!(second.segments.len(), 2);
    let missed: Vec<(String, Option<String>)> = forecast
        .warnings
        .iter()
        .find(|warning| warning.kind == WarningKind::MissedTarget)
        .unwrap()
        .items
        .iter()
        .map(|item| (item.reference.clone(), item.detail.clone()))
        .collect();
    let detail = Some("target 2026-12-01, forecast 2026-12-07".to_owned());
    assert_eq!(
        missed,
        [
            ("p#T2".to_owned(), detail.clone()),
            ("release 1.0".to_owned(), detail),
        ]
    );
}

#[test]
fn blocks_are_read_in_the_calendars_time_zone_from_now_on() {
    let repo = repo(NODE);
    repo.write(
        "p/tasks/T1-a.md",
        &item("T1", "todo", "release: '1.0'\nsize: XS\n"),
    );
    let calendar = "version = 1\ntimezone = \"America/New_York\"\n\n[node.\"p\"]\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"10:00\" }]\n";
    let forecast = forecast_of(&repo, "p", Some(calendar), at("2026-11-16T14:30:00Z"));
    let pieces: Vec<(&str, &str)> = work(&forecast, "p#T1")
        .segments
        .iter()
        .map(|segment| {
            (
                segment.start.at.as_deref().unwrap(),
                segment.end.at.as_deref().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        pieces,
        [
            ("2026-11-16T09:30:00-05:00", "2026-11-16T10:00:00-05:00"),
            ("2026-11-23T09:00:00-05:00", "2026-11-23T09:30:00-05:00"),
        ]
    );
}

#[test]
fn the_carrier_pools_the_work_of_its_descendants_in_one_sequence() {
    let repo = repo(NODE);
    repo.write(
        "p/tasks/T1-a.md",
        &item("T1", "todo", "release: '2.0'\nsize: S\n"),
    )
    .write(
        "p/q/tasks/T1-a.md",
        &item_of("p/q", "T1", "todo", "release: '1.0'\nsize: S\n"),
    )
    .write("p/r/node.toml", "[[release]]\nname = \"1.0\"\n")
    .write(
        "p/r/tasks/T1-a.md",
        &item_of("p/r", "T1", "todo", "release: '1.0'\nsize: S\n"),
    );
    let forecast = forecast_of(&repo, "p", Some(PARIS), monday());
    assert_eq!(forecast.node, NodePath::new("p"));
    let releases: Vec<(&str, &str)> = forecast
        .releases
        .iter()
        .map(|release| (release.name.as_str(), release.declared_by.as_str()))
        .collect();
    assert_eq!(releases, [("1.0", "p"), ("1.0", "p/r"), ("2.0", "p")]);
    assert_eq!(sequence(&forecast), ["p/q#T1", "p/r#T1", "p#T1"]);
    let alone = forecast_of(&repo, "p/q", None, monday());
    assert_eq!((alone.node.as_str(), alone.carrier.as_ref()), ("p/q", None));
    assert_eq!(sequence(&alone), ["p/q#T1"]);
}

#[test]
fn a_child_node_reads_its_part_of_the_carriers_sequence() {
    let node = NODE.replace("name = \"2.0\"\n", "name = \"2.0\"\ntarget = 2026-11-20\n");
    let repo = repo(&node);
    repo.write(
        "p/tasks/T1-a.md",
        &item("T1", "todo", "release: '1.0'\nsize: S\n"),
    )
    .write(
        "p/tasks/T2-b.md",
        &item("T2", "todo", "release: '2.0'\nsize: XS\n"),
    )
    .write("p/r/node.toml", "[[release]]\nname = \"1.0\"\n")
    .write(
        "p/r/tasks/T1-a.md",
        &item_of("p/r", "T1", "todo", "release: '1.0'\n"),
    )
    .write(
        "p/q/tasks/T1-a.md",
        &item_of("p/q", "T1", "todo", "release: '2.0'\nsize: S\n"),
    )
    .write(
        "p/q/tasks/T2-b.md",
        &item_of(
            "p/q",
            "T2",
            "done",
            "release: '1.0'\nsize: XS\nclosed: 2026-11-02\n",
        ),
    );
    let pooled = forecast_of(&repo, "p", Some(PARIS), monday());
    assert_eq!(sequence(&pooled), ["p#T1", "p/r#T1", "p#T2", "p/q#T1"]);
    let child = forecast_of(&repo, "p/q", Some(PARIS), monday());
    assert_eq!(child.node, NodePath::new("p/q"));
    assert_eq!(child.carrier, Some(NodePath::new("p")));
    let releases: Vec<(&str, &str)> = child
        .releases
        .iter()
        .map(|release| (release.name.as_str(), release.declared_by.as_str()))
        .collect();
    assert_eq!(releases, [("1.0", "p"), ("2.0", "p")]);
    assert_eq!(sequence(&child), ["p/q#T1"]);
    let own = work(&child, "p/q#T1");
    assert_eq!(own, work(&pooled, "p/q#T1"));
    assert_eq!(own.sequence, Some(3));
    let first = release(&child, "1.0");
    assert_eq!((first.summary.done, first.summary.open), (1, 0));
    assert_eq!(
        (first.summary.hours, first.summary.span.as_ref()),
        (0.0, None)
    );
    let second = &release(&child, "2.0").summary;
    assert_eq!((second.hours, hours(&second.span)), (2.0, (7.0, 9.0)));
    assert_eq!(release(&pooled, "2.0").summary.hours, 3.0);
    assert_eq!((child.hours, pooled.hours), (2.0, 9.0));
    assert_eq!(flagged(&child, WarningKind::MissedTarget), ["release 2.0"]);
    assert_eq!(flagged(&child, WarningKind::WithoutStory), ["p/q#T1"]);
    assert_eq!(flagged(&pooled, WarningKind::Unsized), ["p/r#T1"]);
    assert_eq!(child.warnings.len(), 2, "{:?}", child.warnings);
    assert!(child.unscheduled.is_empty());
}

#[test]
fn warnings_count_guesses_gaps_in_the_hierarchy_and_targets() {
    let node = NODE.replace("name = \"1.0\"\n", "name = \"1.0\"\ntarget = 2026-12-31\n");
    let repo = repo(&node);
    repo.write(
        "p/epics/E1-a.md",
        &item("E1", "todo", "release: '1.0'\ntarget: 2026-12-01\n"),
    )
    .write(
        "p/stories/S1-a.md",
        &item("S1", "todo", "release: '1.0'\ntarget: 2027-01-01\n"),
    )
    .write(
        "p/stories/S3-b.md",
        &item("S3", "todo", "parent: E1\ntarget: 2026-12-15\n"),
    )
    .write("p/tasks/T1-a.md", &item("T1", "todo", "release: '1.0'\n"))
    .write(
        "p/tasks/T2-b.md",
        &item("T2", "todo", "parent: S1\nsize: S\ntarget: 2027-01-10\n"),
    )
    .write(
        "p/tasks/T3-c.md",
        &item("T3", "todo", "parent: S3\nsize: S\n"),
    )
    .write(
        "p/tasks/T4-d.md",
        &item(
            "T4",
            "done",
            "release: '1.0'\nsize: S\nclosed: 2026-11-10\ntarget: 2026-11-01\n",
        ),
    );
    let forecast = relative(&repo);
    assert_eq!(flagged(&forecast, WarningKind::Unsized), ["p#T1"]);
    assert_eq!(flagged(&forecast, WarningKind::WithoutStory), ["p#T1"]);
    assert_eq!(flagged(&forecast, WarningKind::WithoutEpic), ["p#S1"]);
    assert_eq!(flagged(&forecast, WarningKind::MissedTarget), ["p#T4"]);
    assert_eq!(
        flagged(&forecast, WarningKind::ContradictoryTarget),
        ["p#T2", "p#S3"]
    );
    let kinds: Vec<WarningKind> = forecast
        .warnings
        .iter()
        .map(|warning| warning.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            WarningKind::Unsized,
            WarningKind::WithoutStory,
            WarningKind::WithoutEpic,
            WarningKind::MissedTarget,
            WarningKind::ContradictoryTarget,
        ]
    );
    let json = serde_json::to_value(&forecast).unwrap();
    assert_eq!(json["releases"][0]["name"], "1.0");
    assert!(json["releases"][0]["span"].is_object());
    assert_eq!(json["warnings"][0]["kind"], "unsized");
}

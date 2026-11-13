//! Estimates in hours of focused work (SPEC.md §1.5, ADR-0025).
//!
//! An item keeps its `size`, `XS` to `XL`; the node says what each size costs.
//! The `[estimates]` table of a `node.toml` gives the hours of the five sizes,
//! the `default` size of an item without one, the `test_ratio` of testing added
//! after each piece of work, and who tests (`tester`). It is inherited key by
//! key, the nearest declaration winning, over the values of ADR-0025.
//!
//! This module is the one place where an item's hours are computed, for
//! `ariane tasks --json`, `ariane show --json` and the interface:
//!
//! - a task weighs the hours of its `size`, else of the default size, assumed;
//! - a story with tasks weighs the sum of its tasks, its own `size` ignored;
//!   without tasks, its provisional `size`, else the default size, assumed;
//! - an epic with stories weighs the sum of its stories, each weighed as
//!   above; without stories, its provisional `size`, else the default size.
//!
//! Dropped work leaves both sums; finished work leaves the remaining hours only.
//! Work in a status of the `testing` stage has done its hours: only its
//! testing remains.
//! A story or an epic whose children are all dropped is split all the same and
//! weighs zero hours, never its provisional or default size.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::item::Kind;
use crate::node::{Category, Stage};
use crate::schema::SIZES;
use crate::tree::{Item, Tree, parse_reference};

/// Hours of `XS`, `S`, `M`, `L` and `XL` when no node declares them.
pub const DEFAULT_HOURS: [f64; 5] = [2.0, 4.0, 12.0, 24.0, 48.0];
/// The size of an unsized item when no node declares one.
pub const DEFAULT_SIZE: &str = "M";
/// The share of testing added after each piece of work when no node declares one.
pub const DEFAULT_TEST_RATIO: f64 = 0.25;

/// Who tests a piece of work once it is done (ADR-0025 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tester {
    /// The owner tests: the testing consumes the owner's hours.
    #[default]
    Owner,
    /// Someone else validates: the testing is a delay, not the owner's hours.
    Others,
}

impl Tester {
    /// The value written in `node.toml` and JSON.
    pub fn id(self) -> &'static str {
        match self {
            Tester::Owner => "owner",
            Tester::Others => "others",
        }
    }
}

/// Every value `tester` accepts.
pub const TESTERS: [Tester; 2] = [Tester::Owner, Tester::Others];

/// The `[estimates]` table of one `node.toml`, as written: every key optional,
/// each one overriding the inherited value. `deny_unknown_fields` refuses a key
/// that is neither one of the five sizes nor a known key, such as `XXL`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    /// Hours of an `XS`.
    #[serde(rename = "XS", default, deserialize_with = "hours")]
    pub xs: Option<f64>,
    /// Hours of an `S`.
    #[serde(rename = "S", default, deserialize_with = "hours")]
    pub s: Option<f64>,
    /// Hours of an `M`.
    #[serde(rename = "M", default, deserialize_with = "hours")]
    pub m: Option<f64>,
    /// Hours of an `L`.
    #[serde(rename = "L", default, deserialize_with = "hours")]
    pub l: Option<f64>,
    /// Hours of an `XL`.
    #[serde(rename = "XL", default, deserialize_with = "hours")]
    pub xl: Option<f64>,
    /// The size an item without `size` counts as.
    #[serde(default, deserialize_with = "size")]
    pub default: Option<String>,
    /// Hours of testing added per hour of work.
    #[serde(default, deserialize_with = "ratio")]
    pub test_ratio: Option<f64>,
    /// Who tests.
    #[serde(default)]
    pub tester: Option<Tester>,
}

impl Declaration {
    /// The hours this table declares for the five sizes, smallest first.
    fn hours(&self) -> [Option<f64>; 5] {
        [self.xs, self.s, self.m, self.l, self.xl]
    }
}

/// Reads a number of TOML, integer or float, refusing anything else.
fn number<'de, D: Deserializer<'de>>(deserializer: D, key: &str) -> Result<f64, D::Error> {
    match toml::Value::deserialize(deserializer)? {
        toml::Value::Integer(value) => Ok(value as f64),
        toml::Value::Float(value) if value.is_finite() => Ok(value),
        other => Err(D::Error::custom(format!(
            "`{key} = {other}` is not a number"
        ))),
    }
}

/// Reads the hours of a size: a number above zero.
fn hours<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<f64>, D::Error> {
    let value = number(deserializer, "hours")?;
    if value <= 0.0 {
        return Err(D::Error::custom(format!(
            "hours must be above zero, not {value}"
        )));
    }
    Ok(Some(value))
}

/// Reads `test_ratio`: a number, zero or above.
fn ratio<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<f64>, D::Error> {
    let value = number(deserializer, "test_ratio")?;
    if value < 0.0 {
        return Err(D::Error::custom(format!(
            "`test_ratio = {value}` is negative"
        )));
    }
    Ok(Some(value))
}

/// Reads `default`: one of the five sizes.
fn size<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    let text = String::deserialize(deserializer)?;
    if known_size(&text).is_none() {
        return Err(D::Error::custom(format!(
            "`default = \"{text}\"` is not one of the sizes {}",
            SIZES.join(", ")
        )));
    }
    Ok(Some(text))
}

/// The size `text` names, as the schema spells it; `None` when it is not one
/// of the five.
pub fn known_size(text: &str) -> Option<&'static str> {
    SIZES.iter().find(|size| **size == text).copied()
}

/// What sizes cost on one node, once the `[estimates]` tables of its ancestors
/// are combined key by key.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Pricing {
    /// Hours of an `XS`.
    #[serde(rename = "XS")]
    pub xs: f64,
    /// Hours of an `S`.
    #[serde(rename = "S")]
    pub s: f64,
    /// Hours of an `M`.
    #[serde(rename = "M")]
    pub m: f64,
    /// Hours of an `L`.
    #[serde(rename = "L")]
    pub l: f64,
    /// Hours of an `XL`.
    #[serde(rename = "XL")]
    pub xl: f64,
    /// The size an item without `size` counts as.
    pub default: &'static str,
    /// Hours of testing added per hour of work.
    pub test_ratio: f64,
    /// Who tests.
    pub tester: Tester,
}

impl Default for Pricing {
    /// The values of ADR-0025, for a node with no `[estimates]` above it.
    fn default() -> Pricing {
        let [xs, s, m, l, xl] = DEFAULT_HOURS;
        Pricing {
            xs,
            s,
            m,
            l,
            xl,
            default: DEFAULT_SIZE,
            test_ratio: DEFAULT_TEST_RATIO,
            tester: Tester::Owner,
        }
    }
}

impl Pricing {
    /// Overrides the keys `declaration` gives, keeping the others.
    pub fn apply(&mut self, declaration: &Declaration) {
        let declared = declaration.hours();
        let slots = [
            &mut self.xs,
            &mut self.s,
            &mut self.m,
            &mut self.l,
            &mut self.xl,
        ];
        for (slot, value) in slots.into_iter().zip(declared) {
            if let Some(value) = value {
                *slot = value;
            }
        }
        if let Some(default) = declaration.default.as_deref().and_then(known_size) {
            self.default = default;
        }
        if let Some(ratio) = declaration.test_ratio {
            self.test_ratio = ratio;
        }
        if let Some(tester) = declaration.tester {
            self.tester = tester;
        }
    }

    /// The hours of `size`, `None` when it is not one of the five sizes.
    pub fn hours_of(&self, size: &str) -> Option<f64> {
        match size {
            "XS" => Some(self.xs),
            "S" => Some(self.s),
            "M" => Some(self.m),
            "L" => Some(self.l),
            "XL" => Some(self.xl),
            _ => None,
        }
    }

    /// The hours of the default size.
    pub fn default_hours(&self) -> f64 {
        self.hours_of(self.default).unwrap_or(self.m)
    }
}

/// Where an item's hours come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// The task's own `size`.
    Size,
    /// The `size` of a story or an epic not split yet: a rough estimate.
    Provisional,
    /// No `size`: the node's default size, assumed.
    Default,
    /// The sum of the story's tasks, or of the epic's stories.
    Tasks,
}

impl Basis {
    /// Tells whether the hours are a guess: a default or a provisional size.
    pub fn is_guess(self) -> bool {
        matches!(self, Basis::Provisional | Basis::Default)
    }
}

/// The hours an item weighs, with its testing (ADR-0025).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Estimate {
    /// Hours of focused work, dropped work left out.
    pub hours: f64,
    /// The part of `hours` not finished yet: neither done nor dropped.
    pub remaining_hours: f64,
    /// Hours of testing added after the work, `hours × test_ratio`.
    pub test_hours: f64,
    /// The part of `test_hours` of the work not finished yet.
    pub remaining_test_hours: f64,
    /// Who tests, from the item's node.
    pub tester: Tester,
    /// Where the hours come from.
    pub basis: Basis,
    /// The size priced, for every basis but `tasks`.
    pub size: Option<&'static str>,
    /// How many single estimates the hours add up: 1 for a task or an unsplit
    /// story or epic, the count of its tasks and unsplit stories for a sum.
    pub parts: usize,
    /// How many of those parts are guesses: a default or provisional size.
    pub guessed: usize,
}

/// The hours `item` weighs; `None` for a kind that carries no work, such as a
/// reflexion or a ressource.
pub fn effective(tree: &Tree, item: &Item) -> Option<Estimate> {
    let pricing = tree.configs.pricing(&item.node);
    match item.kind {
        Kind::Task => Some(single(tree, item, &pricing, Basis::Size)),
        Kind::Story => Some(split(tree, item, &pricing, Kind::Task)),
        Kind::Epic => Some(split(tree, item, &pricing, Kind::Story)),
        _ => None,
    }
}

/// A story or an epic: the sum of its live children of kind `part` when it has
/// children, zero hours when all of them are dropped, else its own size as a
/// provisional estimate.
fn split(tree: &Tree, item: &Item, pricing: &Pricing, part: Kind) -> Estimate {
    let all: Vec<&Item> = children(tree, item, part).collect();
    if all.is_empty() {
        return single(tree, item, pricing, Basis::Provisional);
    }
    let children = all
        .into_iter()
        .filter(|child| category(tree, child) != Some(Category::Dropped));
    let mut sum = Estimate {
        hours: 0.0,
        remaining_hours: 0.0,
        test_hours: 0.0,
        remaining_test_hours: 0.0,
        tester: pricing.tester,
        basis: Basis::Tasks,
        size: None,
        parts: 0,
        guessed: 0,
    };
    for child in children {
        let Some(estimate) = effective(tree, child) else {
            continue;
        };
        sum.hours += estimate.hours;
        sum.remaining_hours += estimate.remaining_hours;
        sum.test_hours += estimate.test_hours;
        sum.remaining_test_hours += estimate.remaining_test_hours;
        sum.parts += estimate.parts;
        sum.guessed += estimate.guessed;
    }
    sum.hours = round(sum.hours);
    sum.remaining_hours = round(sum.remaining_hours);
    sum.test_hours = round(sum.test_hours);
    sum.remaining_test_hours = round(sum.remaining_test_hours);
    sum
}

/// One estimate priced from the item's own `size`, `sized` naming that basis,
/// else from the node's default size; finished work has nothing remaining,
/// work in testing only its testing.
fn single(tree: &Tree, item: &Item, pricing: &Pricing, sized: Basis) -> Estimate {
    let own = item.document.text("size").and_then(known_size);
    let (size, basis) = match own {
        Some(size) => (size, sized),
        None => (pricing.default, Basis::Default),
    };
    let hours = pricing
        .hours_of(size)
        .unwrap_or_else(|| pricing.default_hours());
    let finished = category(tree, item).is_some_and(Category::is_closed);
    let testing = stage(tree, item) == Some(Stage::Testing);
    let remaining = if finished || testing { 0.0 } else { hours };
    let remaining_test = if finished { 0.0 } else { hours };
    Estimate {
        hours: round(hours),
        remaining_hours: round(remaining),
        test_hours: round(hours * pricing.test_ratio),
        remaining_test_hours: round(remaining_test * pricing.test_ratio),
        tester: pricing.tester,
        basis,
        size: Some(size),
        parts: 1,
        guessed: usize::from(basis.is_guess()),
    }
}

/// The items of kind `part` whose `parent` is `item`, archived ones included.
fn children<'a>(tree: &'a Tree, item: &'a Item, part: Kind) -> impl Iterator<Item = &'a Item> {
    let id = item.id();
    tree.items.iter().filter(move |candidate| {
        candidate.kind == part
            && id.is_some()
            && candidate
                .document
                .text("parent")
                .and_then(|raw| parse_reference(raw, &candidate.node))
                .is_some_and(|(node, parent)| node == item.node && Some(parent.as_str()) == id)
    })
}

/// The category of `item`'s status in its node's workflow, if known.
fn category(tree: &Tree, item: &Item) -> Option<Category> {
    tree.configs
        .category(&item.node, item.document.text("status")?)
}

/// The stage of `item`'s status in its node's workflow, if it declares one.
fn stage(tree: &Tree, item: &Item) -> Option<Stage> {
    tree.configs
        .stage(&item.node, item.document.text("status")?)
}

/// `value` rounded to the hundredth, so that `12 × 0.1` reads `1.2`.
fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// Unit tests of the hours an item weighs.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// An item of node `p` with `id`, `status` and extra header lines.
    fn item(id: &str, status: &str, extra: &str) -> String {
        format!(
            "---\nariane: 2\nnode: p\nid: {id}\ntitle: {id}\nstatus: {status}\n{extra}---\n# {id}\n"
        )
    }

    /// The estimate of item `id` of `tree`.
    fn of(tree: &Tree, id: &str) -> Estimate {
        let item = tree
            .items
            .iter()
            .find(|item| item.id() == Some(id))
            .unwrap();
        effective(tree, item).unwrap()
    }

    /// A node pricing `M` at 10 hours with a test ratio of 0.5: an epic with a
    /// split story (sized, unsized, done and dropped tasks, its own size
    /// ignored) and an unsplit story with a provisional size; an unsplit epic
    /// without size; a reflexion.
    fn sample() -> Tree {
        let repo = Repo::new();
        repo.write(
            "p/node.toml",
            "[estimates]\nM = 10\ntest_ratio = 0.5\ntester = \"others\"\n",
        )
        .write("p/epics/E1-e.md", &item("E1", "todo", "size: XS\n"))
        .write("p/epics/E2-e.md", &item("E2", "todo", ""))
        .write(
            "p/stories/S1-a.md",
            &item("S1", "todo", "parent: E1\nsize: XL\n"),
        )
        .write(
            "p/stories/S2-b.md",
            &item("S2", "todo", "parent: E1\nsize: L\n"),
        )
        .write(
            "p/stories/S3-c.md",
            &item("S3", "dropped", "parent: E1\nsize: XL\n"),
        )
        .write(
            "p/tasks/T1-sized.md",
            &item("T1", "todo", "parent: S1\nsize: S\n"),
        )
        .write(
            "p/tasks/T2-unsized.md",
            &item("T2", "in_progress", "parent: S1\n"),
        )
        .write(
            "p/archives/tasks/T3-done.md",
            &item("T3", "done", "parent: S1\nsize: XS\n"),
        )
        .write(
            "p/tasks/T4-dropped.md",
            &item("T4", "dropped", "parent: S1\nsize: XL\n"),
        )
        .write("p/tasks/T5-odd.md", &item("T5", "todo", "size: XXL\n"))
        .write("p/reflexions/R1-r.md", &item("R1", "open", ""));
        Tree::load(repo.root()).unwrap()
    }

    #[test]
    fn a_task_weighs_its_size_else_the_default_size_assumed() {
        let tree = sample();
        let sized = of(&tree, "T1");
        assert_eq!(
            (sized.basis, sized.size, sized.hours),
            (Basis::Size, Some("S"), 4.0)
        );
        assert_eq!((sized.test_hours, sized.tester), (2.0, Tester::Others));
        assert_eq!((sized.parts, sized.guessed), (1, 0));
        let bare = of(&tree, "T2");
        assert_eq!(
            (bare.basis, bare.size, bare.hours),
            (Basis::Default, Some("M"), 10.0)
        );
        assert_eq!(bare.guessed, 1);
        let odd = of(&tree, "T5");
        assert_eq!((odd.basis, odd.hours), (Basis::Default, 10.0));
    }

    #[test]
    fn finished_work_has_hours_but_nothing_remaining() {
        let tree = sample();
        let done = of(&tree, "T3");
        assert_eq!((done.hours, done.remaining_hours), (2.0, 0.0));
        assert_eq!((done.test_hours, done.remaining_test_hours), (1.0, 0.0));
        assert_eq!(of(&tree, "T4").remaining_hours, 0.0);
    }

    #[test]
    fn a_split_story_weighs_the_sum_of_its_live_tasks_and_ignores_its_own_size() {
        let story = of(&sample(), "S1");
        assert_eq!((story.basis, story.size), (Basis::Tasks, None));
        assert_eq!(story.hours, 4.0 + 10.0 + 2.0);
        assert_eq!(story.remaining_hours, 4.0 + 10.0);
        assert_eq!(story.test_hours, 8.0);
        assert_eq!(story.remaining_test_hours, 7.0);
        assert_eq!((story.parts, story.guessed), (3, 1));
    }

    #[test]
    fn an_unsplit_story_or_epic_weighs_its_provisional_size_else_the_default() {
        let tree = sample();
        let story = of(&tree, "S2");
        assert_eq!(
            (story.basis, story.size, story.hours),
            (Basis::Provisional, Some("L"), 24.0)
        );
        assert_eq!((story.parts, story.guessed), (1, 1));
        let epic = of(&tree, "E2");
        assert_eq!(
            (epic.basis, epic.size, epic.hours),
            (Basis::Default, Some("M"), 10.0)
        );
    }

    #[test]
    fn an_epic_weighs_the_sum_of_its_live_stories() {
        let epic = of(&sample(), "E1");
        assert_eq!(epic.basis, Basis::Tasks);
        assert_eq!(epic.hours, 16.0 + 24.0);
        assert_eq!(epic.remaining_hours, 14.0 + 24.0);
        assert_eq!((epic.parts, epic.guessed), (4, 2));
    }

    #[test]
    fn a_story_or_epic_whose_children_are_all_dropped_weighs_nothing() {
        let repo = Repo::new();
        repo.write("p/epics/E1-e.md", &item("E1", "todo", "size: XL\n"))
            .write(
                "p/stories/S1-a.md",
                &item("S1", "dropped", "parent: E1\nsize: L\n"),
            )
            .write("p/stories/S2-b.md", &item("S2", "todo", "size: L\n"))
            .write(
                "p/tasks/T1-a.md",
                &item("T1", "dropped", "parent: S2\nsize: S\n"),
            );
        let tree = Tree::load(repo.root()).unwrap();
        for id in ["E1", "S2"] {
            let estimate = of(&tree, id);
            assert_eq!((estimate.basis, estimate.size), (Basis::Tasks, None));
            assert_eq!((estimate.hours, estimate.remaining_hours), (0.0, 0.0));
            assert_eq!((estimate.parts, estimate.guessed), (0, 0));
        }
    }

    #[test]
    fn work_in_testing_has_only_its_testing_remaining() {
        let repo = Repo::new();
        repo.write(
            "p/node.toml",
            "[estimates]\nM = 10\ntest_ratio = 0.5\n\n[[workflow]]\nid = \"en-cours\"\nlabel = \"En cours\"\ncategory = \"in_progress\"\n\n[[workflow]]\nid = \"a-tester\"\nlabel = \"À tester\"\ncategory = \"in_progress\"\nstage = \"testing\"\n",
        )
        .write("p/stories/S1-a.md", &item("S1", "en-cours", ""))
        .write(
            "p/tasks/T1-a.md",
            &item("T1", "a-tester", "parent: S1\nsize: M\n"),
        )
        .write(
            "p/tasks/T2-b.md",
            &item("T2", "en-cours", "parent: S1\nsize: M\n"),
        );
        let tree = Tree::load(repo.root()).unwrap();
        let testing = of(&tree, "T1");
        assert_eq!((testing.hours, testing.remaining_hours), (10.0, 0.0));
        assert_eq!(
            (testing.test_hours, testing.remaining_test_hours),
            (5.0, 5.0)
        );
        let story = of(&tree, "S1");
        assert_eq!(
            (story.remaining_hours, story.remaining_test_hours),
            (10.0, 10.0)
        );
    }

    #[test]
    fn a_reflexion_weighs_nothing() {
        let tree = sample();
        let reflexion = tree
            .items
            .iter()
            .find(|item| item.id() == Some("R1"))
            .unwrap();
        assert!(effective(&tree, reflexion).is_none());
    }

    #[test]
    fn each_task_is_priced_by_its_own_node_and_hours_are_rounded() {
        let repo = Repo::new();
        repo.write("p/node.toml", "[estimates]\ntest_ratio = 0.1\n")
            .write("p/stories/S1-a.md", &item("S1", "todo", ""))
            .write("p/q/node.toml", "[estimates]\nS = 3\n")
            .write(
                "p/q/tasks/T1-a.md",
                "---\nariane: 2\nnode: p/q\nid: T1\ntitle: T1\nstatus: todo\nparent: p#S1\nsize: S\n---\n",
            );
        let tree = Tree::load(repo.root()).unwrap();
        let story = of(&tree, "S1");
        assert_eq!(story.hours, 3.0);
        assert_eq!(story.test_hours, 0.3);
    }
}

//! Writing `calendar.toml` from `ariane calendar set|unset` and `ariane serve`
//! (SPEC.md §2, "Writing the calendar").
//!
//! A change sets the blocks, the focus or the days off of one node, removes a
//! node's table, or sets the time zone and the days off of every node. It is
//! applied to the file through `toml_edit`, so comments, order and the other
//! tables are kept: only the keys the change names differ. A missing file
//! starts from [`super::template`].
//!
//! The edited text is judged by [`super::check`], as `doctor` and `ariane
//! calendar edit` judge it, and written only when it has no error: an overlap,
//! a bad hour, day, date or time zone, a focus out of range refuse the whole
//! change and leave the file as it was. Warnings, such as a table without
//! blocks or one naming a node the tree no longer has, do not stop it and are
//! handed back. A new table for a node the tree does not have is refused: it
//! would only be a typo; an existing one can still be changed or removed.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Key, Table, TableLike, Value};

use super::{FILE, Problem, check, known_node, path, template};
use crate::cli::CalendarSetArgs;
use crate::node::set_value;
use crate::tree::{NodePath, Tree};

/// One weekly block as the owner writes it, checked only once applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockSpec {
    /// The days, such as `mon-fri` or `tue,thu`.
    pub days: String,
    /// When it starts, `HH:MM`.
    pub from: String,
    /// When it ends, `HH:MM`.
    pub to: String,
}

/// What a change does to one node's table; `None` leaves a key as it is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeChange {
    /// The new list of blocks, replacing the old one; empty removes it.
    pub blocks: Option<Vec<BlockSpec>>,
    /// The new focus; `Some(None)` removes it, back to the default.
    pub focus: Option<Option<f64>>,
    /// The node's own days off, replacing the old ones; empty removes them.
    pub days_off: Option<Vec<String>>,
}

impl NodeChange {
    /// Tells whether the change sets nothing.
    fn is_empty(&self) -> bool {
        self.blocks.is_none() && self.focus.is_none() && self.days_off.is_none()
    }
}

/// One change of the calendar.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// Sets the time zone or the days off of every node.
    Globals {
        /// The IANA time zone of the blocks.
        timezone: Option<String>,
        /// The days off of every node, replacing the old ones; empty removes them.
        days_off: Option<Vec<String>>,
    },
    /// Changes the table of `node`, created when absent.
    Node {
        /// The node whose table changes.
        node: NodePath,
        /// What changes in it.
        change: NodeChange,
    },
    /// Removes the table of `node`.
    Remove {
        /// The node whose table goes.
        node: NodePath,
    },
}

/// Why a change was not written.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    /// The request itself is wrong: nothing to change, an unknown node, a
    /// table to remove that does not exist, a file that is not TOML.
    Invalid(String),
    /// The calendar the change would leave has these errors.
    Problems(Vec<Problem>),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Invalid(message) => f.write_str(message),
            Refusal::Problems(problems) => {
                let messages: Vec<&str> = problems
                    .iter()
                    .map(|problem| problem.message.as_str())
                    .collect();
                write!(f, "{FILE} not written: {}", messages.join("; "))
            }
        }
    }
}

/// What a change that went through did.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    /// The file written.
    pub file: PathBuf,
    /// Whether the change created it.
    pub created: bool,
    /// The warnings of the calendar as written.
    pub warnings: Vec<Problem>,
}

/// Applies `change` to the calendar of the personal repository `personal`,
/// whose nodes `tree` holds, and writes it when the result has no error.
///
/// # Errors
///
/// Fails when the file cannot be read or written; refuses what [`apply`]
/// refuses, and a result with an error, writing nothing.
pub fn write(personal: &Path, tree: &Tree, change: &Change) -> Result<Result<Written, Refusal>> {
    let file = path(personal);
    let existing = match std::fs::read_to_string(&file) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).with_context(|| format!("reading {}", file.display())),
    };
    let created = existing.is_none();
    let text = existing.unwrap_or_else(template);
    let known = |node: &NodePath| known_node(tree, node);
    let edited = match apply(&text, change, known) {
        Ok(edited) => edited,
        Err(refusal) => return Ok(Err(refusal)),
    };
    let checked = check(&edited, known);
    let errors: Vec<Problem> = checked.errors().cloned().collect();
    if !errors.is_empty() {
        return Ok(Err(Refusal::Problems(errors)));
    }
    std::fs::write(&file, &edited).with_context(|| format!("writing {}", file.display()))?;
    Ok(Ok(Written {
        file,
        created,
        warnings: checked
            .problems
            .into_iter()
            .filter(|problem| !problem.is_error())
            .collect(),
    }))
}

/// The text of the calendar `text` once `change` is applied, the rest kept;
/// `known` tells which node paths the tree has.
///
/// # Errors
///
/// Refuses a change that sets nothing, a new table for an unknown node, the
/// removal of a table that does not exist, and a text that is not TOML.
pub fn apply(
    text: &str,
    change: &Change,
    known: impl Fn(&NodePath) -> bool,
) -> Result<String, Refusal> {
    let mut document: DocumentMut = text
        .parse()
        .map_err(|error| Refusal::Invalid(format!("{FILE} is not TOML: {error}")))?;
    match change {
        Change::Globals { timezone, days_off } => {
            if timezone.is_none() && days_off.is_none() {
                return Err(nothing());
            }
            let root = document.as_table_mut();
            if let Some(timezone) = timezone {
                set_value(root, "timezone", Value::from(timezone.trim()));
            }
            if let Some(days_off) = days_off {
                put_list(root, "days_off", days_off);
            }
        }
        Change::Node { node, change } => {
            if change.is_empty() {
                return Err(nothing());
            }
            let entry = node_table(&mut document, node, &known)?;
            if let Some(blocks) = &change.blocks {
                if blocks.is_empty() {
                    entry.remove("blocks");
                } else {
                    set_value(entry, "blocks", blocks_value(blocks));
                }
            }
            match change.focus {
                Some(Some(focus)) => set_value(entry, "focus", Value::from(focus)),
                Some(None) => {
                    entry.remove("focus");
                }
                None => {}
            }
            if let Some(days_off) = &change.days_off {
                put_list(entry, "days_off", days_off);
            }
        }
        Change::Remove { node } => {
            let nodes = document
                .get_mut("node")
                .and_then(Item::as_table_like_mut)
                .ok_or_else(|| no_table(node))?;
            if nodes.remove(node.as_str()).is_none() {
                return Err(no_table(node));
            }
            if nodes.is_empty() {
                document.remove("node");
            }
        }
    }
    Ok(document.to_string())
}

/// The refusal of a change that sets nothing.
fn nothing() -> Refusal {
    Refusal::Invalid("nothing to change: give blocks, a focus, days off or a time zone".to_owned())
}

/// The refusal of the removal of a table that does not exist.
fn no_table(node: &NodePath) -> Refusal {
    Refusal::Invalid(format!("{FILE} has no table for `{node}`"))
}

/// The `[node."<path>"]` table of `node`, created after the others when
/// absent, if the tree has that node.
fn node_table<'a>(
    document: &'a mut DocumentMut,
    node: &NodePath,
    known: &impl Fn(&NodePath) -> bool,
) -> Result<&'a mut dyn TableLike, Refusal> {
    let not_tables = || Refusal::Invalid(format!("`node` of {FILE} is not a table of node tables"));
    if !document.contains_key("node") {
        let mut nodes = Table::new();
        nodes.set_implicit(true);
        document.insert("node", Item::Table(nodes));
    }
    let nodes = document.get_mut("node").ok_or_else(not_tables)?;
    let present = nodes
        .as_table_like()
        .ok_or_else(not_tables)?
        .contains_key(node.as_str());
    if !present {
        if !known(node) {
            return Err(Refusal::Invalid(format!(
                "`{node}` is not a node of the tree"
            )));
        }
        let Some(table) = nodes.as_table_mut() else {
            return Err(Refusal::Invalid(format!(
                "`node` of {FILE} is written inline: add `[node.\"{node}\"]` with `ariane calendar edit`"
            )));
        };
        table.insert_formatted(&quoted(node), Item::Table(Table::new()));
    }
    let nodes = nodes.as_table_like_mut().ok_or_else(not_tables)?;
    nodes
        .get_mut(node.as_str())
        .and_then(Item::as_table_like_mut)
        .ok_or_else(not_tables)
}

/// The key `"<path>"`, always quoted as the calendar writes node paths.
fn quoted(node: &NodePath) -> Key {
    let repr = Value::from(node.as_str()).to_string();
    Key::parse(repr.trim())
        .ok()
        .and_then(|mut keys| keys.pop())
        .unwrap_or_else(|| Key::new(node.as_str()))
}

/// Sets `key` of `table` to the list `entries`, or removes it when empty.
fn put_list(table: &mut dyn TableLike, key: &str, entries: &[String]) {
    if entries.is_empty() {
        table.remove(key);
        return;
    }
    let mut array: Array = entries.iter().map(|entry| entry.trim()).collect();
    array.fmt();
    set_value(table, key, Value::Array(array));
}

/// The blocks as an array of inline tables, one per line when there are
/// several.
fn blocks_value(blocks: &[BlockSpec]) -> Value {
    let mut array = Array::new();
    for block in blocks {
        let mut entry = InlineTable::new();
        entry.insert("days", Value::from(block.days.trim()));
        entry.insert("from", Value::from(block.from.trim()));
        entry.insert("to", Value::from(block.to.trim()));
        entry.fmt();
        array.push(entry);
    }
    if blocks.len() > 1 {
        for entry in array.iter_mut() {
            entry.decor_mut().set_prefix("\n  ");
            entry.decor_mut().set_suffix("");
        }
        array.set_trailing_comma(true);
        array.set_trailing("\n");
    } else {
        array.fmt();
    }
    Value::Array(array)
}

/// Reads blocks written `mon-fri 09:00-18:00; sat 10:00-13:00`: the days, then
/// the hours, blocks separated by `;`. Blank text is no block.
///
/// # Errors
///
/// Says which block is not written that way; the days and hours themselves
/// are checked once applied.
pub fn parse_blocks(text: &str) -> Result<Vec<BlockSpec>, String> {
    text.split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            let wrong = || {
                format!(
                    "`{part}` is not a block written `<days> <HH:MM>-<HH:MM>`, such as `mon-fri 09:00-18:00`"
                )
            };
            let (days, hours) = part.rsplit_once(char::is_whitespace).ok_or_else(wrong)?;
            let (from, to) = hours
                .split_once(['-', '–'])
                .ok_or_else(wrong)?;
            Ok(BlockSpec {
                days: days.trim().to_owned(),
                from: from.trim().to_owned(),
                to: to.trim().to_owned(),
            })
        })
        .collect()
}

/// Reads days off written `2026-12-24..2027-01-02, 2027-05-01`; blank text is
/// none. Each entry is checked once applied.
pub fn parse_days_off(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Runs `ariane calendar set` on the calendar of `personal`.
///
/// # Errors
///
/// As [`run_change`], and fails on blocks not written `<days> <HH:MM>-<HH:MM>`.
pub fn set_command(personal: &Path, tree: &Tree, args: &CalendarSetArgs) -> Result<ExitCode> {
    let days_off = args.days_off.as_deref().map(parse_days_off);
    let change = match &args.node {
        Some(node) => Change::Node {
            node: NodePath::new(node),
            change: NodeChange {
                blocks: match args.blocks.as_deref().map(parse_blocks).transpose() {
                    Ok(blocks) => blocks,
                    Err(reason) => bail!("{reason}"),
                },
                focus: args.focus(),
                days_off,
            },
        },
        None => Change::Globals {
            timezone: args.timezone.clone(),
            days_off,
        },
    };
    run_change(personal, tree, &change)
}

/// Writes `change` into the calendar of `personal`, prints its warnings and
/// what the calendar now gives the node changed, or the file and its time
/// zone for a global change.
///
/// # Errors
///
/// Fails when the file cannot be read or written, and when the change is
/// refused: every error of the result is printed first, and nothing is written.
pub fn run_change(personal: &Path, tree: &Tree, change: &Change) -> Result<ExitCode> {
    let written = match write(personal, tree, change)? {
        Ok(written) => written,
        Err(Refusal::Problems(problems)) => {
            for problem in &problems {
                eprintln!("ariane calendar: error: {}", problem.message);
            }
            bail!("{} not written", path(personal).display());
        }
        Err(refusal) => bail!("{refusal}"),
    };
    for warning in &written.warnings {
        eprintln!("ariane calendar: warning: {}", warning.message);
    }
    let verb = if written.created { "created" } else { "saved" };
    println!("{} {verb}", written.file.display());
    let calendar = super::overview(personal, tree)?.calendar;
    match (change, calendar) {
        (Change::Node { node, .. }, Some(calendar)) => {
            print!("{}", super::describe(&super::resolve(&calendar, node)));
        }
        (Change::Globals { .. }, Some(calendar)) => {
            print!(
                "timezone    {}\n{}",
                calendar.timezone,
                super::days_off_lines(&calendar.days_off, &[], None)
            );
        }
        _ => {}
    }
    Ok(ExitCode::SUCCESS)
}

/// Unit tests of the calendar writer.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::ProblemKind;
    use crate::tree::fixture::Repo;

    /// A calendar with comments, a global list and two node tables.
    const SAMPLE: &str = r#"# my week
version = 1
timezone = "Europe/Paris" # where I live
days_off = ["2026-12-24..2027-01-02"]

# the day job
[node."cmc"]
focus = 0.6
blocks = [{ days = "mon-fri", from = "09:00", to = "18:00" }]

# evenings
[node."slashome/pharmakon"]
blocks = [
  { days = "tue,thu", from = "20:00", to = "23:00" },
]
"#;

    /// Every node is known.
    fn any(_: &NodePath) -> bool {
        true
    }

    /// The change of `node`'s table.
    fn on(node: &str, change: NodeChange) -> Change {
        Change::Node {
            node: NodePath::new(node),
            change,
        }
    }

    /// A block.
    fn block(days: &str, from: &str, to: &str) -> BlockSpec {
        BlockSpec {
            days: days.to_owned(),
            from: from.to_owned(),
            to: to.to_owned(),
        }
    }

    #[test]
    fn setting_a_node_s_focus_changes_that_line_only() {
        let edited = apply(
            SAMPLE,
            &on(
                "cmc",
                NodeChange {
                    focus: Some(Some(0.8)),
                    ..NodeChange::default()
                },
            ),
            any,
        )
        .unwrap();
        assert_eq!(edited, SAMPLE.replace("focus = 0.6", "focus = 0.8"));
    }

    #[test]
    fn replacing_blocks_keeps_the_rest_of_the_file_byte_for_byte() {
        let edited = apply(
            SAMPLE,
            &on(
                "slashome/pharmakon",
                NodeChange {
                    blocks: Some(vec![
                        block("tue,thu", "20:00", "23:00"),
                        block("sat", "10:00", "13:00"),
                    ]),
                    ..NodeChange::default()
                },
            ),
            any,
        )
        .unwrap();
        let expected = SAMPLE.replace(
            "  { days = \"tue,thu\", from = \"20:00\", to = \"23:00\" },\n]",
            "  { days = \"tue,thu\", from = \"20:00\", to = \"23:00\" },\n  { days = \"sat\", from = \"10:00\", to = \"13:00\" },\n]",
        );
        assert_eq!(edited, expected);
        assert_eq!(check(&edited, any).problems, vec![]);
    }

    #[test]
    fn a_new_node_table_goes_last_with_its_path_quoted() {
        let edited = apply(
            SAMPLE,
            &on(
                "atlas",
                NodeChange {
                    blocks: Some(vec![block("sat", "14:00", "16:00")]),
                    focus: Some(Some(0.5)),
                    days_off: Some(vec!["2027-02-01".to_owned()]),
                },
            ),
            any,
        )
        .unwrap();
        assert!(edited.starts_with(SAMPLE), "{edited}");
        assert!(
            edited.ends_with(
                "[node.\"atlas\"]\nblocks = [{ days = \"sat\", from = \"14:00\", to = \"16:00\" }]\nfocus = 0.5\ndays_off = [\"2027-02-01\"]\n"
            ),
            "{edited}"
        );
        assert_eq!(check(&edited, any).problems, vec![]);
    }

    #[test]
    fn a_new_table_for_a_node_the_tree_lacks_is_refused_and_an_existing_one_is_not() {
        let unknown = apply(
            SAMPLE,
            &on(
                "typo",
                NodeChange {
                    focus: Some(Some(0.5)),
                    ..NodeChange::default()
                },
            ),
            |_| false,
        );
        assert_eq!(
            unknown,
            Err(Refusal::Invalid(
                "`typo` is not a node of the tree".to_owned()
            ))
        );
        assert!(
            apply(
                SAMPLE,
                &on(
                    "cmc",
                    NodeChange {
                        focus: Some(None),
                        ..NodeChange::default()
                    }
                ),
                |_| false
            )
            .is_ok()
        );
    }

    #[test]
    fn globals_are_set_in_place_and_an_empty_list_removes_its_key() {
        let edited = apply(
            SAMPLE,
            &Change::Globals {
                timezone: Some("America/New_York".to_owned()),
                days_off: Some(vec![]),
            },
            any,
        )
        .unwrap();
        assert_eq!(
            edited,
            SAMPLE
                .replace("Europe/Paris", "America/New_York")
                .replace("days_off = [\"2026-12-24..2027-01-02\"]\n", "")
        );
    }

    #[test]
    fn removing_a_table_takes_its_lines_and_a_missing_one_is_refused() {
        let edited = apply(
            SAMPLE,
            &Change::Remove {
                node: NodePath::new("slashome/pharmakon"),
            },
            any,
        )
        .unwrap();
        assert!(!edited.contains("pharmakon"), "{edited}");
        assert!(edited.contains("[node.\"cmc\"]"), "{edited}");
        assert!(matches!(
            apply(
                SAMPLE,
                &Change::Remove {
                    node: NodePath::new("atlas")
                },
                any
            ),
            Err(Refusal::Invalid(_))
        ));
        let last = apply(
            "version = 1\ntimezone = \"UTC\"\n[node.\"a\"]\nfocus = 1\n",
            &Change::Remove {
                node: NodePath::new("a"),
            },
            any,
        )
        .unwrap();
        assert_eq!(last, "version = 1\ntimezone = \"UTC\"\n");
    }

    #[test]
    fn a_change_that_sets_nothing_is_refused() {
        assert!(matches!(
            apply(SAMPLE, &on("cmc", NodeChange::default()), any),
            Err(Refusal::Invalid(_))
        ));
        assert!(matches!(
            apply(
                SAMPLE,
                &Change::Globals {
                    timezone: None,
                    days_off: None
                },
                any
            ),
            Err(Refusal::Invalid(_))
        ));
    }

    /// A repository with the nodes `a` and `b`, its tree, and a calendar
    /// holding `text` when given.
    fn repository(text: Option<&str>) -> (Repo, Tree) {
        let repo = Repo::new();
        repo.write("a/node.toml", "").write("b/node.toml", "");
        if let Some(text) = text {
            std::fs::write(path(repo.root()), text).unwrap();
        }
        let tree = Tree::load(repo.root()).unwrap();
        (repo, tree)
    }

    #[test]
    fn an_overlap_is_refused_with_both_blocks_and_nothing_is_written() {
        let original = "# mine\nversion = 1\ntimezone = \"UTC\"\n\n[node.\"a\"]\nblocks = [\n  { days = \"sat\", from = \"08:00\", to = \"09:00\" },\n  { days = \"mon-fri\", from = \"09:00\", to = \"12:00\" },\n]\n";
        let (repo, tree) = repository(Some(original));
        let refused = write(
            repo.root(),
            &tree,
            &on(
                "b",
                NodeChange {
                    blocks: Some(vec![block("wed", "11:00", "13:00")]),
                    ..NodeChange::default()
                },
            ),
        )
        .unwrap();
        let Err(Refusal::Problems(problems)) = refused else {
            panic!("expected a refusal, got {refused:?}");
        };
        assert_eq!(problems.len(), 1);
        let overlap = &problems[0];
        assert_eq!(overlap.kind, ProblemKind::Overlap);
        assert_eq!(
            (overlap.node.as_ref(), overlap.block),
            (Some(&NodePath::new("a")), Some(1))
        );
        assert_eq!(
            (overlap.other.as_ref(), overlap.other_block),
            (Some(&NodePath::new("b")), Some(0))
        );
        assert_eq!(
            std::fs::read_to_string(path(repo.root())).unwrap(),
            original
        );
    }

    #[test]
    fn an_invalid_value_is_refused_and_nothing_is_written() {
        let (repo, tree) = repository(None);
        for change in [
            on(
                "a",
                NodeChange {
                    focus: Some(Some(1.5)),
                    ..NodeChange::default()
                },
            ),
            on(
                "a",
                NodeChange {
                    blocks: Some(vec![block("mon", "18:00", "09:00")]),
                    ..NodeChange::default()
                },
            ),
            Change::Globals {
                timezone: Some("Mars/Olympus".to_owned()),
                days_off: None,
            },
            Change::Globals {
                timezone: None,
                days_off: Some(vec!["2026-13-01".to_owned()]),
            },
        ] {
            let outcome = write(repo.root(), &tree, &change).unwrap();
            assert!(
                matches!(outcome, Err(Refusal::Problems(_))),
                "{change:?}: {outcome:?}"
            );
            assert!(!path(repo.root()).exists(), "{change:?}");
        }
    }

    #[test]
    fn a_missing_file_is_created_from_the_template_and_warnings_come_back() {
        let (repo, tree) = repository(None);
        let written = write(
            repo.root(),
            &tree,
            &on(
                "a",
                NodeChange {
                    focus: Some(Some(0.5)),
                    ..NodeChange::default()
                },
            ),
        )
        .unwrap()
        .unwrap();
        assert!(written.created);
        assert_eq!(written.warnings.len(), 1);
        assert_eq!(written.warnings[0].kind, ProblemKind::Unused);
        let saved = std::fs::read_to_string(path(repo.root())).unwrap();
        assert!(saved.starts_with(&template()), "{saved}");
        assert!(saved.ends_with("[node.\"a\"]\nfocus = 0.5\n"), "{saved}");
    }

    #[test]
    fn an_existing_error_elsewhere_refuses_until_the_change_fixes_it() {
        let broken = "version = 1\ntimezone = \"UTC\"\n[node.\"a\"]\nblocks = [{ days = \"mon\", from = \"09:00\", to = \"12:00\" }]\n[node.\"b\"]\nblocks = [{ days = \"mon\", from = \"11:00\", to = \"13:00\" }]\n";
        let (repo, tree) = repository(Some(broken));
        let unrelated = Change::Globals {
            timezone: Some("Europe/Paris".to_owned()),
            days_off: None,
        };
        assert!(matches!(
            write(repo.root(), &tree, &unrelated).unwrap(),
            Err(Refusal::Problems(_))
        ));
        let fixed = write(
            repo.root(),
            &tree,
            &on(
                "b",
                NodeChange {
                    blocks: Some(vec![block("mon", "12:00", "13:00")]),
                    ..NodeChange::default()
                },
            ),
        )
        .unwrap();
        assert!(fixed.is_ok(), "{fixed:?}");
    }

    #[test]
    fn blocks_and_days_off_are_read_from_one_line() {
        assert_eq!(
            parse_blocks("mon-fri 09:00-18:00; sat 10:00–13:00 ;").unwrap(),
            vec![
                block("mon-fri", "09:00", "18:00"),
                block("sat", "10:00", "13:00")
            ]
        );
        assert_eq!(
            parse_blocks("mon, wed-fri 08:00-09:00").unwrap(),
            vec![block("mon, wed-fri", "08:00", "09:00")]
        );
        assert_eq!(parse_blocks("  ").unwrap(), vec![]);
        assert!(parse_blocks("mon-fri").is_err());
        assert!(parse_blocks("mon 09:00").is_err());
        assert_eq!(
            parse_days_off("2026-12-24..2027-01-02, 2027-05-01,"),
            vec!["2026-12-24..2027-01-02", "2027-05-01"]
        );
        assert_eq!(parse_days_off(""), Vec::<String>::new());
    }
}

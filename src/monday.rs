//! `ariane sync monday`: the monday connector, Ariane → monday (ADR-0011,
//! ADR-0015).
//!
//! A node bound to monday says so in its `[tracker]`: the `account`, the
//! `board`, the `status_column`, and three maps of label ids —
//! `[tracker.status]` (label → category, how monday reads in Ariane),
//! `[tracker.push]` (category → label, the label Ariane gives a category) and
//! `[tracker.waiting]` (`waiting_on` → label). Every item of such a node with
//! an `external: monday:<id>` is compared with its ticket:
//!
//! - in step when the ticket's label reads as the item's category (for a
//!   waiting item, when it is exactly the label its reason maps to);
//! - pushed when the item moved since the last sync and the ticket did not;
//!   entering a wait also posts an update that @mentions the people awaited;
//! - a **conflict** when both moved, or when they disagree and no sync has
//!   recorded what the ticket was: the connector refuses to guess and lists
//!   it for the owner to settle;
//! - **refused** when no label is declared for the item's category.
//!
//! What each ticket last read is machine state in
//! `$XDG_STATE_HOME/ariane/monday.json`. Without `--apply`, nothing is sent.
//! The token comes from `secrets.toml` ([`crate::secrets`]), by account.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::{Value, json};

use crate::cli::SyncArgs;
use crate::node::{Category, Person};
use crate::tree::{Item, NodePath, Tree};

const API: &str = "https://api.monday.com/v2";

/// A node's binding to a monday board, read from its effective `[tracker]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub account: String,
    pub board: u64,
    pub column: String,
    /// Label id → the category it reads as.
    pub reads: BTreeMap<String, Category>,
    /// Category → the label id it is pushed as.
    pub push: BTreeMap<Category, String>,
    /// `waiting_on` → the label id a waiting item is pushed as.
    pub waiting: BTreeMap<String, String>,
}

impl Binding {
    /// Reads a `[tracker]` table; `None` when it is not a monday binding.
    pub fn from_tracker(tracker: &toml::Table) -> Option<Binding> {
        if tracker.get("kind")?.as_str()? != "monday" {
            return None;
        }
        let labels = |key: &str| -> BTreeMap<String, String> {
            tracker
                .get(key)
                .and_then(toml::Value::as_table)
                .map(|table| {
                    table
                        .iter()
                        .filter_map(|(name, value)| {
                            Some((name.clone(), value.as_str()?.to_owned()))
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let category = |id: &str| {
            Category::ALL
                .into_iter()
                .find(|category| category.id() == id)
        };
        Some(Binding {
            account: tracker.get("account")?.as_str()?.to_owned(),
            board: u64::try_from(tracker.get("board")?.as_integer()?).ok()?,
            column: tracker
                .get("status_column")
                .and_then(toml::Value::as_str)
                .unwrap_or("status")
                .to_owned(),
            reads: labels("status")
                .into_iter()
                .filter_map(|(label, id)| Some((label, category(&id)?)))
                .collect(),
            push: labels("push")
                .into_iter()
                .filter_map(|(id, label)| Some((category(&id)?, label)))
                .collect(),
            waiting: labels("waiting"),
        })
    }

    /// The label an item of `category` waiting `on` is pushed as: the
    /// declared one, or the single label that reads as the category.
    fn label_for(&self, category: Category, on: Option<&str>) -> Option<String> {
        if category == Category::Waiting {
            let on = on.unwrap_or("other");
            return self
                .waiting
                .get(on)
                .or_else(|| self.waiting.get("other"))
                .cloned();
        }
        if let Some(label) = self.push.get(&category) {
            return Some(label.clone());
        }
        let mut reading = self.reads.iter().filter(|(_, read)| **read == category);
        match (reading.next(), reading.next()) {
            (Some((label, _)), None) => Some(label.clone()),
            _ => None,
        }
    }
}

/// What to do with one ticket.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// The ticket already reads as the item.
    InStep,
    /// Set the ticket's label; `mention` lists the people to tell, by name and
    /// monday id, when the item enters a wait.
    Push {
        from: Option<String>,
        to: String,
        mention: Vec<(String, u64)>,
    },
    /// Both sides moved: the owner decides.
    Conflict {
        monday: Option<String>,
        last: Option<String>,
    },
    /// No label is declared for the item's category.
    Refused { reason: String },
}

/// One ticket of the plan.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Step {
    pub item: String,
    pub ticket: u64,
    pub category: Category,
    #[serde(flatten)]
    pub action: Action,
    #[serde(skip)]
    pub board: u64,
    #[serde(skip)]
    pub column: String,
    #[serde(skip)]
    pub account: String,
}

/// The label each ticket had when last synced, by ticket id.
pub type State = BTreeMap<String, String>;

/// The monday API, as the connector needs it; faked in tests.
pub trait Api {
    /// The current label id of each ticket's status column.
    fn labels(&self, tickets: &[u64], column: &str) -> Result<BTreeMap<u64, Option<String>>>;
    fn set_label(&self, board: u64, ticket: u64, column: &str, label: &str) -> Result<()>;
    fn post_update(&self, ticket: u64, body: &str) -> Result<()>;
}

/// The tickets of the items of `scope` (or of `only`) bound to monday, by account.
fn tickets<'a>(
    tree: &'a Tree,
    scope: Option<&'a NodePath>,
    only: Option<&'a Item>,
) -> Vec<(&'a Item, u64, Binding)> {
    let items: Vec<&Item> = match only {
        Some(item) => vec![item],
        None => tree.items_within(scope).collect(),
    };
    items
        .into_iter()
        .filter(|item| item.kind != crate::item::Kind::Reflexion)
        .filter_map(|item| {
            let binding =
                Binding::from_tracker(tree.configs.effective(&item.node).tracker.as_ref()?)?;
            let ticket = item
                .document
                .texts("external")
                .into_iter()
                .find_map(|entry| entry.strip_prefix("monday:")?.parse().ok())?;
            Some((item, ticket, binding))
        })
        .collect()
}

/// Plans the sync of `bound` tickets, given their current labels and the state of the last sync.
pub fn plan(
    tree: &Tree,
    bound: &[(&Item, u64, Binding)],
    labels: &BTreeMap<u64, Option<String>>,
    state: &State,
) -> Vec<Step> {
    bound
        .iter()
        .filter_map(|(item, ticket, binding)| {
            let effective = tree.configs.effective(&item.node);
            let status = item.document.text("status")?;
            let category = effective.status(status)?.category;
            let on = item.document.text("waiting_on");
            let current = labels.get(ticket).cloned().flatten();
            let last = state.get(&ticket.to_string()).cloned();
            let reads = current
                .as_ref()
                .and_then(|label| binding.reads.get(label))
                .copied();
            let wanted = binding.label_for(category, on);
            let in_step = match category {
                Category::Waiting => wanted.is_some() && current == wanted,
                _ => reads == Some(category),
            };
            let action = if in_step {
                Action::InStep
            } else if let Some(to) = wanted {
                let monday_moved = last
                    .as_ref()
                    .is_none_or(|last| Some(last) != current.as_ref());
                if monday_moved {
                    Action::Conflict {
                        monday: current.clone(),
                        last,
                    }
                } else {
                    let mention = if category == Category::Waiting {
                        people(item, &effective.people)
                    } else {
                        Vec::new()
                    };
                    Action::Push {
                        from: current.clone(),
                        to,
                        mention,
                    }
                }
            } else {
                Action::Refused {
                    reason: format!(
                        "no monday label for `{}`: declare it under [tracker.push]{}",
                        category.id(),
                        if category == Category::Waiting {
                            " or [tracker.waiting]"
                        } else {
                            ""
                        }
                    ),
                }
            };
            Some(Step {
                item: item.reference()?,
                ticket: *ticket,
                category,
                action,
                board: binding.board,
                column: binding.column.clone(),
                account: binding.account.clone(),
            })
        })
        .collect()
}

/// The people an item waits for that have a monday id, with their names.
fn people(item: &Item, directory: &BTreeMap<String, Person>) -> Vec<(String, u64)> {
    item.document
        .texts("waiting_for")
        .into_iter()
        .filter_map(|handle| {
            let person = directory.get(handle)?;
            let id = u64::try_from(person.ids.get("monday")?.as_integer()?).ok()?;
            Some((person.name.clone(), id))
        })
        .collect()
}

/// The update posted when an item enters a wait: the people @mentioned.
pub fn waiting_update(
    account: &str,
    mention: &[(String, u64)],
    on: Option<&str>,
    since: Option<&str>,
) -> String {
    let names: Vec<String> = mention
        .iter()
        .map(|(name, id)| {
            format!(
                "<a href=\"https://{account}.monday.com/users/{id}\" class=\"user_mention_editor router\" data-mention-type=\"User\" data-mention-id=\"{id}\" target=\"_blank\" rel=\"noopener noreferrer\">@{name}</a>"
            )
        })
        .collect();
    let why = match on {
        Some("review") => " (revue)",
        Some("answer") => " (réponse)",
        _ => "",
    };
    let since = since
        .map(|date| format!(", depuis le {date}"))
        .unwrap_or_default();
    format!(
        "⏳ En attente de {}{why}{since}. — Ariane",
        names.join(", ")
    )
}

/// Applies the pushes of `steps` and records every ticket read in step or pushed.
///
/// # Errors
///
/// Fails on the first call monday refuses; what was sent before is recorded.
pub fn apply(api: &dyn Api, tree: &Tree, steps: &[Step], state: &mut State) -> Result<()> {
    for step in steps {
        match &step.action {
            Action::InStep => {
                if let Some(label) = current_label(tree, step) {
                    state.insert(step.ticket.to_string(), label);
                }
            }
            Action::Push { to, mention, .. } => {
                api.set_label(step.board, step.ticket, &step.column, to)
                    .with_context(|| format!("moving monday:{} for {}", step.ticket, step.item))?;
                state.insert(step.ticket.to_string(), to.clone());
                if !mention.is_empty() {
                    let item = crate::show::find(tree, &step.item, None)?;
                    let body = waiting_update(
                        &step.account,
                        mention,
                        item.document.text("waiting_on"),
                        item.document.text("waiting_since"),
                    );
                    api.post_update(step.ticket, &body)
                        .with_context(|| format!("posting on monday:{}", step.ticket))?;
                }
            }
            Action::Conflict { .. } | Action::Refused { .. } => {}
        }
    }
    Ok(())
}

/// For a step in step, the label to remember: the one that reads as the item.
fn current_label(tree: &Tree, step: &Step) -> Option<String> {
    let item = crate::show::find(tree, &step.item, None).ok()?;
    let binding = Binding::from_tracker(tree.configs.effective(&item.node).tracker.as_ref()?)?;
    binding.label_for(step.category, item.document.text("waiting_on"))
}

/// `$XDG_STATE_HOME/ariane/monday.json`.
pub fn state_file() -> PathBuf {
    crate::focus::file().with_file_name("monday.json")
}

pub fn load_state(file: &Path) -> State {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// # Errors
///
/// Fails when the state file cannot be written.
pub fn save_state(file: &Path, state: &State) -> Result<()> {
    if let Some(directory) = file.parent() {
        std::fs::create_dir_all(directory)?;
    }
    std::fs::write(file, serde_json::to_string_pretty(state)?)
        .with_context(|| format!("writing {}", file.display()))
}

/// The monday API over HTTPS with one account's token.
pub struct Http {
    token: String,
}

impl Http {
    pub fn new(token: String) -> Http {
        Http { token }
    }

    fn query(&self, query: &str, variables: Value) -> Result<Value> {
        let mut response = ureq::post(API)
            .header("Authorization", &self.token)
            .header("API-Version", "2025-04")
            .send_json(json!({ "query": query, "variables": variables }))
            .context("calling the monday API")?;
        let body: Value = response
            .body_mut()
            .read_json()
            .context("reading monday's answer")?;
        if let Some(errors) = body.get("errors").or_else(|| body.get("error_message")) {
            bail!("monday refused: {errors}");
        }
        Ok(body)
    }
}

impl Http {
    /// The name of the account's user the token belongs to, to check a token.
    ///
    /// # Errors
    ///
    /// Fails when monday refuses the token.
    pub fn me(&self) -> Result<String> {
        let body = self.query("query{me{name}}", json!({}))?;
        Ok(body["data"]["me"]["name"]
            .as_str()
            .unwrap_or_default()
            .to_owned())
    }
}

impl Api for Http {
    fn labels(&self, tickets: &[u64], column: &str) -> Result<BTreeMap<u64, Option<String>>> {
        let mut labels = BTreeMap::new();
        for chunk in tickets.chunks(100) {
            let ids: Vec<String> = chunk.iter().map(u64::to_string).collect();
            let body = self.query(
                "query($ids:[ID!],$column:[String!]){items(ids:$ids){id column_values(ids:$column){value}}}",
                json!({ "ids": ids, "column": [column] }),
            )?;
            for item in body["data"]["items"].as_array().into_iter().flatten() {
                let Some(id) = item["id"].as_str().and_then(|id| id.parse().ok()) else {
                    continue;
                };
                let label = item["column_values"][0]["value"]
                    .as_str()
                    .and_then(|value| serde_json::from_str::<Value>(value).ok())
                    .and_then(|value| value["index"].as_u64())
                    .map(|index| index.to_string());
                labels.insert(id, label);
            }
        }
        Ok(labels)
    }

    fn set_label(&self, board: u64, ticket: u64, column: &str, label: &str) -> Result<()> {
        let index: u64 = label.parse().context("a label id is a number")?;
        self.query(
            "mutation($board:ID!,$item:ID!,$column:String!,$value:JSON!){change_column_value(board_id:$board,item_id:$item,column_id:$column,value:$value){id}}",
            json!({
                "board": board.to_string(),
                "item": ticket.to_string(),
                "column": column,
                "value": json!({ "index": index }).to_string(),
            }),
        )
        .map(|_| ())
    }

    fn post_update(&self, ticket: u64, body: &str) -> Result<()> {
        self.query(
            "mutation($item:ID!,$body:String!){create_update(item_id:$item,body:$body){id}}",
            json!({ "item": ticket.to_string(), "body": body }),
        )
        .map(|_| ())
    }
}

/// The result of one sync, as `--json` prints it and the interface reads it.
#[derive(Debug, Serialize)]
pub struct Report {
    pub applied: bool,
    pub steps: Vec<Step>,
}

/// Plans, and with `apply` performs, the sync of the items of `scope` (or of
/// the one item `only`), reading tokens from `secrets` and the last sync from
/// `state`.
///
/// # Errors
///
/// Fails when a token is missing, or monday cannot be reached or refuses.
pub fn sync(
    tree: &Tree,
    scope: Option<&NodePath>,
    only: Option<&Item>,
    apply_it: bool,
    secrets: &Path,
    state_path: &Path,
    connect: &dyn Fn(String) -> Box<dyn Api>,
) -> Result<Report> {
    let bound = tickets(tree, scope, only);
    let mut by_account: BTreeMap<String, Vec<(&Item, u64, Binding)>> = BTreeMap::new();
    for entry in bound {
        by_account
            .entry(entry.2.account.clone())
            .or_default()
            .push(entry);
    }
    let mut state = load_state(state_path);
    let mut steps = Vec::new();
    for (account, bound) in by_account {
        let Some(token) = crate::secrets::token(secrets, "monday", &account)? else {
            bail!(
                "no monday token for `{account}`: run `ariane connect monday --account {account}`"
            );
        };
        let api = connect(token);
        let mut labels = BTreeMap::new();
        let mut by_column: BTreeMap<String, Vec<u64>> = BTreeMap::new();
        for (_, ticket, binding) in &bound {
            by_column
                .entry(binding.column.clone())
                .or_default()
                .push(*ticket);
        }
        for (column, ids) in by_column {
            labels.extend(api.labels(&ids, &column)?);
        }
        let planned = plan(tree, &bound, &labels, &state);
        if apply_it {
            let outcome = apply(api.as_ref(), tree, &planned, &mut state);
            save_state(state_path, &state)?;
            outcome?;
        }
        steps.extend(planned);
    }
    Ok(Report {
        applied: apply_it,
        steps,
    })
}

/// Runs `ariane connect monday --account <account>`: reads a token on
/// standard input, without echo on a terminal, checks it against monday and
/// stores it in `secrets.toml`.
///
/// # Errors
///
/// Fails when no token is given, monday refuses it, or it cannot be stored.
pub fn connect(account: &str) -> Result<ExitCode> {
    use std::io::{BufRead, IsTerminal};
    let terminal = std::io::stdin().is_terminal();
    if terminal {
        eprint!(
            "monday API token for `{account}` (monday › avatar › Developers › My access tokens): "
        );
        let _ = std::process::Command::new("stty")
            .arg("-echo")
            .stdin(std::process::Stdio::inherit())
            .status();
    }
    let mut token = String::new();
    let read = std::io::stdin().lock().read_line(&mut token);
    if terminal {
        let _ = std::process::Command::new("stty")
            .arg("echo")
            .stdin(std::process::Stdio::inherit())
            .status();
        eprintln!();
    }
    read.context("reading the token")?;
    let token = token.trim().to_owned();
    if token.is_empty() {
        bail!("no token given");
    }
    let name = Http::new(token.clone())
        .me()
        .context("checking the token with monday")?;
    let file = crate::secrets::file();
    crate::secrets::store(&file, "monday", account, &token)?;
    println!(
        "monday token of {name} stored for `{account}` in {} (readable by you only)",
        file.display()
    );
    Ok(ExitCode::SUCCESS)
}

/// Runs `ariane sync monday`.
///
/// # Errors
///
/// As [`sync`], or when `--item` names no single item.
pub fn run(tree: &Tree, args: &SyncArgs, scope: Option<&NodePath>) -> Result<ExitCode> {
    let only = args
        .item
        .as_deref()
        .map(|reference| crate::show::find(tree, reference, None))
        .transpose()?;
    let report = sync(
        tree,
        scope,
        only,
        args.apply,
        &crate::secrets::file(),
        &state_file(),
        &|token| Box::new(Http::new(token)),
    )?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }
    let undecided = report.steps.iter().any(|step| {
        matches!(
            step.action,
            Action::Conflict { .. } | Action::Refused { .. }
        )
    });
    Ok(if undecided {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

fn print_report(report: &Report) {
    let verb = if report.applied {
        "moved"
    } else {
        "would move"
    };
    let mut in_step = 0;
    for step in &report.steps {
        match &step.action {
            Action::InStep => in_step += 1,
            Action::Push { from, to, mention } => {
                let told = if mention.is_empty() {
                    String::new()
                } else {
                    format!(
                        ", telling {}",
                        mention
                            .iter()
                            .map(|(name, _)| name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                println!(
                    "{}  {verb} monday:{} from label {} to {to}{told}",
                    step.item,
                    step.ticket,
                    from.as_deref().unwrap_or("none")
                );
            }
            Action::Conflict { monday, last } => println!(
                "{}  conflict: monday:{} is on label {} (last synced: {}) but the item is {} — settle it by hand",
                step.item,
                step.ticket,
                monday.as_deref().unwrap_or("none"),
                last.as_deref().unwrap_or("never"),
                step.category.id()
            ),
            Action::Refused { reason } => println!("{}  refused: {reason}", step.item),
        }
    }
    println!("{in_step} tickets already in step");
    if !report.applied
        && report
            .steps
            .iter()
            .any(|step| matches!(step.action, Action::Push { .. }))
    {
        println!("\nnothing sent; run with --apply to update monday");
    }
}

/// Unit tests of the plan and of its application, against a fake monday.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;
    use std::cell::RefCell;

    const NODE: &str = "[tracker]\nkind = \"monday\"\naccount = \"acme\"\nboard = 7\nstatus_column = \"task_status\"\n\n[tracker.status]\n\"3\" = \"todo\"\n\"0\" = \"in_progress\"\n\"2\" = \"waiting\"\n\"4\" = \"blocked\"\n\"8\" = \"done\"\n\"9\" = \"in_progress\"\n\n[tracker.push]\nin_progress = \"0\"\n\n[tracker.waiting]\nreview = \"2\"\nother = \"4\"\n\n[people.keyvan]\nname = \"Keyvan\"\nmonday = 42\n";

    fn task(id: &str, ticket: u64, status: &str, extra: &str) -> String {
        format!(
            "---\nariane: 1\nnode: p\nid: {id}\ntitle: {id}\nstatus: {status}\nexternal: [monday:{ticket}]\n{extra}---\n"
        )
    }

    #[derive(Default)]
    struct Fake {
        labels: BTreeMap<u64, Option<String>>,
        sent: RefCell<Vec<String>>,
    }

    impl Api for Fake {
        fn labels(&self, tickets: &[u64], _: &str) -> Result<BTreeMap<u64, Option<String>>> {
            Ok(tickets
                .iter()
                .map(|id| (*id, self.labels.get(id).cloned().flatten()))
                .collect())
        }
        fn set_label(&self, board: u64, ticket: u64, column: &str, label: &str) -> Result<()> {
            self.sent
                .borrow_mut()
                .push(format!("label {board}/{ticket}/{column} = {label}"));
            Ok(())
        }
        fn post_update(&self, ticket: u64, body: &str) -> Result<()> {
            self.sent
                .borrow_mut()
                .push(format!("update {ticket}: {body}"));
            Ok(())
        }
    }

    fn sample() -> Tree {
        let repo = Repo::new();
        repo.write("p/node.toml", NODE)
            .write("p/tasks/T1-a.md", &task("T1", 1, "todo", ""))
            .write("p/tasks/T2-b.md", &task("T2", 2, "in_progress", ""))
            .write(
                "p/tasks/T3-c.md",
                &task(
                    "T3",
                    3,
                    "waiting",
                    "waiting_for: [keyvan]\nwaiting_on: review\nwaiting_since: 2026-10-23\n",
                ),
            )
            .write("p/tasks/T4-d.md", &task("T4", 4, "done", ""))
            .write("p/tasks/T5-e.md", &task("T5", 5, "backlog", ""));
        let tree = Tree::load(repo.root()).unwrap();
        std::mem::forget(repo);
        tree
    }

    fn actions(steps: &[Step]) -> Vec<(String, Action)> {
        steps
            .iter()
            .map(|step| (step.item.clone(), step.action.clone()))
            .collect()
    }

    #[test]
    fn the_plan_pushes_what_ariane_moved_and_refuses_to_guess() {
        let tree = sample();
        let bound = tickets(&tree, None, None);
        let labels = BTreeMap::from([
            (1, Some("3".to_owned())),
            (2, Some("3".to_owned())),
            (3, Some("0".to_owned())),
            (4, Some("0".to_owned())),
            (5, Some("3".to_owned())),
        ]);
        let state = State::from([
            ("2".to_owned(), "3".to_owned()),
            ("3".to_owned(), "0".to_owned()),
        ]);
        let steps = plan(&tree, &bound, &labels, &state);
        assert_eq!(
            actions(&steps),
            [
                ("p#T1".to_owned(), Action::InStep),
                (
                    "p#T2".to_owned(),
                    Action::Push {
                        from: Some("3".to_owned()),
                        to: "0".to_owned(),
                        mention: vec![]
                    }
                ),
                (
                    "p#T3".to_owned(),
                    Action::Push {
                        from: Some("0".to_owned()),
                        to: "2".to_owned(),
                        mention: vec![("Keyvan".to_owned(), 42)]
                    }
                ),
                (
                    "p#T4".to_owned(),
                    Action::Conflict {
                        monday: Some("0".to_owned()),
                        last: None
                    }
                ),
                (
                    "p#T5".to_owned(),
                    Action::Refused {
                        reason: "no monday label for `backlog`: declare it under [tracker.push]"
                            .to_owned()
                    }
                ),
            ]
        );
    }

    #[test]
    fn a_ticket_moved_in_monday_since_the_last_sync_is_a_conflict() {
        let tree = sample();
        let bound = tickets(&tree, None, None);
        let labels = BTreeMap::from([(2, Some("8".to_owned()))]);
        let state = State::from([("2".to_owned(), "3".to_owned())]);
        let steps = plan(&tree, &bound, &labels, &state);
        assert!(matches!(steps[1].action, Action::Conflict { .. }));
    }

    #[test]
    fn applying_moves_the_tickets_mentions_the_people_and_remembers() {
        let tree = sample();
        let fake = Fake {
            labels: BTreeMap::from([(1, Some("3".to_owned())), (3, Some("0".to_owned()))]),
            ..Fake::default()
        };
        let bound: Vec<_> = tickets(&tree, None, None)
            .into_iter()
            .filter(|(_, ticket, _)| [1, 3].contains(ticket))
            .collect();
        let mut state = State::from([("3".to_owned(), "0".to_owned())]);
        let steps = plan(&tree, &bound, &fake.labels, &state);
        apply(&fake, &tree, &steps, &mut state).unwrap();
        let sent = fake.sent.borrow();
        assert_eq!(sent[0], "label 7/3/task_status = 2");
        assert!(
            sent[1].starts_with(
                "update 3: ⏳ En attente de <a href=\"https://acme.monday.com/users/42\""
            ),
            "{}",
            sent[1]
        );
        assert!(
            sent[1].contains("@Keyvan</a> (revue), depuis le 2026-10-23"),
            "{}",
            sent[1]
        );
        assert_eq!(state["1"], "3");
        assert_eq!(state["3"], "2");
    }

    #[test]
    fn a_binding_reads_its_maps_and_a_single_label_needs_no_push_entry() {
        let table: toml::Table = toml::from_str(NODE).unwrap();
        let binding = Binding::from_tracker(table["tracker"].as_table().unwrap()).unwrap();
        assert_eq!(binding.board, 7);
        assert_eq!(
            binding.label_for(Category::Done, None).as_deref(),
            Some("8")
        );
        assert_eq!(
            binding.label_for(Category::InProgress, None).as_deref(),
            Some("0")
        );
        assert_eq!(
            binding
                .label_for(Category::Waiting, Some("answer"))
                .as_deref(),
            Some("4")
        );
        assert_eq!(binding.label_for(Category::Backlog, None), None);
    }
}

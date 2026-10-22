//! `ariane focus`: which item an agent session is working on.
//!
//! When a discussion becomes a task, a story, a reflexion or a ressource, the
//! agent attaches it to that item with `ariane focus <node>#<id>`. The
//! Claude Code hook ([`crate::hook`]) sees that command go by with the
//! session's id and notes it here; later in the same session, the branches and
//! pull requests the agent creates are recorded in that item. A branch created
//! before any focus waits here, pending, and goes into the item when the focus
//! is set.
//!
//! This is machine state, not content: it lives in
//! `$XDG_STATE_HOME/ariane/sessions.json` (`~/.local/state/…`), never in a
//! repository. Sessions untouched for 30 days are forgotten.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::FocusArgs;
use crate::tree::{NodePath, Tree};
use crate::work::Links;

/// How long a session is remembered after its last change, in seconds.
const KEEP: u64 = 30 * 24 * 3600;

/// What is known of one session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// The item the session works on, `<node>#<id>`.
    #[serde(default)]
    pub item: Option<String>,
    /// Branches and pull requests created before the session had an item.
    #[serde(default)]
    pub pending: Links,
    /// Seconds since the Unix epoch of the last change.
    #[serde(default)]
    pub touched: u64,
}

/// Every remembered session, by id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sessions(pub BTreeMap<String, Session>);

/// The state file: `$XDG_STATE_HOME/ariane/sessions.json`, falling back to
/// `~/.local/state/ariane/sessions.json`.
pub fn file() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("ariane/sessions.json")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

impl Sessions {
    /// Reads the state file; empty when it is missing or unreadable, since
    /// losing it only loses notes.
    pub fn load(file: &std::path::Path) -> Sessions {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Writes the state file, forgetting the sessions older than [`KEEP`].
    ///
    /// # Errors
    ///
    /// Fails when the file or its directory cannot be written.
    pub fn save(&mut self, file: &std::path::Path) -> Result<()> {
        let limit = now().saturating_sub(KEEP);
        self.0.retain(|_, session| session.touched >= limit);
        if let Some(directory) = file.parent() {
            std::fs::create_dir_all(directory)
                .with_context(|| format!("creating {}", directory.display()))?;
        }
        std::fs::write(file, serde_json::to_string_pretty(self)?)
            .with_context(|| format!("writing {}", file.display()))
    }

    /// The session `id`, created when unknown, marked as just changed.
    pub fn touch(&mut self, id: &str) -> &mut Session {
        let session = self.0.entry(id.to_owned()).or_default();
        session.touched = now();
        session
    }
}

/// Runs `ariane focus`: checks that the reference names one item and prints
/// it. With `--session`, as the hook calls it, also notes the focus and
/// records the session's pending links in the item.
///
/// # Errors
///
/// Fails when the reference names no single item, or the item or the state
/// file cannot be written.
pub fn run(tree: &Tree, args: &FocusArgs, here: Option<&NodePath>) -> Result<ExitCode> {
    let item = crate::show::find(tree, &args.reference, here)?;
    let reference = item.reference().unwrap_or_else(|| args.reference.clone());
    println!(
        "focus: {reference}  {}",
        item.document.text("title").unwrap_or_default()
    );
    if let Some(session) = &args.session {
        let added = attach(tree, session, &reference, &file())?;
        if !added.is_empty() {
            println!("recorded {}", describe(&added));
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Notes that `session` works on `reference` in the state file `state`, and
/// records there the links the session had pending; returns them.
///
/// # Errors
///
/// Fails when the item or the state file cannot be written.
pub fn attach(
    tree: &Tree,
    session: &str,
    reference: &str,
    state: &std::path::Path,
) -> Result<Links> {
    let mut sessions = Sessions::load(state);
    let entry = sessions.touch(session);
    entry.item = Some(reference.to_owned());
    let pending = std::mem::take(&mut entry.pending);
    let added = if pending.is_empty() {
        Links::default()
    } else {
        let item = crate::show::find(tree, reference, None)?;
        crate::work::record(&item.file, &pending)?
    };
    sessions.save(state)?;
    Ok(added)
}

/// `branch a, pull request b` for messages.
pub fn describe(links: &Links) -> String {
    links
        .branches
        .iter()
        .map(|branch| format!("branch {branch}"))
        .chain(
            links
                .pull_requests
                .iter()
                .map(|url| format!("pull request {url}")),
        )
        .collect::<Vec<_>>()
        .join(", ")
}

/// Unit tests of the session notes.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    #[test]
    fn focusing_records_the_links_a_session_had_pending() {
        let repo = Repo::new();
        repo.write(
            "p/tasks/T1-a.md",
            "---\nariane: 1\nnode: p\nid: T1\ntitle: A\nstatus: todo\n---\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        let state = repo.root().join("state/sessions.json");
        let mut sessions = Sessions::load(&state);
        sessions
            .touch("s1")
            .pending
            .branches
            .push("o/r:feat".to_owned());
        sessions.save(&state).unwrap();
        let added = attach(&tree, "s1", "p#T1", &state).unwrap();
        assert_eq!(added.branches, ["o/r:feat"]);
        let saved = Sessions::load(&state);
        assert_eq!(saved.0["s1"].item.as_deref(), Some("p#T1"));
        assert!(saved.0["s1"].pending.is_empty());
        let text = std::fs::read_to_string(repo.root().join("tree/p/tasks/T1-a.md")).unwrap();
        assert!(text.contains("branches:\n- o/r:feat\n"), "{text}");
    }

    #[test]
    fn old_sessions_are_forgotten_on_save() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("sessions.json");
        let mut sessions = Sessions::default();
        sessions.0.insert(
            "old".to_owned(),
            Session {
                touched: 1,
                ..Session::default()
            },
        );
        sessions.touch("new");
        sessions.save(&state).unwrap();
        assert_eq!(Sessions::load(&state).0.keys().collect::<Vec<_>>(), ["new"]);
    }
}

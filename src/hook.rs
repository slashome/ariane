//! `ariane hook claude-code`: what Ariane does after each shell command of a
//! Claude Code session.
//!
//! Claude Code runs it as a `PostToolUse` hook on `Bash` and sends the event
//! as JSON on standard input: the session id, its directory, the command and
//! its output. Three commands matter:
//!
//! - `ariane focus <ref>`: the session now works on that item ([`crate::focus`]);
//! - a branch created (`git switch -c`, `git checkout -b`, `git branch <name>`,
//!   `git worktree add -b`): `<owner>/<repo>:<branch>` goes into the item's
//!   `branches`;
//! - `gh pr create`: the URL it prints goes into the item's `pull_requests`.
//!
//! Without an item yet, a branch or a pull request created where Ariane's
//! rules apply waits in the session's notes, and the agent is reminded to
//! attach the discussion to its item; the focus records it then. This is not a
//! git hook (ADR-0003): it lives in the agent's harness, touches no host
//! repository, and only sees what goes through it. `ariane links` catches up
//! with the rest.
//!
//! A hook must never get in the way: whatever happens, it exits `0`, and says
//! what it did to the agent through `additionalContext`.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Deserialize;
use serde_json::json;

use crate::focus::{self, Sessions};
use crate::tree::Tree;
use crate::work::{self, Links};

/// The part of a `PostToolUse` event this hook reads.
#[derive(Debug, Default, Deserialize)]
pub struct Event {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub cwd: PathBuf,
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: Input,
    #[serde(default)]
    pub tool_response: serde_json::Value,
}

/// The input of a `Bash` tool call.
#[derive(Debug, Default, Deserialize)]
pub struct Input {
    #[serde(default)]
    pub command: String,
}

/// What one shell command line did, as far as Ariane is concerned.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Seen {
    /// The reference given to `ariane focus`, if the line ran it.
    pub focus: Option<String>,
    /// Branches created, each with the directory of the repository.
    pub branches: Vec<(PathBuf, String)>,
    /// Whether the line ran `gh pr create`.
    pub pull_request: bool,
}

/// Runs the hook on the event read from standard input; always exits `0`.
pub fn run() -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return ExitCode::SUCCESS;
    }
    let Ok(event) = serde_json::from_str::<Event>(&input) else {
        return ExitCode::SUCCESS;
    };
    if let Some(message) = handle(&event, &focus::file()) {
        let output = json!({
            "hookSpecificOutput": { "hookEventName": "PostToolUse", "additionalContext": message }
        });
        println!("{output}");
    }
    ExitCode::SUCCESS
}

/// Acts on one event with the session notes in `state`; returns what to tell
/// the agent, if anything.
pub fn handle(event: &Event, state: &Path) -> Option<String> {
    if event.tool_name != "Bash" || event.session_id.is_empty() {
        return None;
    }
    let seen = read_line(&event.tool_input.command, &event.cwd);
    let mut links = Links::default();
    for (dir, name) in &seen.branches {
        if branch_exists(dir, name)
            && let Some(repository) = work::repository(dir)
        {
            links.branches.push(format!("{repository}:{name}"));
        }
    }
    if seen.pull_request {
        links.pull_requests = pull_request_urls(&event.tool_response);
    }
    if let Some(reference) = &seen.focus {
        return focus_on(event, reference, state);
    }
    if links.is_empty() {
        return None;
    }
    let mut sessions = Sessions::load(state);
    match sessions
        .0
        .get(&event.session_id)
        .and_then(|session| session.item.clone())
    {
        Some(reference) => {
            sessions.touch(&event.session_id);
            let _ = sessions.save(state);
            Some(record_in(event, &reference, &links))
        }
        None if crate::repo::in_a_node(&event.cwd) => {
            sessions.touch(&event.session_id).pending.merge(&links);
            let _ = sessions.save(state);
            Some(format!(
                "Ariane: {} is attached to no item yet. Attach this discussion to its item — find it, or create the task, story, reflexion or ressource — then run `ariane focus <node>#<id>`; it will be recorded in that item.",
                focus::describe(&links)
            ))
        }
        None => None,
    }
}

/// Notes the focus of the session and records its pending links.
fn focus_on(event: &Event, reference: &str, state: &Path) -> Option<String> {
    let tree = load_tree(&event.cwd)?;
    let here = crate::repo::discover_from(None, &event.cwd).ok()?.here;
    let item = crate::show::find(
        &tree,
        reference,
        here.as_ref().filter(|node| !node.is_home()),
    )
    .ok()?;
    let reference = item.reference()?;
    match focus::attach(&tree, &event.session_id, &reference, state) {
        Ok(added) if added.is_empty() => Some(format!(
            "Ariane: this session works on {reference}; its branches and pull requests will be recorded there."
        )),
        Ok(added) => Some(format!(
            "Ariane: this session works on {reference}; recorded {}.",
            focus::describe(&added)
        )),
        Err(failure) => Some(format!(
            "Ariane: could not focus on {reference}: {failure:#}"
        )),
    }
}

/// Records `links` in the item `reference`, and says so.
fn record_in(event: &Event, reference: &str, links: &Links) -> String {
    let result = load_tree(&event.cwd)
        .ok_or_else(|| anyhow::anyhow!("no content repository found"))
        .and_then(|tree| {
            let item = crate::show::find(&tree, reference, None)?;
            work::record(&item.file, links)
        });
    match result {
        Ok(added) if added.is_empty() => format!(
            "Ariane: {} already recorded in {reference}.",
            focus::describe(links)
        ),
        Ok(added) => format!(
            "Ariane: recorded {} in {reference}.",
            focus::describe(&added)
        ),
        Err(failure) => format!(
            "Ariane: could not record {} in {reference}: {failure:#}",
            focus::describe(links)
        ),
    }
}

fn load_tree(cwd: &Path) -> Option<Tree> {
    let location = crate::repo::discover_from(None, cwd).ok()?;
    Tree::load(&location.root).ok()
}

/// Reads a shell command line the way the hook needs: split at `&&`, `||`,
/// `;`, `|` and new lines, following `cd <dir>` and `git -C <dir>`. A
/// command wrapped by a rewriting proxy (`rtk git …`, `rtk proxy gh …`) is
/// read as the command it wraps.
/// Quoting is only stripped, never interpreted: the line is a hint, and each
/// branch is checked against the repository before it is recorded.
pub fn read_line(line: &str, cwd: &Path) -> Seen {
    let mut seen = Seen::default();
    let mut dir = cwd.to_path_buf();
    for part in line.split(['\n', ';', '|', '&']) {
        let mut words: Vec<String> = part
            .split_whitespace()
            .map(|word| word.trim_matches(['"', '\'']).to_owned())
            .collect();
        if words.first().map(String::as_str) == Some("rtk") {
            let wrapper = if words.get(1).map(String::as_str) == Some("proxy") {
                2
            } else {
                1
            };
            words.drain(..wrapper.min(words.len()));
        }
        let Some(program) = words.first() else {
            continue;
        };
        match program.as_str() {
            "cd" => {
                if let Some(target) = words.get(1) {
                    dir = dir.join(target);
                }
            }
            "ariane" => {
                if let Some(index) = words.iter().position(|word| word == "focus") {
                    seen.focus = words[index + 1..]
                        .iter()
                        .find(|word| !word.starts_with('-'))
                        .cloned();
                }
            }
            "gh" if words.get(1).map(String::as_str) == Some("pr")
                && words.get(2).map(String::as_str) == Some("create") =>
            {
                seen.pull_request = true;
            }
            "git" => {
                let (repository, rest) = match words.get(1).map(String::as_str) {
                    Some("-C") => (
                        words.get(2).map_or(dir.clone(), |target| dir.join(target)),
                        &words[3.min(words.len())..],
                    ),
                    _ => (dir.clone(), &words[1..]),
                };
                if let Some(name) = created_branch(rest) {
                    seen.branches.push((repository, name));
                }
            }
            _ => {}
        }
    }
    seen
}

/// The branch a git subcommand creates, if it creates one.
fn created_branch(words: &[String]) -> Option<String> {
    let after = |flags: &[&str]| {
        words.iter().enumerate().find_map(|(index, word)| {
            if flags.contains(&word.as_str()) {
                return words.get(index + 1).cloned();
            }
            word.strip_prefix("--create=").map(str::to_owned)
        })
    };
    match words.first()?.as_str() {
        "switch" => after(&["-c", "-C", "--create", "--force-create"]),
        "checkout" => after(&["-b", "-B"]),
        "worktree" if words.get(1).map(String::as_str) == Some("add") => after(&["-b", "-B"]),
        "branch" => words.get(1).filter(|name| !name.starts_with('-')).cloned(),
        _ => None,
    }
}

fn branch_exists(dir: &Path, name: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("refs/heads/{name}"))
        .output()
        .is_ok_and(|output| output.status.success())
}

/// The pull request URLs `gh pr create` printed, read from the tool's output
/// whatever its shape.
fn pull_request_urls(response: &serde_json::Value) -> Vec<String> {
    let text = response.to_string().replace("\\n", " ");
    let mut urls: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == '"' || c == '\\')
        .filter(|word| word.starts_with("https://") && word.contains("/pull/"))
        .map(|word| word.trim_end_matches(['.', ',', ')']).to_owned())
        .collect();
    urls.dedup();
    urls
}

/// Unit tests of command reading and of the whole hook on a fixture.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    #[test]
    fn a_command_line_tells_focus_branches_and_pull_requests() {
        let cwd = Path::new("/w/app");
        let seen = read_line(
            "cd ../lib && git switch -c fix/keys && gh pr create --fill",
            cwd,
        );
        assert_eq!(
            seen.branches,
            [(PathBuf::from("/w/app/../lib"), "fix/keys".to_owned())]
        );
        assert!(seen.pull_request);
        assert_eq!(
            read_line("ariane focus cmc/cmc-lib#T009", cwd)
                .focus
                .as_deref(),
            Some("cmc/cmc-lib#T009")
        );
        assert_eq!(
            read_line("ariane --repo x focus --json p#T1", cwd)
                .focus
                .as_deref(),
            Some("p#T1")
        );
        let names = |line: &str| {
            read_line(line, cwd)
                .branches
                .into_iter()
                .map(|(_, name)| name)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names("git checkout -b a; git branch b; git switch --create=c"),
            ["a", "b", "c"]
        );
        assert_eq!(names("git -C ../x worktree add ../y -b d"), ["d"]);
        assert!(names("git switch main; git branch -d old; git checkout x").is_empty());
        assert_eq!(
            names("rtk git switch -c e && rtk proxy git checkout -b f"),
            ["e", "f"]
        );
        assert!(read_line("rtk gh pr create --fill", cwd).pull_request);
    }

    #[test]
    fn pull_request_urls_are_read_from_the_output() {
        let response = json!({ "stdout": "Creating pull request\nhttps://github.com/o/r/pull/12\n", "stderr": "" });
        assert_eq!(
            pull_request_urls(&response),
            ["https://github.com/o/r/pull/12"]
        );
        assert!(pull_request_urls(&json!({ "stdout": "nothing" })).is_empty());
    }

    /// A content repository with one task, and a host repository next to it
    /// whose `_ariane` link points into it, with an `origin` remote.
    fn workspace() -> (Repo, tempfile::TempDir) {
        let repo = Repo::new();
        repo.write("p/INDEX.md", "# p\n").write(
            "p/tasks/T1-a.md",
            "---\nariane: 1\nnode: p\nid: T1\ntitle: A\nstatus: todo\n---\n",
        );
        let host = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let ok = Command::new("git")
                .arg("-C")
                .arg(host.path())
                .args(args)
                .status()
                .unwrap()
                .success();
            assert!(ok, "{args:?}");
        };
        git(&["init", "-q"]);
        git(&["remote", "add", "origin", "git@github.com:acme/app.git"]);
        git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@e",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ]);
        git(&["switch", "-q", "-c", "feat/a"]);
        std::os::unix::fs::symlink(repo.root().join("tree/p"), host.path().join("_ariane"))
            .unwrap();
        (repo, host)
    }

    fn event(host: &Path, command: &str, stdout: &str) -> Event {
        Event {
            session_id: "s1".to_owned(),
            cwd: host.to_path_buf(),
            tool_name: "Bash".to_owned(),
            tool_input: Input {
                command: command.to_owned(),
            },
            tool_response: json!({ "stdout": stdout }),
        }
    }

    #[test]
    fn a_branch_before_the_focus_waits_then_lands_in_the_item_with_the_pull_request() {
        let (repo, host) = workspace();
        let state = repo.root().join("state.json");
        let reminder = handle(&event(host.path(), "git switch -c feat/a", ""), &state).unwrap();
        assert!(reminder.contains("attached to no item"), "{reminder}");
        let focused = handle(
            &event(host.path(), "ariane focus T1", "focus: p#T1"),
            &state,
        )
        .unwrap();
        assert!(
            focused.contains("recorded branch acme/app:feat/a"),
            "{focused}"
        );
        let created = handle(
            &event(
                host.path(),
                "gh pr create --fill",
                "https://github.com/acme/app/pull/7\n",
            ),
            &state,
        )
        .unwrap();
        assert!(created.contains("recorded pull request"), "{created}");
        let text = std::fs::read_to_string(repo.root().join("tree/p/tasks/T1-a.md")).unwrap();
        assert!(text.contains("branches:\n- acme/app:feat/a\n"), "{text}");
        assert!(
            text.contains("pull_requests:\n- https://github.com/acme/app/pull/7\n"),
            "{text}"
        );
        assert!(handle(&event(host.path(), "git status", ""), &state).is_none());
    }

    #[test]
    fn outside_a_node_nothing_happens_without_a_focus() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state.json");
        assert!(handle(&event(dir.path(), "git switch -c x", ""), &state).is_none());
    }
}

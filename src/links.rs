//! `ariane links`: catches up with the pull requests the Claude Code hook did
//! not see — opened from a terminal, a browser, another machine.
//!
//! It asks GitHub, through `gh`, for the owner's pull requests, and matches
//! each one to the items it names: an item whose `external` holds
//! `monday:<id>` matches a pull request whose title or body carries that id
//! (as `[<id>]` titles do), and any item matches one that writes its full
//! reference, `<node>#<id>`. A bare id such as `T009` is never matched: every
//! node has one. The pull requests of the content repository itself are left
//! out: they talk about items, they are not their work. Without `--apply` it prints what it would record and writes
//! nothing.

use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::cli::LinksArgs;
use crate::tree::{Item, NodePath, Tree};
use crate::work::{self, Links};

/// How many pull requests are read, most recently updated first.
const LIMIT: &str = "300";

/// A pull request as `gh search prs --json` gives it.
#[derive(Debug, Clone, Deserialize)]
pub struct PullRequest {
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
}

/// Pull requests to record in one item.
#[derive(Debug, Serialize)]
pub struct Proposal {
    pub item: String,
    pub path: String,
    pub pull_requests: Vec<String>,
}

/// Runs `ariane links`: reads the owner's pull requests and records, with
/// `--apply`, those that name an item of `scope`.
///
/// # Errors
///
/// Fails when `gh` cannot run or answers something unreadable, or an item
/// cannot be written.
pub fn run(tree: &Tree, args: &LinksArgs, scope: Option<&NodePath>) -> Result<ExitCode> {
    let content = crate::work::repository(&tree.root);
    let pulls: Vec<PullRequest> = fetch()?
        .into_iter()
        .filter(|pull| {
            !content
                .as_deref()
                .is_some_and(|name| of_repository(&pull.url, name))
        })
        .collect();
    let proposals = propose(tree, scope, &pulls);
    if args.apply {
        for proposal in &proposals {
            let item = crate::show::find(tree, &proposal.item, None)?;
            let links = Links {
                branches: Vec::new(),
                pull_requests: proposal.pull_requests.clone(),
            };
            work::record(&item.file, &links)?;
        }
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&proposals)?);
    } else {
        let verb = if args.apply {
            "recorded"
        } else {
            "would record"
        };
        for proposal in &proposals {
            println!(
                "{}  {verb} {}",
                proposal.item,
                proposal.pull_requests.join(", ")
            );
        }
        if proposals.is_empty() {
            println!("every pull request that names an item is already recorded in it");
        } else if !args.apply {
            println!("\nnothing written; run with --apply to record");
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The owner's pull requests, from `gh search prs --author @me`.
///
/// # Errors
///
/// Fails when `gh` is missing, not logged in, or answers something unreadable.
fn fetch() -> Result<Vec<PullRequest>> {
    let output = Command::new("gh")
        .args([
            "search",
            "prs",
            "--author",
            "@me",
            "--limit",
            LIMIT,
            "--json",
            "url,title,body",
        ])
        .output()
        .context("running gh, the GitHub CLI")?;
    if !output.status.success() {
        bail!("gh: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    serde_json::from_slice(&output.stdout).context("reading gh's answer")
}

/// What each item of `scope` lacks among the pull requests that name it.
pub fn propose(tree: &Tree, scope: Option<&NodePath>, pulls: &[PullRequest]) -> Vec<Proposal> {
    tree.items_within(scope)
        .filter_map(|item| {
            let reference = item.reference()?;
            let recorded = item.document.texts("pull_requests");
            let mut found: Vec<String> = pulls
                .iter()
                .filter(|pull| {
                    names(item, &reference, pull) && !recorded.contains(&pull.url.as_str())
                })
                .map(|pull| pull.url.clone())
                .collect();
            found.dedup();
            (!found.is_empty()).then(|| Proposal {
                item: reference,
                path: item.relative.to_string_lossy().into_owned(),
                pull_requests: found,
            })
        })
        .collect()
}

/// Whether the pull request at `url` belongs to `repository`, `<owner>/<repo>`.
fn of_repository(url: &str, repository: &str) -> bool {
    url.contains(&format!("/{repository}/pull/"))
}

/// Whether `pull` names `item`: its full reference, or the id of one of its
/// monday tickets as a whole number.
fn names(item: &Item, reference: &str, pull: &PullRequest) -> bool {
    let text = format!("{}\n{}", pull.title, pull.body);
    contains_word(&text, reference)
        || item
            .document
            .texts("external")
            .into_iter()
            .filter_map(|entry| entry.strip_prefix("monday:"))
            .any(|id| contains_word(&text, id))
}

/// Whether `needle` appears in `text` not glued to other letters or digits.
fn contains_word(text: &str, needle: &str) -> bool {
    text.match_indices(needle).any(|(index, _)| {
        let before = text[..index].chars().next_back();
        let after = text[index + needle.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// Unit tests of the matching.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    fn pull(url: &str, title: &str, body: &str) -> PullRequest {
        PullRequest {
            url: url.to_owned(),
            title: title.to_owned(),
            body: body.to_owned(),
        }
    }

    #[test]
    fn a_pull_request_knows_its_repository() {
        assert!(of_repository(
            "https://github.com/slashome/_ariane/pull/19",
            "slashome/_ariane"
        ));
        assert!(!of_repository(
            "https://github.com/slashome/ariane/pull/19",
            "slashome/_ariane"
        ));
    }

    #[test]
    fn pull_requests_are_matched_by_monday_id_or_full_reference() {
        let repo = Repo::new();
        repo.write(
            "cmc/lib/tasks/T9-a.md",
            "---\nariane: 1\nnode: cmc/lib\nid: T9\ntitle: A\nstatus: todo\nexternal: [monday:2955884643]\npull_requests: [https://x/pull/1]\n---\n",
        )
        .write("p/tasks/T9-b.md", "---\nariane: 1\nnode: p\nid: T9\ntitle: B\nstatus: todo\n---\n");
        let tree = Tree::load(repo.root()).unwrap();
        let pulls = [
            pull("https://x/pull/1", "[2955884643] fix(store): keys", ""),
            pull("https://x/pull/2", "[2955884643] feat(store): more", ""),
            pull("https://x/pull/3", "chore", "Part of p#T9."),
            pull("https://x/pull/4", "T9 everywhere [29558846430]", ""),
        ];
        let proposals = propose(&tree, None, &pulls);
        let found: Vec<_> = proposals
            .iter()
            .map(|p| (p.item.as_str(), p.pull_requests.clone()))
            .collect();
        assert_eq!(
            found,
            [
                ("cmc/lib#T9", vec!["https://x/pull/2".to_owned()]),
                ("p#T9", vec!["https://x/pull/3".to_owned()]),
            ]
        );
    }
}

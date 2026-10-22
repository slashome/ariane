# ADR-0014 — Record branches and pull requests through the agent's harness

- **Status:** Accepted
- **Date:** 2026-10-23
- **Concerns:** SPEC §1.3; [ADR-0003](0003-no-git-hooks.md), [ADR-0004](0004-write-or-judge.md), [ADR-0008](0008-give-every-item-a-versioned-front-matter.md), [ADR-0011](0011-sync-external-trackers-through-connectors.md)

## Context

An item says what the work is; nothing said where it is. `refs` was the only place for a pull request — a free list mixed with ADRs and documents, with no format and nothing filling it — and a branch had no place at all. Going from a task to its code meant searching GitHub by hand.

The owner's rule is that a discussion about work is attached to its item first — found, or created — and that the branches and pull requests it produces belong to that item. Two things make it hard to hold:

- **Nothing knows which item a session is about.** A command that creates a branch carries no item.
- **An instruction to the agent is not a guarantee** (ADR-0003). "Write the branch in the item" is exactly the kind of rule an agent forgets, and git hooks, the obvious enforcement, are ruled out: `core.hooksPath` replaces every repository's own hooks.

## Decision

> **Two core keys hold the work's places; a hook of the agent's harness, not of git, fills them for the item the session is attached to.**

- **Keys.** `branches`, a list of `<owner>/<repo>:<branch>` (a task may span a library and its consumer), and `pull_requests`, a list of URLs. `doctor` checks their form. `refs` stays for everything else.
- **Focus.** When a discussion becomes a task, a story, a reflexion or a ressource, the agent runs `ariane focus <node>#<id>`. That is the "item first" rule made into one command.
- **Hook.** `ariane hook claude-code` runs as Claude Code's `PostToolUse` hook on `Bash`. It reads the event — session id, directory, command, output — and:
  - notes `ariane focus <ref>` for the session, in machine state (`$XDG_STATE_HOME/ariane/sessions.json`, never a repository);
  - after `git switch -c`, `git checkout -b`, `git branch <name>` or `git worktree add -b`, checks that the branch exists and records `<owner>/<repo>:<branch>`, the name read from the `origin` remote;
  - after `gh pr create`, records the URL it printed.
  Without a focus, in a directory where Ariane's rules apply, the branch or pull request **waits** in the session's notes and the agent is told to attach the discussion; the focus records what waited. Nothing is blocked: the owner chose a reminder over a refusal.
- **Catch-up.** `ariane links` asks GitHub (`gh search prs --author @me`) for the owner's pull requests and proposes those that name an item — its monday id, as `[<id>]` titles do, or its full `<node>#<id>` — and are not recorded; `--apply` writes. A bare `T009` never matches: every node has one. Pull requests of the content repository are left out: they discuss items, they are not their work.

## Consequences

- An item shows where its work is, and the interface links to it.
- Only what goes through the harness is recorded at once. A branch made in a terminal is not, until someone records it; a pull request is caught up by `ariane links`. This is the price of not touching git.
- The hook writes item headers without the owner asking at that moment. It is a writer in the family of `fix` and `migrate` (ADR-0004), limited to two keys, appending only, and it never commits.
- The hook must never get in the way: it always exits `0` and speaks only through `additionalContext`.
- The focus is per session and per machine. Two sessions on two items do not interfere; a session resumed elsewhere starts without a focus.

## Alternatives considered

### Git hooks (`post-checkout`, a `pre-push` reading the branch)

They would see every branch, terminal included. Ruled out by ADR-0003 for the reason that still holds: per-repository hooks need installing in every clone, and a global `core.hooksPath` silently disables team repositories' own hooks.

### The agent's instruction alone

The skill tells the agent to write the branch into the item. It is best-effort by construction (ADR-0003), and the failure is silent: nothing notices a branch that was not written.

### Blocking branch and pull request creation without a focus

It makes "item first" impossible to skip in the harness. Rejected by the owner: a branch made before the discussion was attached is still useful work, and waiting for the focus loses nothing.

### Deriving the item from the branch name (`T038-…`)

No state at all. It imposes a naming convention on repositories whose teams already have their own, and an id alone is ambiguous across nodes.

### Putting them in `refs`

No schema change. But `refs` has no form to check, mixes pull requests with ADRs and documents, and gives the interface nothing to link reliably.

## Revisit if

- Another agent harness needs the same: `ariane hook` takes the harness as a subcommand for that reason.
- Branches made outside the harness turn out to be the common case, which would argue for a catch-up on branches too (from `git for-each-ref` in the materialized repositories).

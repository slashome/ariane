# ADR-0003 — No git hooks; worktrees are enumerated on demand

- **Status:** Accepted
- **Date:** 2026-08-15
- **Concerns:** PLAN §4, §10

## Context

A node that is a git repository can have several working directories: `git worktree add` creates a second checkout of the same repository, typically to review a branch without disturbing work in progress. The new directory has no `_ariane` symlink, so an agent launched there finds no node and loses all context.

The tempting fix is to have git create the link automatically, via a `post-checkout` hook — which does fire on `git worktree add`. The user's own objection is what forced this decision: *"you would need a skill for Ariane to create the link in the worktree? but a skill is never guaranteed to run"*. That is correct, and it applies to every instruction given to an agent.

Two questions follow: is a worktree a case the central layout cannot represent, and what mechanism can carry an actual guarantee?

## Decision

**No git hooks, under any form.** Worktrees are enumerated on demand with `git worktree list --porcelain`, which is a check of `doctor` and an action of `update`.

A worktree is **not** a case that breaks the mirror layout of ADR-0001. `_ariane` is a symlink, not content: several working directories point at the *same* central subtree, sharing content and history. This is a multiplicity of **materialization**, not of content — the node → central directory relation stays 1:1, and `ariane link` needs no worktree-specific code.

Three consequences for the CLI:

- The key is the **repository**, never the path: `git rev-parse --git-common-dir` identifies it, which handles the ephemeral worktree created outside the projects tree (in `/tmp`, say) where the disk path no longer resembles the logical path at all.
- `doctor` descends from nodes to their materializations, never the reverse. A worktree is therefore never an orphan.
- The check must be re-run per materialization rather than per node, because `.gitignore` is a versioned file: whether `_ariane` is ignored **depends on the current branch**, so two worktrees of one repository on two branches can legitimately give two different answers. This result is never cached.

Behind it sits the general rule this decision exists to enforce:

> **What must hold invariably is the CLI's job to verify. What requires judgement is the agent's.**
> An agent instruction is best-effort by nature and carries no guarantee. If the violation of a rule is detectable by a program, it must be detected by one.

## Alternatives rejected

### A `post-checkout` hook, installed per repository

It works: the hook does fire on `git worktree add` (with a null object id as its first argument), and hooks live in the common directory shared by all worktrees of a repository.

Rejected because `.git/` is neither versioned nor shared: the hook has to be installed in every repository, one by one, and reinstalled after every fresh clone. That is the same reliability as asking an agent to remember, with more moving parts.

### A `post-checkout` hook installed globally via `core.hooksPath`

The apparent fix to the previous point, and the reason this ADR exists.

Rejected because **`core.hooksPath` replaces the repository's hooks directory instead of adding to it.** Setting it globally silently disables every repository's own hooks on the machine — including team repositories where husky or lefthook run linting and tests before each commit. Public precedent for exactly this failure: Claude Code issues #66993 and #72714. It would also be masked in turn by any local `core.hooksPath` those tools set.

The contrast with §4 is the whole lesson, and it is worth stating because the two look like the same trick: **`core.excludesFile` adds to the repository's `.gitignore`, `core.hooksPath` substitutes for the repository's hooks.** One global git setting is safe to use as a mechanism; the other is a trap. This asymmetry should be checked before leaning on any further global git configuration.

### Nothing automatic — the user runs `ariane link` in the worktree

Rejected as a *sole* mechanism: it depends on remembering, so it will fail eventually, which is the objection that started this ADR. It remains available as an explicit action; it is simply not what carries the guarantee.

### Scanning the disk for directories that look like nodes

Rejected: slow, noisy (`~/Documents`, `~/Zomboid`), and asking the wrong question. "Should this directory be a node, and under which project?" is logical placement — judgement, therefore the agent's (§5). A bounded `doctor --scan` may list candidates as information, never as an error, never by default.

## Consequences

- Ariane writes nothing inside a host repository — no `.gitignore`, no `.git/info/exclude`, no `.git/config`, no hook, no `core.hooksPath`. This becomes a prohibition, not a preference.
- The link appears in a new worktree at the next `ariane update`, not at the instant the worktree is created. There is a window during which an agent launched in that worktree finds no node. Accepted: the failure is loud (no node found) rather than silent, and the remedy is one command.
- git maintains the worktree registry itself and marks stale entries `prunable`, so Ariane needs no state of its own to track materializations.
- The "is `_ariane` ignored here" check becomes cheap and repeatable by construction, since it can never be cached.

## Revisit if

- git gains a mechanism for repository-agnostic, additive hooks — the reason for rejection is `core.hooksPath`'s substitution semantics, not hooks as an idea.
- The window between `worktree add` and the next `update` proves painful enough in practice to warrant a shell integration, which would then be the user's own choice and not something Ariane installs.

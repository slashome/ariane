# ADR-0004 — A command writes, or a command judges — never both

- **Status:** Proposed
- **Date:** 2026-08-15
- **Concerns:** PLAN §4, §9, §10

## Context

§10 defines three commands and already says `doctor` is read-only. The question that forced this ADR was nonetheless a good one: if `doctor` verifies that a node's symlink is missing, and is meant to run after `init` and `update`, does `doctor` create the missing symlink?

The discomfort behind the question points at a real defect, though not where it seems to. **The plan describes `init` and `update` as each placing the symlinks** — "creates the symlinks" and "refreshes links for new nodes". Two commands doing the same thing means two implementations that will drift, and a permanent ambiguity about who repairs what.

## Decision

> **A command writes, or a command judges. Never both.**

- **`init` acquires** what does not exist yet and cannot be derived from the declared state: the central clone, the materialization of the configuration, the line in the user's effective global excludes file. It runs once, it is interactive, and it touches files the user owns.
- **`update` converges**: it makes the disk match the declared state. It runs often, non-interactively, idempotently. **It, and only it, creates a missing symlink.**
- **`doctor` observes.** No write path anywhere in its code, and **no `--fix` flag**. Every finding names the exact command that repairs it, or states that the decision belongs to a human.

And the structural consequence: **`init` does not implement link placement, it calls `update`'s convergence engine after bootstrapping.** `init` ≡ *bootstrap* + `update`. Exactly one piece of code knows how to make the disk match the declaration.

Two properties are stated as requirements rather than intentions:

1. **The set of `doctor` findings whose remedy is `ariane update` *is* `update`'s plan.** This makes the pair testable: inject a defect into a fixture, run `update`, run `doctor`, expect `0`; for a non-repairable defect, `update` must change nothing and the finding must be identical before and after. One test forbids both drifts — a `doctor` reporting things `update` cannot fix without saying so, and an `update` fixing things `doctor` does not watch.
2. **`doctor` runs at the end of `init` and `update`, and must never be given access to the plan that was just executed.** It re-observes the system. `init` fails if the pass fails — "this machine is ready" has no other honest definition. `update` fails only on the nodes it claims to have converged; findings elsewhere are printed without failing the command.

Exit codes are monotone in *how much human involvement is required*: `0` healthy · `1` findings `ariane update` repairs on its own · `2` findings needing a human decision · `3` the diagnosis itself could not be performed. `--json` is the interface the agent consumes; `--strict` promotes warnings to errors for CI.

## Alternatives rejected

### `doctor --fix`

Rejected on four grounds.

The repair command already exists and is called `update`; a `--fix` is either a second implementation to keep in sync, or an alias that **misrepresents who owns the writing**. A user who gets used to `doctor --fix` no longer knows that `update` is the command that writes, and the ergonomics gained cost the mental model, which is the actual deliverable.

The argument that settles it for this project is absent from the precedents: **a `doctor` that cannot write is invocable by an agent, in a loop, under a standing permission rule.** §2 makes agent navigation the target use, and ADR-0003 establishes that the agent carries no guarantee. The only way for an agent to *know* without *risking* is a command whose read-only nature is a property of the binary rather than a promise of the caller. The day `doctor` can write, every invocation becomes a permission prompt, and the agent stops running it.

The precedents point the same way: Nix moved from `nix-store --verify --repair` to a separate `nix store repair` command in its new CLI — the flag was the old design. Homebrew never put a `--fix` on `doctor`; its one "fix on a check" (`brew bundle check --install`) is documented as *run install, then check*, an explicit composition rather than repair logic inside the verifier. `git fsck` does write with `--lost-found`, but into a **quarantine directory**, never modifying the corrupt object: the only write a diagnosis may perform is a non-destructive deposit for human inspection.

The ergonomic compensation is mandatory, otherwise the purity is pedantry: every finding prints its exact command, `--json` exposes the `remedy` field, and the documented one-liner is two characters longer than `--fix` and honest.

### `doctor` failing on everything it finds, the way `brew doctor` does

Verified on this machine: `brew doctor` exits 1 on benign findings and its own message says to ignore it. **A tool that has to tell the user to ignore its exit code has failed at its exit code.** Only errors weigh on `ariane doctor`'s status by default; `--strict` exists for those who want a hard gate.

### `doctor` always exiting 0, the way `flutter doctor` does

Verified on this machine: `flutter doctor` exits 0 despite findings, forcing callers to parse text — the worst possible contract. Two public issues document exactly this (flutter#32106, flutter#19703). It is also the pattern by which an action eventually grafts itself onto a diagnostic: `flutter doctor --android-licenses`.

### `update` repairing the dangerous case

A **real directory** where a symlink is expected is never deleted, with no flag, not even `--force`. The reasoning is arithmetic rather than prudential: such content is untracked by the host repository, matched by the global exclude rule, therefore invisible in `git status`, and absent from the central clone. Nothing backs it up, and it is already one `git clean -fdx` from destruction — while §4 promises that this very command is harmless, a promise that holds only for a symlink. Ariane must not become the second thing on the machine able to destroy those files.

Made mechanical rather than declarative: **the binary never calls a recursive delete on a path outside a temporary directory it created itself.** Replacing a directory uses the non-recursive call, which fails natively when the directory is not empty — so the empty-directory case is handled correctly and for free.

What the user actually wants there is not deletion but **adoption**: move the content into the central subtree, then place the link. Non-destructive, but a decision — hence a separate, explicitly invoked `ariane adopt`, which refuses if the corresponding central subtree already holds something (merging is judgement).

### Having `init` set `core.excludesFile`

Rejected, and this one is a trap worth recording. Verified on this machine: `core.excludesFile` is **not** set, and yet `~/.config/git/ignore` is active — it is git's XDG default, and `git check-ignore -v` cites it as the source. Setting `core.excludesFile` on a machine where the user already has one configured elsewhere would **silently disable all their existing global excludes**: the same substitution failure as `core.hooksPath` in ADR-0003. `init` writes one line into the *effective* excludes file, creates that file if absent, and refuses to write at all when it is a symlink or lives inside a git worktree — the signals that a dotfiles manager owns it — printing the exact line to add instead.

Two related facts, both verified, both of which would have produced silent bugs:

- **The pattern must be `_ariane`, never `_ariane/`.** A trailing slash matches directories only, and a symlink to a directory is not a directory for git. The materialization would not be ignored, silently — and a `grep _ariane` check would happily validate the broken file. This is the proof by example that the *property* must be tested, not the presence of a line.
- **`git check-ignore` exits 0 on a negated pattern**, i.e. on a path that is *not* ignored (`.gitignore:20:!.env.example` → exit 0). The exit code alone is a false test; the pattern field must be parsed. `--no-index` is mandatory, otherwise a tracked path is reported as unignored with no rule at all.

## Consequences

- Three commands with three verbs, and no flag can blur them.
- `doctor` is safe to run in a loop, from a script, from CI, or from an agent under a standing permission — which is what makes the agent able to know the state of the deployment at all.
- A check that cannot name its remedy is not a check but an opinion, and gets deleted. This is the rule that keeps `doctor` from drifting into a list of recommendations nobody reads.
- Ariane gains a fourth command, `ariane adopt`, for the one situation `update` will never resolve on its own.
- Some findings are permanently unfixable by the CLI — a host repository negating the exclusion, a back-pointer contradicting the mirror position, a real directory in place of a link. They exit `2` and stop automation, by design.

## Revisit if

- Users demonstrably run `ariane update` blindly on exit code `2`, which would mean the severity distinction is not carrying — the fix would be in the messages, not in a `--fix`.
- A repair turns out to be needed in a context where `update` cannot run, which would be an argument for a narrower command, not for widening `doctor`.

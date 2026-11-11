# ADR-0023 — Plan by releases declared in order per node

- **Status:** Proposed
- **Date:** 2026-11-11
- **Concerns:** SPEC §1.3, §1.5, §1.6; reflexion `slashome/ariane#R010`; [ADR-0019](0019-group-stories-into-epics.md), [ADR-0022](0022-order-items-by-a-sparse-rank-in-their-header.md)

## Context

Reflexion R010 asks for a roadmap view: what will be delivered, in which order, and roughly when. The header already has two targets, `release` and `milestone`, both **free strings** (SPEC §1.3). Nothing declares them and nothing orders them: no tool can tell that `0.9` comes before `1.0`, that `mango` comes before `strawberry`, or that `plus-tard` is not a target at all. A lexical or semver sort fails on the first case (`0.10` after `0.9` only by luck of the parser) and has nothing to say about code names.

`pharmakon` is the only node that uses them, and uses both levels (`grep -rh '^release:\|^milestone:' tree/slashome/pharmakon | sort | uniq -c` in `_ariane`): `release` `1.0` (201 files), `2.0` (6), `plus-tard` (4); `milestone` `0.1` to `1.0` on the 200 files of release `1.0`. Each milestone is a version played and judged by the owner, never published. It is a delivered version of the product in every sense but publication.

What the market calls these levels, checked before deciding:

- On GitHub a **milestone conventionally is the next version**: `gh api 'repos/microsoft/vscode/milestones?state=open' --jq '.[].title'` lists `1.147.0`, `1.148.0`; kubernetes lists `v1.34`…`v1.38`, rust `1.100.0`…`1.102.0`. GitLab follows the same convention.
- GitHub has **no product link** between a milestone and a release: `gh api repos/microsoft/vscode/releases/latest --jq keys` has no milestone field.
- Linear, Jira, GitLab and Productboard all model targets as objects with optional dates. Only their timeline views require dates, which pushes users to invent them (Aha!'s "TBD", Linear's fuzzy quarters).
- Epics and parents are optional in Jira, Linear, Shortcut and Azure DevOps: work is allowed to exist outside the hierarchy.

## Decision

> **A release is a delivered version of the product, even internal. A node declares its releases in order in `node.toml`; a task names one in `release`. There is no level between a release and an epic.**

1. **Declaration.** Each `[[release]]` entry of `node.toml` declares one release: `name` MUST be given, free (`1.8.0`, `0.3`, `mango`); `parking = true` MAY mark a release that is never scheduled and never closed, such as `plus-tard`; `desired` MAY give a date (ADR-0028). The order of entries is the delivery order.
2. **Inheritance.** Like `[[workflow]]`, a `[[release]]` list declared in a node replaces the inherited list **as a whole**. A repository node under a project node plans against the project's releases unless it declares its own.
3. **Carried by tasks.** `release` is a key of tasks. Stories and epics SHOULD NOT carry it: they appear in each release through their tasks, and an epic whose tasks span two releases is drawn as one fragment per release. `doctor` warns on `release` on any other kind, and errors on a value absent from the node's effective list when a list applies. A node with no list anywhere above it keeps free values, which the roadmap cannot order.
4. **The hierarchy is optional.** A task without a story and a story without an epic are legal, and neither `doctor` nor the roadmap treats them as an error. The roadmap gathers them in "(sans story)" and "(sans epic)" groups and folds an empty level. [ADR-0019](0019-group-stories-into-epics.md) still forbids a task under an epic.
5. **Tasks without release** are "non planifié": shown, not scheduled, like a parking release.
6. **Closing a release.** `ariane release close <name>` refuses while a task of the release is open, listing them; `--move-open-to <next>` rewrites their `release` instead, so the slip stays visible in git. Then it moves the release's `done` tasks into `archives/`. It writes the working tree and never commits ([ADR-0009](0009-migrate-content-with-versioned-migrations.md)). A parking release cannot be closed. Whether a release is closed is read from its tasks: none open, none left outside `archives/`.

## Consequences

- One level of target, named the way the forges name it. Epics and stories carry the *what*, releases the *when*.
- The order of releases becomes data a tool can read and an interface can edit, shared with everyone who reads the node.
- Archiving `done` work moves from "when its status changes" to "when its release closes": a validated task stays visible in its release until then, which is what `pharmakon`'s `build.py` already approximates with its milestones. `build.py` stops archiving and only writes the status.
- Every node that wants a roadmap has to write its release list once. A node that never does gets no dates, only an unordered list.
- Moving tasks to the next release is an explicit act with a diff, never a silent roll-over.
- The milestone level disappears; [ADR-0024](0024-remove-milestone-from-the-header.md) removes the key and migrates `pharmakon`.

## Alternatives considered

### Keep `milestone` as an intermediate level

`pharmakon` uses it, and a long 1.0 does need intermediate checkpoints. Rejected because a milestone *is* the next version where every forge uses the word: a second level named milestone is a false friend for anyone coming from GitHub or GitLab, and a checkpoint that is played and judged is a release that is not published. Renaming the level (stage, checkpoint) keeps two nested targets on every task, which can disagree, for a need that ordered releases already meet.

### A release as an item kind, in `releases/`

A file per release could hold a description, a status and a date. Rejected: the prefix `R` is taken by reflexions, and a release item would need its own status, its own order through `rank`, and links from tasks that `doctor` reconciles with the files present — a second source of truth for a list the node already owns. A release has no body worth writing that is not the body of its epics.

### `release` on stories and epics

Planning a whole story at once. Rejected for the reason [ADR-0019](0019-group-stories-into-epics.md) rejected an `epic` key on tasks: a task would have two targets that can disagree, and moving one task to the next release would have to update its story.

### Order releases by sorting their names

No declaration to write. Rejected: code names do not sort, version strings sort only through a parser that guesses, and a parking release has no place in any sort.

## Revisit if

- A team needs a recurring timebox (sprints, cycles) between a release and an epic. That is a cadence, not a target, and would be a new decision.
- Two sibling nodes need to plan against the same releases without a common ancestor declaring them.
- Archiving at release close leaves so many `done` tasks in `tasks/` that lists become hard to read.

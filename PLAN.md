# Ariane — Design Plan

> This document is the founding plan of the method. It will progressively be superseded by `SPEC.md` as concepts get formalized, and by the issue tracker for the roadmap. The decisions taken along the way — and, more importantly, the alternatives rejected and why — are recorded in [`docs/adr/`](docs/adr/).

## Why

Working with AI agents on real, long-lived projects requires a persistent, structured memory of *what we are doing and why*: roadmaps, stories, tasks, decisions, conventions. In practice this knowledge is scattered — per repository, per tool, per conversation — and existing methods (e.g. BMAD) assume a flat world where every project shares the same agents and the work is mostly greenfield.

Ariane formalizes a practice that emerged from daily use: a **file-based** project management layer, **hierarchical** (from the whole of one's activity down to a single repository), designed to be **navigated by AI agents** as much as by humans.

## Core concepts

### 1. A tree of levels, arbitrary depth

Projects are organized as a tree: the **home** level (everything you do) contains **projects** (an employer, a personal brand, an open-source org…), which contain **repositories**, which may contain deeper nodes. The tree is not fixed to three levels — any node can carry project management content.

### 2. One conventional directory per node: `_ariane/`

Each node exposes its project management content in a conventional directory (default name: `_ariane/`, configurable). The leading underscore signals: *this is not the project, this is the meta*.

Every `_ariane/` directory has an **`INDEX.md`** as its single entry point: current state, priorities, and pointers. An agent (or a human) launched at any node reads the local `INDEX.md`, which either answers directly or points up/down the tree. The home-level `INDEX.md` is the minimal registry of all projects.

### 3. Common process, per-user centralized content

- The method itself — spec, templates, agents — lives in **this repository** (`ariane`) and is common to all users.
- Each user's content lives in **one single private repository** (conventional name: `_ariane`, e.g. `<identity>/_ariane`), whose internal tree mirrors their project tree. One clone, one history, atomic commits across levels, nothing ever lost.

### 4. Materialization: symlinks + global gitignore

The central content repository is cloned once, then **materialized** where you work: each node's `_ariane/` directory is a symlink into the corresponding subtree of the central clone. A single `_ariane` line in the user's **global git excludes file** (`core.excludesFile`) makes every host repository ignore it — zero footprint in team repositories, no `.gitignore` to negotiate, and a `git clean -fdx` in a host repo only removes a link, never content. The user's configuration itself is versioned in the central repository and materialized the same way (e.g. `~/.config/ariane` → `<central>/config/`).

### 5. Logical placement vs physical hosting

Two distinct concerns, deliberately separated:

- **Logical placement** — where information *belongs*: stories and tasks belong to the most specific node they concern (usually a repository); roadmaps, cross-repo initiatives and conventions belong to the project level; the registry of projects belongs to home.
- **Physical hosting** — where files *live*: always in the central content repository, materialized at the node. When a node cannot be materialized yet, its content is simply reachable through the parent's `INDEX.md`.

### 6. Item lifecycle

```
reflexion → backlog → task | story → archive
```

Free-form thinking matures into backlog items, which get promoted to actionable tasks or stories, and end up archived — never deleted. `INDEX.md` reflects the live state at all times.

Physically, each state is a directory inside the node's `_ariane/`, and an item is one file:

```
_ariane/
  INDEX.md
  reflexions/
  backlog/
  tasks/
  stories/
  archives/
```

Changing state means moving the file and updating the node's `INDEX.md`. Nothing is ever deleted: completed and abandoned items alike end up in `archives/`, and the trail stays in git.

> **Status: state of the art, not a decision.** The lifecycle above is transcribed from a project-management directory in production use, where it governs the work of many repositories at once. It is the one part of the method proven by daily practice rather than designed, and it is recorded here so that the step 2 workshop challenges something real instead of starting from a blank page — not because it is settled.
>
> Known open points: whether `tasks/` and `stories/` are two directories or one with a typed header; whether `archives/` is per-node or centralized at the project level; whether every node carries all five directories or only the ones it uses; and how an item that concerns two nodes is placed. A dedicated elicitation workshop (a good use case for BMAD's brainstorming/elicitation skills) settles all of it — see the roadmap.
>
> One more, and this one is measured rather than suspected: **an item's state is not readable without reading the item.** The directories above carry the lifecycle state, but status and priority live in free prose inside the file — `> Créée : … — statut : …` in one, `- **Priorité** : …` in another, nothing at all in a third. On the reference instance, **44% of live items** (27 of 61) expose no status a tool can parse; the count comes from the prototype in [`tools/`](tools/), which had to guess. Whatever the workshop decides about the directories, `templates/` (roadmap step 4) has to settle the **item header** — the small set of fields every item carries, in one place, in one form.

### 7. Agents and skills are owned per project

Unlike methods that ship a fixed cast of shared agents, Ariane states that **each project owns its agents and skills**, referenced from its `INDEX.md`. The home level only registers the user's global skills. The method itself ships exactly one agent: **Ariane**, the guide — it resolves the right level, reads the indexes, answers "where are we / what's next", and operates the lifecycle.

Third-party skill packs (e.g. BMAD, FSD skills) follow the lockfile philosophy: the user's content repository holds a **registry** — name, source, version — and the CLI **installs** them at `init`/`update` into the platform's skill location. Their content is **never duplicated** into the user's `_ariane` repository.

### 8. Agent definitions are platform-neutral

An agent is defined once in a neutral `AGENT.md` (persona, mandate, rules — no platform assumptions), plus thin **adapters** per platform (first: a Claude skill in the Agent Skills format, invocable as `/ariane`). The same contract applies to project-owned agents.

### 9. Configuration

Ariane defines a **logical contract**: its configuration is a TOML file at the XDG path `~/.config/ariane/config.toml` (first key: `dir_name = "_ariane"`). How that path is materialized is the user's choice:

- **Default**: the file is versioned in the user's content repository (`<central>/config/`) and `~/.config/ariane` is a symlink into it — set up by `ariane init`.
- **Delegated**: users who manage their configurations with a dedicated tool (a dotfiles manager such as dotflies — Ariane's sibling project —, GNU stow, chezmoi…) own that path themselves; Ariane suggests this but never requires it.

Either way, `ariane doctor` only checks that the file exists and parses.

### 10. Tooling: the `ariane` CLI

Materialization relies on local state (a central clone, symlinks, a global git excludes entry) that can silently drift. The method therefore ships a small CLI: an **`ariane` binary written in Rust** ([ADR-0005](docs/adr/0005-write-ariane-in-rust.md)) — a single static binary with no runtime dependency, because the tool that diagnoses a machine must not depend on that machine being healthy (or on Node being installed at all). Distributed through a Homebrew tap (`brew install slashome/tap/ariane`) and GitHub Releases binaries — never added as a dependency of host repositories, in line with the zero-footprint rule.

> The no-runtime-dependency argument above is **not** what chose the language: it is equally true of Go, Rust, Zig and C, and it rules out Node and Python without separating the candidates that were actually in contention. ADR-0005 settles that separately, on the ground that Ariane's closed set is on the *output* side — the finding-to-remedy contract of [ADR-0004](docs/adr/0004-write-or-judge.md) — and it drops the `go install` channel this section used to advertise.

Core commands:

- **`ariane doctor`** (implemented first) — a linter for the local deployment. It verifies that the central content clone exists, that every declared node's `_ariane` symlink resolves into the right subtree, that the global git excludes file contains the configured `dir_name`, that the config parses, that each node has an `INDEX.md`, and that no node's `HANDOFF.md` (§11) is stale — a handoff whose receipt names a commit older than the node's current head means work has landed since the last one was picked up. Run from anywhere, it diagnoses the whole tree; run inside a node, it focuses on it. Read-only: it validates by hand-made setups as well as CLI-made ones.
- **`ariane init`** — bootstraps a machine: clones the user's content repository, creates the symlinks, adds `dir_name` to the global git excludes, materializes the config, and installs the registered skill packs.
- **`ariane update`** — re-syncs an existing setup: refreshes links for new nodes and updates skill packs to their registered versions.
- **`ariane tasks`** — the one read command: every item of the tree in a single cross-cutting view, which no `INDEX.md` can give. Each index answers for its own node — that is §2 working as designed — but *"what is on my plate, everywhere at once"* crosses them all, and nothing in the method answers it today. A throwaway prototype lives in [`tools/`](tools/) until this ships, and it is deleted when it does.

### 11. `HANDOFF.md` — passing the work on

Everything above records what a node **is**: its state, its items, its decisions. None of it records what is **in flight** — the half-finished change, the question waiting on an answer, the assumption that has not been checked yet, the reason the obvious approach was abandoned an hour ago. That knowledge exists only in the session that produced it and dies with it. Whoever comes next — the user on another machine, a fresh agent session, the same user in three weeks — restarts from the code, which records what was *built* and never what was being *attempted*.

Ariane therefore defines one conventional file per node, **`HANDOFF.md`**, holding the live state of the work in progress.

**It is hosted like everything else.** It lives in the central content repository at its node, materialized through the node's `_ariane/` directory, and is **never committed into the host repository** — §4 and §5 apply to it unchanged. The zero-footprint rule has no exception.

#### Its normal state is empty

Three states must stay distinguishable, because confusing the last two is what costs a day of work:

| The file | Means |
|---|---|
| absent | this node has never handed anything over |
| a receipt line only | nothing in flight — the last handoff was picked up |
| content | work is in flight, read it before anything else |

A consumed handoff is emptied down to a receipt: **who picked it up, when, and at which commit.** The commit is the part that matters — "picked up on 14 August" says nothing about whether work happened since, while a commit lets `ariane doctor` state that twenty-three commits have landed since the last handoff was consumed, and that the node has been running blind ever since.

#### Why not a section of `INDEX.md`

Because they are not two contents, they are **two incompatible writing disciplines**:

| | `INDEX.md` | `HANDOFF.md` |
|---|---|---|
| Written by | editing, accumulating, curating | wholesale replacement |
| Emptied | never | routinely — that is its resting state |
| Lifetime | that of the node | that of a session |

Putting a *replace-this-whole-block-every-time* region inside a *preserve-and-enrich* file, and handing it to an agent, is how durable memory eventually leaves with the replacement. Two files means two writing regimes, and a truncation can then never reach the memory. This is a safety argument, not a stylistic one.

#### Structure: borrowed from clinical handoff

The problem is not new, and the field that took it most seriously is medicine: a *shift handoff* transfers a patient between two teams, and information lost at that boundary kills people. The answer there is a short document with a fixed structure — SBAR. Ariane takes the same four movements:

- **Situation** — where the work stands right now, in one or two sentences.
- **Background** — how it got here: what landed, what was decided, and *where that decision is recorded* — a pointer, never a copy.
- **Assessment** — what is blocked and on whose decision, what is uncertain, and above all what is **verified** versus what is merely **asserted**.
- **Recommendation** — the next action, concretely, and what to do first.

Four rules keep it honest:

1. **It is a snapshot, not a log.** Rewritten wholesale, never appended to. The reader must find the current state, not reconstruct it from entries.
2. **It never becomes a second source of truth.** Decisions, stories and conventions live in `_ariane/` and are pointed at, never copied. Where a fact exists in both, the durable artifact wins.
3. **It is written or emptied in the same commit as the work it describes.** A stale handoff is worse than none: it is trusted, and it lies.
4. **What was verified carries its proof.** "The suite passes", "the check is green", "it is deployed" each carry the command that proves it, so the next reader re-runs instead of believing.

#### Whose job it is

Reading the handoff when a session opens, consuming it down to its receipt, and writing it before a session ends belong to the **mandate of the Ariane agent** (§7) — defined in `AGENT.md` and exposed by the adapters (§8). Not a separate skill: the method ships one agent, and skills are owned by projects. A method-level skill would be a third kind of shipped artifact whose authority over `AGENT.md` would have to be defined, for no gain.

`INDEX.md` and `HANDOFF.md` must not be merged: one is the durable, cumulative state of a node, the other the volatile state of one session's work in flight. One is memory, the other is a baton.

## Roadmap (v1)

Each step is one reviewable PR:

1. ~~Basic README~~ · ~~This plan~~
2. Item lifecycle workshop — settle the lifecycle and its directory layout (§6), likely run as an elicitation/brainstorming session using BMAD's skills
3. `SPEC.md` — the concepts above, normatively specified
4. `templates/` — `INDEX.md`, `HANDOFF.md` (§11), `story.md`, `task.md`, `conventions.md`, `config.toml`
5. `agents/ariane/AGENT.md` + `adapters/claude/skills/ariane/SKILL.md` — the Ariane agent
6. Reference instance — a documented walkthrough of bootstrapping a user's `_ariane` repository
7. `ariane` CLI (Go, single static binary) — `doctor` first, then `init` and `update`

Later: more CLI commands (`ariane link` to materialize a new node), more adapters (AGENTS.md, Cursor, subagents), multi-user projects, migration guides.

## Positioning vs BMAD

BMAD is ceremony-driven (PRD → architecture → epics → stories), greenfield-oriented, and ships a fixed cast of shared agent personas. Ariane is **file-driven** (`INDEX.md` is the source of truth), **brownfield-first** (continuous work on living projects), **hierarchical** (home → project → repo), and **decentralizes agents to the projects that own them**. Some BMAD ideas remain worth borrowing at the item level (story formats, course correction, retrospectives); they will be re-expressed within Ariane's structure rather than imported wholesale.

They are **complementary, not exclusive**: an Ariane user can perfectly use BMAD. It is registered as a skill pack in the user's content repository and installed by `ariane init`/`update` (§7) — never vendored. As for BMAD's working folders (`_bmad/`, `_bmad_output/`): Ariane does not track stray tool folders. Either the user points BMAD's configurable output location **inside the node's materialized `_ariane/` directory** — and the central repository captures those artifacts like any other content — or they remain ephemeral and git-ignored. No conflict, one rule: persistent knowledge lives in `_ariane`, everything else is disposable.

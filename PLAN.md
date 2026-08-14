# Ariane — Design Plan

> This document is the founding plan of the method. It will progressively be superseded by `SPEC.md` as concepts get formalized, and by the issue tracker for the roadmap.

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

> **Status: draft.** This lifecycle is a snapshot of the practice Ariane emerged from, not a settled decision. It is a foundation of the method and must be deliberately designed — see the roadmap: a dedicated elicitation workshop (a good use case for BMAD's brainstorming/elicitation skills) will settle it.

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

Materialization relies on local state (a central clone, symlinks, a global git excludes entry) that can silently drift. The method therefore ships a small CLI: an **`ariane` binary written in Go** — a single static binary with no runtime dependency, because the tool that diagnoses a machine must not depend on that machine being healthy (or on Node being installed at all). Distributed through a Homebrew tap (`brew install slashome/tap/ariane`), GitHub Releases binaries, and `go install` — never added as a dependency of host repositories, in line with the zero-footprint rule.

Core commands:

- **`ariane doctor`** (implemented first) — a linter for the local deployment. It verifies that the central content clone exists, that every declared node's `_ariane` symlink resolves into the right subtree, that the global git excludes file contains the configured `dir_name`, that the config parses, and that each node has an `INDEX.md`. Run from anywhere, it diagnoses the whole tree; run inside a node, it focuses on it. Read-only: it validates by hand-made setups as well as CLI-made ones.
- **`ariane init`** — bootstraps a machine: clones the user's content repository, creates the symlinks, adds `dir_name` to the global git excludes, materializes the config, and installs the registered skill packs.
- **`ariane update`** — re-syncs an existing setup: refreshes links for new nodes and updates skill packs to their registered versions.

### 11. `RESUME.md` — the handoff protocol

Everything above assumes the reader has the user's content repository. Someone who clones only a host repository — a collaborator, a fresh agent session, the user on another machine before `ariane init` — has none of it: no `_ariane/`, no `INDEX.md`, no history of what was decided and why. The work is unrecoverable from the code alone, because code records what was built, never what is in flight, what is blocked, or what is waiting on a decision.

Ariane therefore defines one conventional file, **`RESUME.md`, committed at the root of the host repository**, holding the live state of the work in progress. It answers, for a reader arriving cold with nothing but a clone: where the work stands, what just landed, what is in flight, what is blocked and on whose decision, what comes next, and how to verify all of that mechanically rather than take it on trust.

**This is a deliberate exception to §4 and §5, and the only one.** Every other Ariane artifact is physically hosted in the central content repository and merely materialized at the node, leaving zero footprint in the host repository. `RESUME.md` is the inverse: it is tracked *by the host repository*, travels with the code, and is visible to people who do not use Ariane at all. That is the entire point — it is an **exchange surface**, not memory. The private tree is where knowledge accumulates; `RESUME.md` is the baton handed over at the boundary.

The exception is contained by four rules:

1. **It is a snapshot, not a log.** Rewritten wholesale each time, never appended to. A reader must find the current state at the top of the file, not reconstruct it from entries.
2. **It never becomes a second source of truth.** Decisions, stories and conventions live in `_ariane/` (or in the project's own documents) and are *pointed at* from `RESUME.md`, never copied into it. Where a fact exists in both, the tracked artifact wins.
3. **It is updated in the same commit as the work it describes, or it is deleted.** A stale `RESUME.md` is worse than none: it is trusted, and it lies. Deleting it is always a legitimate move — the repository simply stops offering a handoff.
4. **It distinguishes what was verified from what was asserted.** Claims that a suite passes, a check is green, or a deployment is live carry the command that proves it, so the next reader re-runs rather than believes.

`INDEX.md` and `RESUME.md` are complementary and must not be merged: `INDEX.md` is the durable state of a node inside the private tree, cumulative and cross-repository; `RESUME.md` is the volatile, shareable state of one repository's work in flight. One is memory, the other is a baton.

## Roadmap (v1)

Each step is one reviewable PR:

1. ~~Basic README~~ · ~~This plan~~
2. Item lifecycle workshop — settle the lifecycle (§6), likely run as an elicitation/brainstorming session using BMAD's skills
3. `SPEC.md` — the concepts above, normatively specified
4. `templates/` — `INDEX.md`, `RESUME.md` (§11), `story.md`, `task.md`, `conventions.md`, `config.toml`
5. `agents/ariane/AGENT.md` + `adapters/claude/skills/ariane/SKILL.md` — the Ariane agent
6. Reference instance — a documented walkthrough of bootstrapping a user's `_ariane` repository
7. `ariane` CLI (Go, single static binary) — `doctor` first, then `init` and `update`

Later: more CLI commands (`ariane link` to materialize a new node), more adapters (AGENTS.md, Cursor, subagents), multi-user projects, migration guides.

## Positioning vs BMAD

BMAD is ceremony-driven (PRD → architecture → epics → stories), greenfield-oriented, and ships a fixed cast of shared agent personas. Ariane is **file-driven** (`INDEX.md` is the source of truth), **brownfield-first** (continuous work on living projects), **hierarchical** (home → project → repo), and **decentralizes agents to the projects that own them**. Some BMAD ideas remain worth borrowing at the item level (story formats, course correction, retrospectives); they will be re-expressed within Ariane's structure rather than imported wholesale.

They are **complementary, not exclusive**: an Ariane user can perfectly use BMAD. It is registered as a skill pack in the user's content repository and installed by `ariane init`/`update` (§7) — never vendored. As for BMAD's working folders (`_bmad/`, `_bmad_output/`): Ariane does not track stray tool folders. Either the user points BMAD's configurable output location **inside the node's materialized `_ariane/` directory** — and the central repository captures those artifacts like any other content — or they remain ephemeral and git-ignored. No conflict, one rule: persistent knowledge lives in `_ariane`, everything else is disposable.

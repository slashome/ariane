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

### 7. Agents and skills are owned per project

Unlike methods that ship a fixed cast of shared agents, Ariane states that **each project owns its agents and skills**, referenced from its `INDEX.md`. The home level only registers the user's global skills. The method itself ships exactly one agent: **Ariane**, the guide — it resolves the right level, reads the indexes, answers "where are we / what's next", and operates the lifecycle.

### 8. Agent definitions are platform-neutral

An agent is defined once in a neutral `AGENT.md` (persona, mandate, rules — no platform assumptions), plus thin **adapters** per platform (first: a Claude skill in the Agent Skills format, invocable as `/ariane`). The same contract applies to project-owned agents.

### 9. Configuration

TOML, versioned in the user's content repository: `config/config.toml`. First key: `dir_name = "_ariane"`.

### 10. Tooling: the `ariane` CLI

Materialization relies on local state (a central clone, symlinks, a global git excludes entry) that can silently drift. The method therefore ships a small CLI, distributed as an npm package exposing an **`ariane` binary** (scoped name, e.g. `@slashome/ariane`, if the short name is unavailable). It is installed globally (`pnpm add -g`) so it works from any directory, or run ad hoc with `pnpm dlx` — never added as a dependency of host repositories, in line with the zero-footprint rule.

First command: **`ariane doctor`** — a linter for the local deployment. It verifies that the central content clone exists, that every declared node's `_ariane` symlink resolves into the right subtree, that the global git excludes file contains the configured `dir_name`, that the config parses, and that each node has an `INDEX.md`. Run from anywhere, it diagnoses the whole tree; run inside a node, it focuses on it.

## Roadmap (v1)

Each step is one reviewable PR:

1. ~~Basic README~~ · ~~This plan~~
2. `SPEC.md` — the concepts above, normatively specified
3. `templates/` — `INDEX.md`, `story.md`, `task.md`, `conventions.md`, `config.toml`
4. `agents/ariane/AGENT.md` + `adapters/claude/skills/ariane/SKILL.md` — the Ariane agent
5. Reference instance — a documented walkthrough of bootstrapping a user's `_ariane` repository
6. `ariane` CLI (npm) — `ariane doctor`, the local deployment integrity linter

Later: more CLI commands (`ariane init`, `ariane link` to automate materialization), more adapters (AGENTS.md, Cursor, subagents), multi-user projects, migration guides.

## Positioning vs BMAD

BMAD is ceremony-driven (PRD → architecture → epics → stories), greenfield-oriented, and ships a fixed cast of shared agent personas. Ariane is **file-driven** (`INDEX.md` is the source of truth), **brownfield-first** (continuous work on living projects), **hierarchical** (home → project → repo), and **decentralizes agents to the projects that own them**. Some BMAD ideas remain worth borrowing at the item level (story formats, course correction, retrospectives); they will be re-expressed within Ariane's structure rather than imported wholesale.

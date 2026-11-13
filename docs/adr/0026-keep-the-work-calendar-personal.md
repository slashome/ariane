# ADR-0026 — Keep the work calendar personal, in the personal content repository

- **Status:** Accepted
- **Date:** 2026-11-11
- **Concerns:** PLAN §3, §9; reflexion `slashome/ariane#R010`; [ADR-0013](0013-define-the-configuration-schema-in-code.md), [ADR-0021](0021-share-content-through-team-repositories.md), [ADR-0025](0025-estimate-in-hours-of-focused-work-per-node.md)

## Context

Hours of work ([ADR-0025](0025-estimate-in-hours-of-focused-work-per-node.md)) become dates only through a capacity: when does the owner work on what. The owner's week is split between nodes: `cmc` on weekdays during office hours, `slashome/pharmakon` on two evenings and a Saturday morning. Office hours are not all focused work: meetings, reviews and untracked tickets take a share of them, more on `cmc` than on a solo project. Days off apply to everything (holidays) or to one node (a client's closure).

The calendar describes a person, not a project. Three files exist where it could go, and each has a precedent against it:

- `node.toml` is shared with everyone who reads the node. Once `cmc` moves to `choosemycompany/cmc-ariane` ([ADR-0021](0021-share-content-through-team-repositories.md)), a colleague would inherit the owner's hours.
- `config.toml` is per machine. The amendment of 2026-10-23 to [ADR-0013](0013-define-the-configuration-schema-in-code.md) moved the dashboard pins out of it precisely because they did not follow the owner across machines.
- `settings.toml` follows the owner, but holds the interface's look, read on every request, and is written by the interface's theme editor.

Two nodes may also share hours. `cmc` has seven repository nodes below it; each one, asked alone, would believe it has the whole of `cmc`'s week.

## Decision

> **The work calendar is `calendar.toml`, at the root of the owner's personal content repository. Blocks belong to nodes; a node's forecast is computed on the nearest ancestor that has blocks, and shared by its descendants.**

1. **Content.** A `timezone`; global `days_off`; per node, weekly blocks, a `focus` coefficient in `]0, 1]` and node-specific `days_off`:

   ```toml
   version = 1
   timezone = "Europe/Paris"
   days_off = ["2026-12-24..2027-01-02"]

   [node."cmc"]
   focus = 0.6
   blocks = [{ days = "mon-fri", from = "09:00", to = "18:00" }]

   [node."slashome/pharmakon"]
   focus = 1.0
   blocks = [
     { days = "tue,thu", from = "20:00", to = "23:00" },
     { days = "sat", from = "10:00", to = "13:00" },
   ]
   ```

2. **Schema in code.** The schema is defined in Rust with its documentation, the file carries a `version`, and changes reach it through `ariane migrate`, as for `config.toml` ([ADR-0013](0013-define-the-configuration-schema-in-code.md)).
3. **An hour counts once.** Two blocks of different nodes that overlap are a `doctor` **error**, and the interface refuses to save a calendar that has one. Blocks of one node may touch, not overlap.
4. **Pooled capacity.** The forecast runs on the node that carries the blocks (`cmc`), over every task below it, in one sequence; a child (`cmc/cmc-lib`) displays its share of that sequence. A node with no blocks on itself or an ancestor has no dates ([ADR-0028](0028-compute-the-forecast-in-one-place.md)).
5. **Never in a team repository.** `calendar.toml` is only read from the personal content repository. A shared repository that holds one is reported, never read.
6. **Operated** by `ariane calendar [node]` (prints the blocks and the resulting hours per week) and `ariane calendar edit` (opens `$EDITOR`, then validates), and by the *Réglages* view of the interface, which shows the other nodes' blocks greyed out so that an overlap is visible before saving. That view separates what is shared with the team (`node.toml`: releases, estimates) from what is personal (`calendar.toml`).

> **Implemented on 2026-11-13 (S032), with these precisions.** Until shared repositories are mounted ([ADR-0021](0021-share-content-through-team-repositories.md)), the **personal repository** is the one the `repo` key of `config.toml` names, else the repository being worked on; a `calendar.toml` anywhere else is a `doctor` warning and is never read. Blocks are written in the calendar's single time zone and never cross midnight (`to` after `from`, `24:00` at the latest), so an overlap is a plain intersection of two intervals on a shared weekday. A node's **carrier** is the nearest node, itself or an ancestor, whose table has blocks; the carrier's `focus` and `days_off` apply to its descendants, so a table without blocks has nothing to say and is a `doctor` warning. An unknown node is an error, like an overlap, so `ariane calendar edit` refuses to save it. `ariane calendar edit` never rewrites the file: it edits a draft, saves it as written once valid, reopens the editor on the same draft with the errors in `#!` lines on top otherwise, and abandons when the draft is left unchanged or emptied. `GET /api/calendar` serves the calendar read-only; the *Réglages* view will write it.

## Consequences

- The calendar follows the owner across machines, versioned with the rest of their personal state, and no teammate ever sees it.
- A teammate who uses Ariane on the same shared node has their own calendar, so their forecast of the same releases differs. Accepted: in v1 the roadmap is one person, sequential.
- Pooling makes a child's dates depend on its siblings' work. That is true of the owner's week, and it is the only way the seven `cmc` repositories do not each claim all of it.
- `focus` is a guess until velocity exists ([ADR-0025](0025-estimate-in-hours-of-focused-work-per-node.md)); it is the first number velocity will correct.
- One more file at the root of the personal repository, with its own schema and migration chain, next to `settings.toml`.

## Alternatives considered

### Capacity in `node.toml`, as hours per week

Jira Plans' model: a capacity per team. Rejected: it is shared with the team once the node is ([ADR-0021](0021-share-content-through-team-repositories.md)), and hours per week on two nodes cannot be checked for overlap. Forty hours on `cmc` and nine on `pharmakon` may well be the same evenings.

### In `config.toml`

Per machine, outside the content. Rejected on the precedent of ADR-0013's amendment: what follows the owner cannot live in a file that does not.

### In `settings.toml`

Already personal and versioned. Rejected: it is the interface's presentation, written by its theme and pin editors; a calendar has its own validation (overlaps), its own command and grows with every holiday. Keeping them apart keeps their diffs, schemas and migrations apart.

### Read an external calendar (Google Calendar, Reclaim)

Reclaim's model, per category of task, is the owner's. Rejected for v1: it needs credentials and the network, and the forecast must run offline and be testable from a file. Importing days off from an external calendar can be added on top.

## Revisit if

- Several people work the same node and need a forecast of the team, not of one person. Capacity would then be several calendars pooled, which this file does not model.
- Blocks per node prove too rigid (work moves between nodes week to week), and a weekly budget per node with checked totals becomes the better model.

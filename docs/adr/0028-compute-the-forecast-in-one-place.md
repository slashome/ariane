# ADR-0028 — Compute the forecast in one place, and keep its warnings in the roadmap

- **Status:** Accepted
- **Date:** 2026-11-11
- **Concerns:** SPEC §1.3, §1.6; reflexion `slashome/ariane#R010`; [ADR-0004](0004-write-or-judge.md), [ADR-0012](0012-serve-the-interface-from-the-binary.md), [ADR-0022](0022-order-items-by-a-sparse-rank-in-their-header.md), [ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md) to [ADR-0027](0027-record-when-an-item-closes.md)

## Context

ADR-0023 to ADR-0027 give the roadmap its inputs: ordered releases, sizes priced in hours, a personal calendar, closing dates. Something has to turn them into dates, and three readers want the result: the interface's *Roadmap* section, the owner in a terminal, and agents answering "when will 0.4 be ready?".

The result is a projection. It changes with every edit, every passing day and every recalibration. Written into files, it would rewrite hundreds of them each time and claim a precision nobody has.

The owner also wants to say "I would like this by the 15th" without that wish deciding the order. The order is the owner's judgment, already written as `rank` ([ADR-0022](0022-order-items-by-a-sparse-rank-in-their-header.md)).

Deviations already have homes: `doctor` reports breaches of the file contract, offline, without opinion ([ADR-0004](0004-write-or-judge.md)); the synchronisation page shows gaps with the tracker. A task without a size or a missed wish is neither.

## Decision

> **The forecast is computed by one engine in the CLI, served to every reader, and never written. Target dates are checked, never obeyed. The roadmap shows its own warnings.**

1. **One engine, in Rust.** It reads the tree, the node's releases, estimates and the calendar, and returns the forecast. `ariane roadmap <node> [--json]` prints it; `GET /api/roadmap?node=<node>` serves the same JSON to the interface, which draws and never computes.
2. **Order.** Releases in their declared order, parking releases and tasks without release left out. Each task in its effective release (task, then story, then epic, [ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md) §3); an epic or story without tasks is one block at its provisional size ([ADR-0025](0025-estimate-in-hours-of-focused-work-per-node.md)). Within a release, tasks grouped by epic, then by story, each level and the tasks in the order of ADR-0022. `depends_on` is respected: a dependency on a story or an epic is a dependency on all its tasks. Grouping is a preference, dependencies are constraints: a task that cannot start yet is passed over for the next ready one, and its epic's bar shows a gap.
3. **Scheduling.** One person, sequential. Each task takes its hours ([ADR-0025](0025-estimate-in-hours-of-focused-work-per-node.md)) then its testing, out of the calendar's blocks for the node that carries them, times `focus` ([ADR-0026](0026-keep-the-work-calendar-personal.md)), from now on. Tasks already `done` are placed at `closed` ([ADR-0027](0027-record-when-an-item-closes.md)). Without a calendar the engine returns the same sequence in relative hours ("+36 h de travail") and no dates.
4. **Target dates.** A key `target`, the desired date, MAY on tasks, stories and epics; `target` MAY on a `[[release]]` entry. The engine never reorders for them. It reports a target the forecast misses, and a child whose target is later than its parent's.
5. **Warnings belong to the roadmap.** The forecast carries its warnings: tasks without size (counted at the default size), epics and stories estimated by a provisional size, items without story or epic, tasks without effective release, missed targets, contradictory targets, and dependency cycles the scheduler cannot order. `doctor` does not report them. Each view shows its own deviations: Health shows `doctor`, the synchronisation page the tracker, the roadmap its plan.
6. **Edits go through existing writes.** Dragging a task, a story or an epic in the Gantt writes its `release` or `rank`, stretching it writes `size` snapped to the five sizes (on an epic or story only while it has no tasks). The forecast is then recomputed.

> **Engine implemented on 2026-11-13 (T121), with these precisions; `ariane roadmap` and its route follow (T122).** The engine counts in focused hours from now and lays them on the carrier's blocks only at the end, so the sequence with and without a calendar is the same. With `tester = "others"`, the validation is measured in the carrier's focused hours, while the owner goes on; a task in the `waiting` category takes no hours and is a validation from now as long as its testing; a task in progress counts all its hours, since partial progress is not recorded. The `depends_on` of a story or an epic applies to all its work. A dependency on open work that is never scheduled is ignored and warned about; a cycle and the work waiting on it stay unscheduled. Work whose release no applicable list declares is unscheduled, as `undeclared_release`. When descendants of the carrier declare their own release lists, releases are ordered by their place in their list, then by the path of the node declaring it. `target` became a core key, which `doctor` only checks as a date. SPEC §3 describes the engine.

> **Amended 2026-11-13: a status at the testing stage, started work first, inherited dependencies (T132).** Three rules settle what §2 and §3 left open, at the delivery of T121.
>
> - **Testing stage.** A `[[workflow]]` status MAY declare `stage = "testing"` in `node.toml` (SPEC §1.5), and only a status of the `in_progress` category; any other category, or an unknown stage, makes the `node.toml` invalid, an error of `doctor` like any invalid workflow declaration. A task in such a status has done its work: its remaining hours are 0 and its remaining testing is all of it, in its `estimate` as in the forecast, where its work entry reads `stage: testing`. With `tester = "owner"` the testing takes the owner's hours; with `tester = "others"` it is a validation from now, like a task in the `waiting` category, which keeps its testing as a delay.
> - **Started work first.** Tasks of the `in_progress` category, the testing stage included, are scheduled before the rank order of everything else, whatever their release, since the owner is on them now; among themselves, release order then rank. Their release's span starts with them, so a later release can start before an earlier one. They still wait for their dependencies, which are not pulled forward. The report keeps showing work in its release, epic and story order.
> - **Inherited dependencies.** The `depends_on` of a story or of an epic applies to every task inside it, as if each task declared it; a dependency on a story or an epic is still one on all its work.

> **Command and route delivered on 2026-11-13 (T122).** `ariane roadmap [node] [--json]` and `GET /api/roadmap?node=<node>` give the same JSON. The forecast of a node below its carrier is computed on the carrier, then narrowed to the work of the node and its descendants, whose summaries are computed again from that work alone; the JSON names the node asked (`node`) and the carrier (`carrier`). SPEC §3.6 describes the reading.

## Consequences

- The CLI, the interface and agents read one answer. The engine is testable with fixtures and no browser.
- Nothing in the content records a forecast date; a diff never says that a date moved. To compare forecasts over time, the reader has to recompute from an older commit.
- The engine is a new, non-trivial part of the binary: calendar arithmetic across days off and time zones, dependency ordering, pooling across child nodes. Velocity (later) plugs in by changing its inputs, not its shape.
- `doctor`'s result stays independent of today's date and of the owner's calendar, so it remains a contract check that can run anywhere.
- Warnings are not visible outside the roadmap until the dashboard adds counters per source. A missed target is noticed only by someone who opens the roadmap.

## Alternatives considered

### Compute in the interface

The Gantt is there. Rejected: the CLI and agents would get no forecast, and a second implementation for them would drift from the first.

### Write forecast dates into items (`planned:`)

Readable without the binary, visible in diffs. Rejected: every edit and every day rewrites hundreds of files, two machines conflict on dates neither chose, and a written date reads as a commitment — the invented-dates problem every timeline tool has.

### Let target dates reorder the plan

Deadline-first scheduling. Rejected: the order is the owner's judgment ([ADR-0022](0022-order-items-by-a-sparse-rank-in-their-header.md)); a date that silently overrides it makes the plan unexplainable. A warning lets the owner reorder, which writes the decision down.

### Report planning gaps in `doctor`

One place for every problem. Rejected: a missing size is not a broken file, and a missed date depends on today and on a personal calendar, so `doctor` would change verdict with the clock and between people. A merged "À traiter" page was rejected for the same reason: each source keeps its own view, the dashboard can add up counters later.

## Revisit if

- Several people plan the same node and the scheduling has to run in parallel rather than in sequence.
- The engine gets too slow to answer on every edit, which would argue for caching, still not for writing dates into files.
- Owners routinely want a target to win over the rank, often enough that reordering by hand becomes a chore.

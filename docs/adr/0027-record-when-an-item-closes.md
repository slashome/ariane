# ADR-0027 — Record when an item closes

- **Status:** Accepted
- **Date:** 2026-11-11
- **Concerns:** SPEC §1.3, §1.6; reflexion `slashome/ariane#R010`; [ADR-0022](0022-order-items-by-a-sparse-rank-in-their-header.md), [ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md)

## Context

The roadmap shows the past: a finished task is drawn where it was actually finished, so the owner can see how a release went and, later, how long work really takes. The header records `created` but not when an item was finished. No file in the content carries such a date (`grep -rl '^closed:' tree | wc -l` in `_ariane` prints `0`).

Git is the only record today, and a poor one. `src/history.rs` reads the date of the **latest commit touching a file**, which is not the date its status changed: a later edit of the body moves it, renames are not followed (its own documentation says so), and a migration or a bulk move dates hundreds of files to one day. A status changed on Monday and committed on Friday is dated Friday.

## Decision

> **A core key `closed`, a date, is written whenever an item enters a status of the `done` category.**

1. **The key.** `closed`, ISO 8601 date `YYYY-MM-DD`, MAY on tasks, stories and epics. `doctor` warns on `closed` on an item whose status is not in the `done` category.
2. **Who writes it.** Every tool that changes a status writes it on the transition into `done`, with the current date: the CLI, the interface, the tracker connector on pull ([ADR-0011](0011-sync-external-trackers-through-connectors.md)), and `pharmakon`'s `build.py` when it sets `valide`. A transition out of `done` removes it. A transition between two `done` statuses keeps it.
3. **From now on.** Nothing back-fills the key. An older `done` item without `closed` is approximated from git — the first commit where its status line reads a `done` status — and the roadmap shows that date as approximate. When git gives nothing, the item is not drawn in the past. The approximation is never written into the file.
4. **No version change on its own.** An optional key leaves every file valid, as `rank` did ([ADR-0022](0022-order-items-by-a-sparse-rank-in-their-header.md)). It ships in version `2` if [ADR-0024](0024-remove-milestone-from-the-header.md) lands first.

## Consequences

- The past of the roadmap is exact for everything finished after the change, and honestly approximate before it.
- Measured durations become possible (from the `in_progress` transition to `closed`, read from git or from a later `started` key), which is what velocity needs ([ADR-0025](0025-estimate-in-hours-of-focused-work-per-node.md)).
- Every writer of statuses has to remember the rule. A writer that forgets leaves a `done` item without a date; `doctor` cannot tell it from an old one. The rule belongs in SPEC so external writers such as `build.py` follow it.
- A reopened and re-closed item keeps only its last closing date.
- `dropped` items get no date: they are not drawn in the past. If that turns out to be missing, it is the same key on one more category.

## Alternatives considered

### Always derive the date from git

No new key, nothing to write. Rejected: the content repository's history records commits, not work. Commit batching, body edits, renames and migrations all move the date, and nothing in the file says the date is wrong. Kept only as the approximation for items closed before the key exists.

### A datetime instead of a date

Closer to the hours of the forecast. Rejected: the past is drawn by day, `created` and `waiting_since` are dates, and a timestamp is noise in a hand-read header.

### A log of transitions, in a file per node

Every status change with its date, enough for durations as well as closing. Rejected: a second source of truth for the status, appended by every writer and conflicting on every machine that changes two items at once, which [ADR-0022](0022-order-items-by-a-sparse-rank-in-their-header.md) avoided for the order with the same argument.

## Revisit if

- Velocity needs start dates too, and `started` would be the second key of the same pattern. If a third follows, a transition log becomes worth its cost.
- External writers routinely forget the key, so most `done` items end up approximated anyway.

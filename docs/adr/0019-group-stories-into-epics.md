# ADR-0019 — Group stories into epics

- **Status:** Accepted
- **Date:** 2026-10-27
- **Concerns:** SPEC §1.1, §1.3, §1.4, §1.6; [ADR-0008](0008-give-every-item-a-versioned-front-matter.md)

## Context

A task groups nothing; a story groups tasks through their `parent`. Nothing groups stories. A large piece of work — sharing a content repository between several people, absorbing Knossos into Ariane — spans several stories, each with its own tasks, and today it lives only as prose in the body of one of them, or in a file the tool does not read: `_ariane` already keeps four such files under `tree/slashome/ariane/epics/`, invisible to `ariane tasks`, `show` and `doctor`.

## Decision

> **An epic is a fifth kind of item, in `epics/`, with the id prefix `E`. A story names its epic in `parent`; a task keeps its story.**

- **The kind comes from the directory**, like every other (SPEC §1.1): `epics/`, archived under `archives/epics/`. An epic takes a status of the node's workflow, as a story does.
- **One level per link.** Only a story names an epic in `parent`. An epic has no `parent` — the key is forbidden for its kind — and `doctor` reports a task, a reflexion or a ressource whose `parent` is an epic. A task belongs to an epic through its story, so a task moved between stories never leaves a stale epic behind.
- **Progress is computed, never written.** `ariane show` of an item with children gives their count and how many are done or dropped; for an epic, its stories. Nothing in the epic's header records it.

> **Amended 2026-10-27: epics in the interface.** Every row of `ariane tasks --json` and `/api/items` carries `epic`, the epic of a story or of a task's story, computed from the `parent` links. The interface lists epics under *Epics*, each with the progress of its stories and of their tasks; an epic's page lists its stories; a story's page names its epic; *Stories* groups by epic; and the search field filters any view with `epic:E002` or `epic:<node>#E002`.

## Consequences

- `ariane tasks --kind epic`, `ariane show <node>#E001` and `ariane search` read epics with no further change; `ariane schema --json` lists the kind and its prefix, so the interface learns them from the export.
- `ariane show --json` gains a `progress` object (`total`, `done`, `dropped`) on any item with children, stories included.
- An epic's progress counts its stories, not their tasks: a story half done counts as not done.

## Alternatives considered

### A `type: epic` on a story

No new kind: a story marked as an epic by a custom field. Rejected: the kind is the directory (SPEC §1.1), and a story that is an epic would have to be told apart from its own children in every list, filter and check.

### An `epic` key on tasks and stories

Each item names its epic directly. Rejected: a task would carry two links upwards that can disagree, and moving a task between stories would have to update both.

### Allow a task under an epic

Some work needs no story. Rejected for now: allowing it makes an epic's children of two kinds, and its progress a mix of stories and tasks.

## Revisit if

- Small tasks keep being wrapped in a one-task story only to reach an epic.
- An epic needs to group epics, which would make the level recursive rather than fixed.

# Ariane — Specification

> This document states what the method **requires**. It progressively supersedes [`PLAN.md`](PLAN.md), one concept at a time, starting with the item header. Where the two disagree on a concept specified here, this document is authoritative. The reasons behind each rule live in [`docs/adr/`](docs/adr/).

The core keys of this section, and the keys of a `[[release]]` entry and of the `[estimates]` table of `node.toml` with their inheritance and defaults, are also published by `ariane schema --json`, generated from the same tables the CLI validates against, and so are the keys of the work calendar (§2).

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

## 1. Item header

Decided in [ADR-0008](docs/adr/0008-give-every-item-a-versioned-front-matter.md). Migration between versions is governed by ADR-0009.

### 1.1 Scope

A **node** is a directory under `tree/` that has a `node.toml` (§1.5) or holds items; `tree/` itself is the home node. A node has no index file: its judgment — its role, the reasons for its priorities, its rules — lives in its items, in the item it concerns or in a ressource of the node ([ADR-0017](docs/adr/0017-keep-a-nodes-judgment-in-its-items.md)).

An **item** is a Markdown file inside one of a node's item directories (`tasks/`, `epics/`, `stories/`, `reflexions/`, `ressources/`, `archives/` and their subdirectories). `HANDOFF.md` and `README.md` are not items, and neither is a leftover `INDEX.md`, which `doctor` reports (§1.6).

The **kind** of an item is given by its directory. The header MUST NOT carry it.

An **epic** (`epics/`) is a large piece of work that groups stories; a story groups tasks ([ADR-0019](docs/adr/0019-group-stories-into-epics.md)). A story names its epic in `parent`, and a task keeps its story for `parent`, never the epic.

A backlog item is a task whose status belongs to the `backlog` category (§1.5), not a kind of its own. `backlog/` is a former item directory: an item found there MUST be moved into `tasks/` (`archives/backlog/` into `archives/tasks/`). Until it is, tools read it as a task.

### 1.2 Form

An item MUST begin with a YAML front-matter block delimited by lines containing exactly `---`. The block MUST be a mapping.

Keys MUST be written in English, in lower case, with `_` as the word separator. Values that belong to a vocabulary defined by this specification MUST use that vocabulary verbatim. Free-text values (`title`, workflow labels) MAY be written in any language.

### 1.3 Core keys

| Key | Type | Required | Values |
|---|---|---|---|
| `ariane` | integer | **MUST** | The schema version of this file. This specification defines version `2`; version `1` also carried `milestone`, which `ariane migrate` turns into `release` ([ADR-0024](docs/adr/0024-remove-milestone-from-the-header.md)). |
| `node` | string | **MUST** | The logical path of the node holding the file, e.g. `cmc/cmc-lib`. MUST equal the file's location under `tree/`; for an item of the home node, directly under `tree/`, it is the empty string `''`. |
| `id` | string | **MUST** | Matches `^[A-Za-z][A-Za-z0-9-]*$`. Unique within the node. |
| `title` | string | **MUST** | Non-empty. |
| `status` | string | MUST for tasks, epics and stories; MUST for reflexions; MUST NOT for ressources | A status id from the node's workflow (§1.5). Reflexions use `open`, `trial` (a decision applied on trial, not yet confirmed) or `settled`. |
| `type` | string | MAY | A classification declared by the node (`[fields.type]` in `node.toml`); free when undeclared. |
| `personas` | list of strings | MAY for stories; SHOULD NOT otherwise | The personas the story is written for, each an id declared under `[personas]` (§1.5), e.g. `cmc:dev`. |
| `parent` | reference | MAY; MUST NOT for epics | One item reference (§1.4). Only a story may name an epic. |
| `depends_on` | list of references | MAY | Item references (§1.4). |
| `size` | string | MAY | `XS`, `S`, `M`, `L`, `XL`: hours of focused work, priced by the node's `[estimates]` (§1.5). On a story or an epic, a provisional estimate that yields to the sum of its tasks once it has some ([ADR-0025](docs/adr/0025-estimate-in-hours-of-focused-work-per-node.md)). |
| `impact` | string | MAY | `low`, `medium`, `high`. |
| `priority` | string | MAY | `low`, `medium`, `high`. |
| `rank` | integer | MAY; SHOULD NOT for ressources | The item's place in the manual order of its node and kind, negative values included (ADR-0022). |
| `created` | date | SHOULD | ISO 8601 calendar date, `YYYY-MM-DD`. |
| `closed` | date | MAY for tasks, epics and stories; SHOULD NOT for reflexions; MUST NOT for ressources | When the item last entered a status of the `done` category, `YYYY-MM-DD` ([ADR-0027](docs/adr/0027-record-when-an-item-closes.md)). Written by the tool that changes the status, never by hand from memory. |
| `release` | string | MAY | The release the item is delivered in, a `name` of the node's effective `[[release]]` list (§1.5); free when no list applies. On a task it is the decision; on a story or an epic, a provisional forecast for its tasks that carry none ([ADR-0023](docs/adr/0023-plan-by-releases-declared-in-order-per-node.md)). |
| `target` | date | MAY | The desired date of a task, a story or an epic, `YYYY-MM-DD`. The forecast checks it and never reorders for it (§3, [ADR-0028](docs/adr/0028-compute-the-forecast-in-one-place.md)). |
| `refs` | list of strings | MAY | Pointers to ADRs, pull requests, documents. |
| `external` | list of strings | MAY | Links to external trackers, each of the form `<connector>:<id>`, e.g. `monday:2955884643` (ADR-0011). |
| `branches` | list of strings | MAY | The git branches the item's work lives in, each of the form `<owner>/<repo>:<branch>`, e.g. `slashome/ariane:feat/web-node-select`; a task may span several repositories. |
| `pull_requests` | list of strings | MAY | The pull requests of the item's work, as `https://` URLs. |
| `waiting_for` | list of strings | MUST when the status is in the `waiting` category; MUST NOT otherwise | The people the item waits for, each a handle declared under `[people]` (§1.5). |
| `waiting_on` | string | MAY when waiting; MUST NOT otherwise | Why it waits: `review` (a review of its work), `validation` (of its result, by a product manager or a client), `answer` (a reply, a decision, an information) or `other`. |
| `waiting_since` | date | SHOULD when waiting; MUST NOT otherwise | When the wait began, `YYYY-MM-DD`. |

**Order of items.** Items are ordered per node and per kind; the ranks of two nodes or two kinds are never compared. Within one node and kind, items carrying `rank` come first, by ascending `rank`; the others follow, by descending `priority` (`high`, `medium`, `low`, then none). Ties fall back to the ids in natural order (`T9` before `T10`). An item under `archives/` keeps its `rank` but the order ignores it. A tool that lists items across nodes keeps the nodes in tree order and applies this order within each. A tool that writes `rank` spaces ranks by 1000, takes the middle of the gap when an item is moved between two ranked items, re-spaces only the neighbours of the drop point when the gap is exhausted, and ranks every unranked item of the node and kind, in their displayed order, when an item is moved below one of them. A tool MUST NOT give an item a `rank` the user did not ask for.

**Closing date.** Every tool that changes the status of a task, an epic or a story — the CLI, the interface, a tracker connector, a host repository's script, an agent editing the header — MUST write `closed` with the current date when the new status is in the `done` category and the former one was not, MUST remove `closed` when the new status is in another category, and MUST keep it when both are in `done`. A reopened and re-closed item keeps only its last date. Nothing back-fills the key: for an item in `done` without `closed`, `ariane tasks --dates --json`, `ariane show --json` and the interface give `closed_approx`, the date of the commit where git last saw its status enter the `done` category, renames followed and the status read under the legacy key `statut` for the versions before the migration to version 1; it is an approximation, shown as such and never written to the file, and it is absent when git gives nothing.

**Effective release.** The release an item is planned in is its own `release`; for a task without one, its story's; for a task or a story still without one, the epic's. The nearest declaration wins: task, then story, then epic. Other kinds only have their own. `ariane tasks --json`, `ariane show --json` and the interface give each item its `effective_release`: the `name`, its `source` (`own`, `story` or `epic`), the item it comes `from` as `<node>#<id>`, its `position` in the delivery order of the item's node (from 0, `null` when the list lacks it or none applies) and whether it is `parking`; `null` when no header of the chain carries a release.

Any other key is a **custom key**. A custom key SHOULD be declared in the node's `node.toml` (§1.5). A tool that reads an item MUST preserve every key it does not know, unchanged, when it writes the item back.

The body of the item MUST NOT restate in prose a fact the header carries.

### 1.4 Identifiers and references

- An `id` identifies an item **within its node**.
- Inside the same node, an item is referenced by its bare id: `T022`.
- Across nodes, an item is referenced as `<node>#<id>`: `slashome/pharmakon#T022`.
- A file name MUST begin with the item's id followed by `-` and a slug: `T022-tirage-du-temperament.md`.
- An id SHOULD start with the prefix of its kind followed by a number: `T` for tasks, `E` for epics, `S` for stories, `R` for reflexions, `D` for ressources (`T038`, `E002`, `S004`, `R012`, `D002`). `ariane migrate` allocates ids this way, per node and per prefix.

### 1.5 `node.toml`

A node MAY carry a `node.toml` file at the root of its directory. It describes the node, declares custom fields, the status workflow, the releases, what sizes cost, the people items wait for and the personas stories are written for, and may name the node's development server.

**Description.** A top-level `description` key MAY describe the node: its role, its host repository, its stack, its sources of truth. It is a Markdown string, multi-line with TOML's `"""`, and it is **not inherited**: it describes the node whose `node.toml` holds it ([ADR-0017](docs/adr/0017-keep-a-nodes-judgment-in-its-items.md), amended). Durable rules and conventions of the node belong in its ressources, not here.

```toml
description = """
Shared front-end library of the `cmc` apps, hosted in `choosemycompany/cmc-lib`.

- Stack: Vue 3, TypeScript, Vite.
- Sources of truth: the design system named **flow** by ChooseMyCompany; the CI of the other repositories is aligned on this one.
"""
```

**Fields.** Each `[fields.<key>]` table declares one custom key:

- `type` MUST be one of `string`, `integer`, `number`, `boolean`, `date`, `ref`, `list<string>`, `list<ref>`, `enum`.
- When `type = "enum"`, `values` MUST list the allowed strings.
- `required` MAY be `true`; it defaults to `false`.

**Workflow.** Each `[[workflow]]` entry declares one status:

- `id` MUST be unique within the workflow and match `^[a-z][a-z0-9-]*$`. It is the value written in items, and it is stable.
- `label` is free text shown to humans. Changing it MUST NOT require any change to items.
- `category` MUST be one of `backlog`, `todo`, `in_progress`, `waiting`, `blocked`, `done`, `dropped`. `waiting` is for an item that waits for someone — a review, an answer — named in `waiting_for`; `blocked`, for one that cannot move until something else happens.
- `stage` MAY say where the work of the status stands within its category, for the forecast (§3). Its one value is `testing`, which only a status of the `in_progress` category MAY declare: the work is done, only its testing is left.

The order of entries is the display order.

```toml
[[workflow]]
id = "a-tester"
label = "À tester"
category = "in_progress"
stage = "testing"
```

**Releases.** Each `[[release]]` entry declares one release of the node, a delivered version of the product, even internal ([ADR-0023](docs/adr/0023-plan-by-releases-declared-in-order-per-node.md)):

- `name` MUST be given, non-blank and unique within the list; it is free (`1.8.0`, `0.3`, `mango`) and is the value items write in `release`.
- `parking` MAY be `true` to mark a release that is never scheduled and never closed, such as `plus-tard`; it defaults to `false`.
- `target` MAY give a desired date, `YYYY-MM-DD`, as a TOML date or a string. It only checks a forecast; it never changes the order.

No other key is allowed. The order of entries is the delivery order.

```toml
[[release]]
name = "1.0"
target = 2027-03-01

[[release]]
name = "2.0"

[[release]]
name = "plus-tard"
parking = true
```

**Estimates.** An `[estimates]` table says what each size costs in hours of focused work, and how testing is added ([ADR-0025](docs/adr/0025-estimate-in-hours-of-focused-work-per-node.md)). Every key is optional:

- `XS`, `S`, `M`, `L`, `XL` give the hours of that size, a number above zero. A table cannot add a sixth size.
- `default` is the size an item without `size` counts as, one of the five; its hours are an assumption.
- `test_ratio` is the hours of testing added after each hour of work, a number zero or above; `0` removes testing.
- `tester` is who tests: `owner`, whose hours the testing consumes, or `others`, whose testing only delays the release.

No other key is allowed. A node with no `[estimates]` anywhere above it uses:

```toml
[estimates]
XS = 2
S = 4
M = 12
L = 24
XL = 48
default = "M"
test_ratio = 0.25
tester = "owner"
```

The hours an item weighs are computed, never written: a task weighs the hours of its `size`, else those of `default`; a story with tasks weighs the sum of its tasks and an epic with stories the sum of its stories, their own `size` ignored; a story or an epic not split yet weighs its provisional `size`, else `default`. Dropped work counts in no sum, and a story or an epic whose children are all dropped weighs 0 hours. Finished work has nothing remaining; a task in a status of the `testing` stage has no hours of work remaining, and all its testing. Each piece of work adds `hours × test_ratio` of testing, priced by its own node. `ariane tasks --json`, `ariane show --json` and the interface give each task, story and epic its `estimate`: `hours` and `test_hours`, the `remaining_hours` and `remaining_test_hours` of the work neither done nor dropped, the `tester`, the `basis` (`size`, `provisional`, `default` or `tasks`), the `size` priced, and how many of the summed `parts` are `guessed` from a default or provisional size.

**People.** Each `[people.<handle>]` table declares one person an item may wait for: `name` MUST be given, free text; every other key is the person's id in a tracker, such as `monday = 58294966`. A handle matches `^[a-z][a-z0-9-]*$`.

**Personas.** Each `[personas."<id>"]` table declares one persona a story may be written for ([ADR-0018](docs/adr/0018-name-the-personas-a-story-is-written-for.md)): `name` MUST be given, free text, the readable label; no other key is allowed. An id SHOULD read `<group>:<role>`, such as `cmc:dev` or `user:rh`; it is a quoted TOML key because of its colon:

```toml
[personas."cmc:dev"]
name = "Développeur CMC"

[personas."user:rh"]
name = "Responsable RH"
```

**Inheritance.** A node inherits the `node.toml` of each of its ancestors, from the root down:

- `[fields]`, `[estimates]`, `[people]` and `[personas]` accumulate key by key. A descendant MAY redeclare a key, an estimate, a person or a persona inherited from an ancestor, and the descendant's declaration wins.
- A `[[workflow]]` declared in a node replaces the inherited workflow **as a whole**; entries are never merged.
- A `[[release]]` list declared in a node replaces the inherited list **as a whole**, the same way. A node with no list anywhere above it has none, and its items keep free `release` values.
- `description` and `[proxy]` (or `[[proxy]]`) are never inherited.
- A node with no workflow anywhere above it uses the **default workflow**: one status per category, whose `id` equals the category.

**Tracker.** A `[tracker]` table binds the node to an external tracker. Its content is specified with the connectors (ADR-0011) and is out of scope of this section.

**Proxy.** A `[proxy]` table gives the node's development server a local name (ADR-0016): `host` MUST be a host name — labels of lowercase letters, digits and inner hyphens, separated by dots, such as `dreamcatchr.local` — and `port` MUST be the port it listens on, on `127.0.0.1`, from 1 to 65535. A node that runs several servers declares one `[[proxy]]` entry per server instead, each with its own `host` and `port`. No two entries of the tree MAY declare the same `host`. Routes are not inherited.

**Changing releases.** A release list is changed only in the `node.toml` that declares it, through `ariane release` or the interface (`POST`, `PATCH` and `DELETE /api/releases`, and `release` in `PATCH /api/items`), which run the same code; both write at once and never commit:

- `ariane release add <node> <name>` declares a release, last unless `--top`, `--before <release>` or `--after <release>` places it, with `--parking` and `--target YYYY-MM-DD`. The name is trimmed; a blank name, a name the list already has or a target that is not a date is refused. A node with no list anywhere above it starts its own.
- `ariane release move <node> <name>` with one of `--top`, `--bottom`, `--before <release>`, `--after <release>` changes its place in the delivery order.
- `ariane release set <node> <name>` with `--parking` or `--no-parking`, and `--target YYYY-MM-DD` or `--no-target`, marks it as a parking release or not and sets or removes its target date, the rest of its entry kept; at least one is required, and a target that is not a date is refused. `PATCH /api/releases` takes the same as `parking` (a boolean) and `target` (a date, or `null` to remove it).
- `ariane release rename <node> <old> <new>` renames it and rewrites `release` in every item, archived ones included, whose own value is the old name and whose node plans against that list.
- `ariane release remove <node> <name>` removes it, and is refused while an item of that scope still names it, listing them. Removing the last entry removes the list: the node then plans against its ancestors' list, or none.
- `ariane release assign <node>#<id> <name>` writes `release` on an epic, a story or a task that is not archived, and `--none` removes it. A name the item's effective list does not declare is refused, and so is any name when no list applies.

A node that inherits its list cannot change it: the change is refused, naming the node that declares the list, rather than copying the list down. Every change of `node.toml` keeps its comments, the order of its tables and the keys Ariane does not know, touches only the lines it changes, and writes nothing when the result would not be a valid `node.toml`.

**Changing a workflow.** Renaming a status id MUST go through `ariane status rename <node> <old> <new>`; removing a status still in use MUST go through `ariane status remove <node> <id> --into <other>`. Both commands rewrite `node.toml` and every affected item in one operation.

### 1.6 Validation

`ariane doctor` checks every item against this section. It never modifies a file (ADR-0004).

| Finding | Severity |
|---|---|
| No front-matter, or `ariane` key absent | error — remedy: `ariane migrate` |
| `ariane` lower than the version the CLI implements | error — remedy: `ariane migrate` |
| `ariane` higher than the version the CLI implements | error — the CLI is outdated |
| A required core key is missing | error |
| A core key has a value of the wrong type (`rank` not an integer, `release` not a string, `refs` not a list, an `external` entry not `<connector>:<id>`, a `branches` entry not `<owner>/<repo>:<branch>`, a `pull_requests` entry not an `https://` URL) | error |
| `node` does not match the file's location | error |
| `id` duplicated within the node | error |
| `status` not in the node's effective workflow | error |
| `release` not a `name` of the node's effective `[[release]]` list, on any kind, when a list applies | error |
| A status or `closed` present on a ressource | error |
| An item in `archives/` whose status category is neither `done` nor `dropped`, or a reflexion in `archives/` that is not `settled` | error |
| `parent` or `depends_on` points to an item that does not exist | error |
| `parent` on an epic | error |
| `parent` points to an epic from an item that is not a story | error |
| `size`, `impact`, `priority`, `created`, `closed` or `target` outside its vocabulary or format | error |
| A custom key declared in `node.toml` has a value of the wrong type | error |
| A status in the `waiting` category without `waiting_for` | error |
| A `waiting_for` handle declared under no applicable `[people]` | error |
| A `personas` id declared under no applicable `[personas]` | error |
| `personas` on an item that is not a story, `rank` on a ressource, or `closed` on a reflexion | warning |
| `waiting_for`, `waiting_on` or `waiting_since` on an item whose status is not in the `waiting` category | warning |
| `closed` on a task, an epic or a story whose status is not in the `done` category | warning — remedy: remove `closed`, or set a status of the `done` category |
| A required custom key is missing | error |
| A file name that does not begin with the item's `id`, `-` and a slug | error — remedy: `ariane fix` (renames to `<id>-<current name>` and rewrites the links); a human when the name is the bare id |
| An item in `backlog/` or `archives/backlog/` | error — remedy: `ariane fix` (moves it into `tasks/` or `archives/tasks/` and rewrites the links) |
| A custom key not declared in any applicable `node.toml` | warning — `milestone` on a version `2` file is one ([ADR-0024](docs/adr/0024-remove-milestone-from-the-header.md)) |
| A `node.toml` that does not parse, among others an `[estimates]` key that is neither a size nor `default`, `test_ratio` or `tester`, hours zero or below, a `default` that is not a size, a negative `test_ratio` or an unknown `tester`, a workflow `category` or `stage` outside its vocabulary, or a `stage` on a status of a category other than the one it requires (`in_progress` for `testing`) | error — the node falls back on its inherited configuration until the file is fixed |
| A `[proxy]` table or `[[proxy]]` entry whose `host` is not a host name, whose `port` is 0, or whose `host` another entry declares | error |
| An `INDEX.md` anywhere in the tree | warning — remedy: move the node's description into the `description` of its `node.toml`, its judgment into the item it concerns, its rules into a ressource of the node, then delete the file |

### 1.7 Examples

An item, `tree/slashome/pharmakon/tasks/T038-blessure-au-couteau-et-compte-a-rebours.md`:

```markdown
---
ariane: 2
node: slashome/pharmakon
id: T038
title: Blessure au couteau et compte à rebours
status: valide
type: code
parent: T035
depends_on:
  - T036
size: M
impact: high
created: 2026-09-12
release: "0.1"
refs:
  - DR 0018
branches:
  - slashome/pharmakon:feat/knife-wound
pull_requests:
  - https://github.com/slashome/pharmakon/pull/12
mod: NonLethalTakedowns
tests:
  - knife_wound
  - bleed_out
---

# T038 — Blessure au couteau et compte à rebours

…
```

Its node, `tree/slashome/pharmakon/node.toml`:

```toml
[fields.mod]
type = "string"
required = true

[fields.tests]
type = "list<string>"

[fields.type]
type = "enum"
values = ["code", "decision", "ecriture", "outil", "test"]

[[workflow]]
id = "a-faire"
label = "À faire"
category = "todo"

[[workflow]]
id = "en-cours"
label = "En cours"
category = "in_progress"

[[workflow]]
id = "a-tester"
label = "À tester"
category = "in_progress"

[[workflow]]
id = "valide"
label = "Validé"
category = "done"

[[workflow]]
id = "reporte"
label = "Reporté"
category = "backlog"

[[workflow]]
id = "abandonne"
label = "Abandonné"
category = "dropped"
```

## 2. Work calendar

Decided in [ADR-0026](docs/adr/0026-keep-the-work-calendar-personal.md). The calendar says when the owner works on what; it turns hours of focused work (§1.5) into dates.

### 2.1 Location

The calendar is `calendar.toml`, at the root of the owner's **personal** content repository, next to `settings.toml`. The personal repository is the one the `repo` key of `config.toml` names (`ariane config get repo`); when that key is not set, the content repository being worked on is the personal one. A `calendar.toml` in any other content repository MUST NOT be read: `doctor` reports it. A missing calendar is not an error: the roadmap then has no dates.

### 2.2 Form

```toml
version = 1
timezone = "Europe/Paris"
days_off = ["2026-12-24..2027-01-02"]

[node."cmc"]
focus = 0.6
blocks = [{ days = "mon-fri", from = "09:00", to = "18:00" }]

[node."slashome/pharmakon"]
focus = 1.0
days_off = ["2026-08-03..2026-08-14"]
blocks = [
  { days = "tue,thu", from = "20:00", to = "23:00" },
  { days = "sat", from = "10:00", to = "13:00" },
]
```

| Key | Type | Required | Rule |
|---|---|---|---|
| `version` | integer | MUST | `1`. Changed only by `ariane migrate`. |
| `timezone` | string | MUST | An IANA time zone, such as `Europe/Paris`. Every block is written in it. |
| `days_off` | list of strings | MAY | Days off of every node: dates `YYYY-MM-DD` and inclusive ranges `YYYY-MM-DD..YYYY-MM-DD`, the first day not after the last. |
| `node."<path>"` | table | MAY | One per node with hours of its own; `<path>` SHOULD be a node of the tree, quoted. |
| `node."<path>".focus` | number | MAY | The share of the blocks that is focused work, above `0` and at most `1`. Default `1`. |
| `node."<path>".blocks` | list of tables | MAY | The node's weekly blocks of work. |
| `node."<path>".blocks[].days` | string | MUST | Days `mon` to `sun`, as single days, ranges and lists: `mon-fri`, `tue,thu`, `sat`, `mon,wed-fri`. A range does not wrap around the week. |
| `node."<path>".blocks[].from` | string | MUST | `HH:MM`, `00:00` to `23:59`. |
| `node."<path>".blocks[].to` | string | MUST | `HH:MM` after `from`, `24:00` at the latest. A block never crosses midnight: one that does is written as two. |
| `node."<path>".days_off` | list of strings | MAY | Days off of this node only, written as the global `days_off`. |

No other key is allowed. Writing the file keeps its comments and layout: `ariane calendar edit` saves the text exactly as the owner wrote it.

### 2.3 Resolution

A node works on the blocks of the nearest node, itself or an ancestor, whose table has blocks: the **carrier**. Its descendants share the carrier's hours, its `focus` and its `days_off`, and the forecast runs on the carrier over every task below it ([ADR-0028](docs/adr/0028-compute-the-forecast-in-one-place.md)). A node with no carrier has no dates. The hours a carrier has per week are the sum of its blocks' lengths times their days, and its **focused hours** are those times `focus`.

`ariane calendar [node]` prints the blocks, focus and days off that apply to a node, the carrier when they are inherited, and the hours per week; without a node, those of the current directory's node, else of every carrier. `ariane calendar edit` opens the file in `$VISUAL` or `$EDITOR` (`vi` otherwise), from `version` and the system's time zone when it does not exist; it saves the file only when it has no error, and otherwise opens the editor again on the same text with the errors on top. Leaving the text unchanged, or emptying it, leaves the file as it was. `ariane calendar --json` and `GET /api/calendar` give the calendar, its valid part and its problems.

### 2.4 Validation

`ariane doctor` checks the personal calendar along with the tree. The findings of a node's table belong to that node's scope, and an overlap between two nodes to the scope of each of them; the others are reported when the whole tree is checked. Each finding is reported once.

| Finding | Severity |
|---|---|
| The file does not parse, has an unknown key, or lacks `version` or `timezone` | error |
| `version` higher than the version the CLI implements | error — the CLI is outdated |
| `timezone` that is not an IANA time zone | error |
| A day off, `days`, `from` or `to` outside its format, or a block whose `to` is not after its `from` | error |
| `focus` outside `]0, 1]` | error |
| Two blocks of **different** nodes that share time on a day: an hour counts once | error |
| Two blocks of one node that overlap; blocks MAY touch (`12:00` ends one, starts the other) | error |
| A `node."<path>"` table that names no node of the tree, such as a node renamed or moved | warning — its blocks still count, and still take part in the overlap checks |
| A `node."<path>"` table without blocks: its `focus` and `days_off` are never used | warning |
| A `calendar.toml` in a content repository that is not the personal one | warning — it is never read |

## 3. Forecast

Decided in [ADR-0028](docs/adr/0028-compute-the-forecast-in-one-place.md). The forecast says when each release of a node should be delivered. One engine computes it, in the binary; every reader gets its result, and nothing it computes is written to a file.

### 3.1 Scope

The forecast of a node runs on the node's **carrier** (§2.3) over every task, story and epic below the carrier, archived ones included: one sequence, shared by its descendants. Without a carrier, it runs on the node itself and its descendants, in hours from now and without dates.

### 3.2 Work

- A **piece of work** is a task, or a story or an epic not split yet (a story without tasks, an epic without stories), which counts as one block at its provisional size, else at the default size (§1.5).
- Its hours are those of §1.5: for open work its remaining hours and testing, the whole of them for work in progress, since partial progress is not recorded. Work in a status of the `testing` stage (§1.5) has done its work: it counts only its testing hours.
- Its release is its effective release (§1.3). Work of a parking release, work without effective release and work whose release no applicable list declares is listed as **unscheduled**, with the reason, and never scheduled. When the carrier's descendants declare release lists of their own, a release is identified by its name and the node declaring its list; releases are ordered by their place in their list, then by the path of that node.
- Dropped work is left out. Finished work (`done` category) takes no hours: it is shown in its release at its `closed` date, else at the date approximated from git (§1.3, marked approximate), else without a date.

### 3.3 Order

Releases come in their declared order. Within a release, work is grouped by epic, then by story, each level and the work in the order of items (§1.3), nodes in the order of their paths; the work without epic, and within an epic the work without story, comes last.

**Started work first.** Work of the `in_progress` category, `testing` stage included, is scheduled before all the rest, whatever its release, since the owner is on it now; among itself, in the order above, release first. Its release's span starts with it, so a later release can start before an earlier one. It still waits for its dependencies: one on work not started is scheduled in its own turn, never pulled forward.

`depends_on` is a constraint, the grouping a preference: the scheduler always takes the first piece of work in that order whose dependencies are delivered, and passes over the ones that are not, so a group can show a gap. A dependency on a story or an epic is a dependency on all its work; the `depends_on` of a story and of an epic apply to every piece of work inside them. A dependency on finished work is met; one on open work that is never scheduled (parking, no release, out of scope) is ignored and warned about. Work caught in a dependency cycle, and work waiting on it, stays unscheduled, and the cycle is warned about.

### 3.4 Scheduling

One person works in sequence, from now on. The scheduler counts in **focused hours from now**. A piece of work takes its hours, then its testing:

- with `tester = "owner"`, the testing takes the owner's hours right after the work;
- with `tester = "others"`, the testing is a **validation** that elapses from the end of the work, measured in the carrier's focused hours, while the owner goes on with the next piece of work; the work is delivered, and its release can end, only after it.

Work in a status of the `waiting` category takes none of the owner's hours: it is a validation from now that lasts its testing hours. Work in a status of the `testing` stage has no hours of work left: with `tester = "owner"` its testing takes the owner's hours, in its turn among the started work; with `tester = "others"` it is, like waiting work, a validation from now that lasts its testing hours. Work handed to someone this way, waiting or tested by others, ignores its dependencies. Statuses of the `backlog`, `todo` and `blocked` categories are scheduled alike.

With a carrier, focused hours become moments: the carrier's weekly blocks are laid out from now in the calendar's time zone, the global days off and the carrier's skipped, each block worth its length times `focus`, the hours accruing evenly over the block. A piece of work spread over several blocks has one segment per block. Moments are written RFC 3339 with the offset of the calendar's time zone; work further than ten years ahead has hours and no moment.

### 3.5 Result

The result gives, per release in delivery order, its epics then their stories then their work, each with its hours left, its span (from the start of its first scheduled work to the delivery of its last) and the first and last closing dates of its finished work. Each piece of work has its `stage` (`open`, `in_progress`, `testing`, `waiting` or `done`), its place in the sequence, its start, its end, its delivery, its segments (work and testing) and its validation. Every moment carries its focused hours from now and, with a carrier, its date. Without a carrier the same sequence comes in hours only (“+36 h”).

### 3.6 Reading the forecast

`ariane roadmap [node]` prints the forecast of a node, by default the current directory's node: a header naming the node whose blocks give the dates (the node itself, or the carrier and the other nodes sharing its sequence), or saying there is none and that `ariane calendar edit` adds blocks; then the releases in delivery order, their epics, stories and work indented, each with its hours left (work + testing), its span (dates in the calendar's time zone, or `+N h` from now without a carrier), its done and open counts or its stage, its `target` and `MISSED` when the forecast misses it; then the unscheduled work by reason and the warnings by kind, long lists cut with a count. `--release <name>` prints one release. `ariane roadmap <node> --json` and `GET /api/roadmap?node=<node>` give the same JSON: the engine's result, from `now` to the second.

**A node below its carrier** reads its part of the carrier's pooled sequence. The sequence is computed over the whole carrier; the result keeps the work whose node is the node asked or one of its descendants, at the place and moments the pooled sequence gives it (its `sequence` keeps its rank in the whole sequence, so it can show gaps). Every summary — the totals, each release, epic and story — is computed again from that work only: a span is the span of the node's part. A release is kept when it holds some of that work or when its list is declared by the node, an ancestor or a descendant. Unscheduled work and warnings keep the items of the node and its descendants, and the release warnings of the releases kept; a release's missed target is judged on all its work. The JSON says the node asked in `node` and the node whose blocks give the dates in `carrier` (`null` without a carrier, when the sequence runs on the node asked alone).

### 3.7 Warnings

The forecast carries its own warnings, counted by kind with the items they concern; `doctor` reports none of them ([ADR-0028](docs/adr/0028-compute-the-forecast-in-one-place.md) §5):

| Kind | Flags |
|---|---|
| `unsized` | A scheduled task without `size`, counted at the default size |
| `provisional` | A scheduled story or epic not split yet, weighed by its provisional or the default size |
| `without_story` | An open task without story |
| `without_epic` | A story with open work and without epic |
| `without_release` | Open work without effective release |
| `undeclared_release` | Open work whose release no applicable list declares |
| `missed_target` | Work, a story, an epic or a release whose `target` is earlier than its forecast delivery, or than its closing date for finished work; checked on future work only when dated |
| `contradictory_target` | A `target` later than the `target` of the story, the epic or the release that contains the work, or a story's later than its epic's |
| `dependency_cycle` | Work caught in a dependency cycle, left unscheduled |
| `unscheduled_dependency` | Work depending on open work that is never scheduled; the dependency is ignored |

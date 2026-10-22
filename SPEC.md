# Ariane — Specification

> This document states what the method **requires**. It progressively supersedes [`PLAN.md`](PLAN.md), one concept at a time, starting with the item header. Where the two disagree on a concept specified here, this document is authoritative. The reasons behind each rule live in [`docs/adr/`](docs/adr/).

The core keys of this section are also published by `ariane schema --json`, generated from the same table the CLI validates against.

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT** and **MAY** are to be interpreted as described in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

## 1. Item header

Decided in [ADR-0008](docs/adr/0008-give-every-item-a-versioned-front-matter.md). Migration between versions is governed by ADR-0009.

### 1.1 Scope

An **item** is a Markdown file inside one of a node's item directories (`tasks/`, `stories/`, `reflexions/`, `ressources/`, `archives/` and their subdirectories). `INDEX.md`, `HANDOFF.md` and `README.md` are not items.

The **kind** of an item is given by its directory. The header MUST NOT carry it.

A backlog item is a task whose status belongs to the `backlog` category (§1.5), not a kind of its own. `backlog/` is a former item directory: an item found there MUST be moved into `tasks/` (`archives/backlog/` into `archives/tasks/`). Until it is, tools read it as a task.

### 1.2 Form

An item MUST begin with a YAML front-matter block delimited by lines containing exactly `---`. The block MUST be a mapping.

Keys MUST be written in English, in lower case, with `_` as the word separator. Values that belong to a vocabulary defined by this specification MUST use that vocabulary verbatim. Free-text values (`title`, workflow labels) MAY be written in any language.

### 1.3 Core keys

| Key | Type | Required | Values |
|---|---|---|---|
| `ariane` | integer | **MUST** | The schema version of this file. This specification defines version `1`. |
| `node` | string | **MUST** | The logical path of the node holding the file, e.g. `cmc/cmc-lib`. MUST equal the file's location under `tree/`; for an item of the home node, directly under `tree/`, it is the empty string `''`. |
| `id` | string | **MUST** | Matches `^[A-Za-z][A-Za-z0-9-]*$`. Unique within the node. |
| `title` | string | **MUST** | Non-empty. |
| `status` | string | MUST for tasks and stories; MUST for reflexions; MUST NOT for ressources | A status id from the node's workflow (§1.5). Reflexions use `open`, `trial` (a decision applied on trial, not yet confirmed) or `settled`. |
| `type` | string | MAY | A classification declared by the node (`[fields.type]` in `node.toml`); free when undeclared. |
| `parent` | reference | MAY | One item reference (§1.4). |
| `depends_on` | list of references | MAY | Item references (§1.4). |
| `size` | string | MAY | `XS`, `S`, `M`, `L`, `XL`. |
| `impact` | string | MAY | `low`, `medium`, `high`. |
| `priority` | string | MAY | `low`, `medium`, `high`. |
| `created` | date | SHOULD | ISO 8601 calendar date, `YYYY-MM-DD`. |
| `release` | string | MAY | Free name of a delivery target. |
| `milestone` | string | MAY | Free name of an intermediate target. |
| `refs` | list of strings | MAY | Pointers to ADRs, pull requests, documents. |
| `external` | list of strings | MAY | Links to external trackers, each of the form `<connector>:<id>`, e.g. `monday:2955884643` (ADR-0011). |
| `branches` | list of strings | MAY | The git branches the item's work lives in, each of the form `<owner>/<repo>:<branch>`, e.g. `slashome/ariane:feat/web-node-select`; a task may span several repositories. |
| `pull_requests` | list of strings | MAY | The pull requests of the item's work, as `https://` URLs. |

Any other key is a **custom key**. A custom key SHOULD be declared in the node's `node.toml` (§1.5). A tool that reads an item MUST preserve every key it does not know, unchanged, when it writes the item back.

The body of the item MUST NOT restate in prose a fact the header carries.

### 1.4 Identifiers and references

- An `id` identifies an item **within its node**.
- Inside the same node, an item is referenced by its bare id: `T022`.
- Across nodes, an item is referenced as `<node>#<id>`: `slashome/pharmakon#T022`.
- A file name MUST begin with the item's id followed by `-` and a slug: `T022-tirage-du-temperament.md`.
- An id SHOULD start with the prefix of its kind followed by a number: `T` for tasks, `S` for stories, `R` for reflexions, `D` for ressources (`T038`, `S004`, `R012`, `D002`). `ariane migrate` allocates ids this way, per node and per prefix.

### 1.5 `node.toml`

A node MAY carry a `node.toml` file next to its `INDEX.md`. It declares custom fields and the status workflow.

**Fields.** Each `[fields.<key>]` table declares one custom key:

- `type` MUST be one of `string`, `integer`, `number`, `boolean`, `date`, `ref`, `list<string>`, `list<ref>`, `enum`.
- When `type = "enum"`, `values` MUST list the allowed strings.
- `required` MAY be `true`; it defaults to `false`.

**Workflow.** Each `[[workflow]]` entry declares one status:

- `id` MUST be unique within the workflow and match `^[a-z][a-z0-9-]*$`. It is the value written in items, and it is stable.
- `label` is free text shown to humans. Changing it MUST NOT require any change to items.
- `category` MUST be one of `backlog`, `todo`, `in_progress`, `blocked`, `done`, `dropped`.

The order of entries is the display order.

**Inheritance.** A node inherits the `node.toml` of each of its ancestors, from the root down:

- `[fields]` accumulate. A descendant MAY redeclare a key inherited from an ancestor, and the descendant's declaration wins.
- A `[[workflow]]` declared in a node replaces the inherited workflow **as a whole**; entries are never merged.
- A node with no workflow anywhere above it uses the **default workflow**: one status per category, whose `id` equals the category.

**Tracker.** A `[tracker]` table binds the node to an external tracker. Its content is specified with the connectors (ADR-0011) and is out of scope of this section.

**Changing a workflow.** Renaming a status id MUST go through `ariane status rename <node> <old> <new>`; removing a status still in use MUST go through `ariane status remove <node> <id> --into <other>`. Both commands rewrite `node.toml` and every affected item in one operation.

### 1.6 Validation

`ariane doctor` checks every item against this section. It never modifies a file (ADR-0004).

| Finding | Severity |
|---|---|
| No front-matter, or `ariane` key absent | error — remedy: `ariane migrate` |
| `ariane` lower than the version the CLI implements | error — remedy: `ariane migrate` |
| `ariane` higher than the version the CLI implements | error — the CLI is outdated |
| A required core key is missing | error |
| A core key has a value of the wrong type (`release` or `milestone` not a string, `refs` not a list, an `external` entry not `<connector>:<id>`, a `branches` entry not `<owner>/<repo>:<branch>`, a `pull_requests` entry not an `https://` URL) | error |
| `node` does not match the file's location | error |
| `id` duplicated within the node | error |
| `status` not in the node's effective workflow | error |
| A status present on a ressource | error |
| An item in `archives/` whose status category is neither `done` nor `dropped`, or a reflexion in `archives/` that is not `settled` | error |
| `parent` or `depends_on` points to an item that does not exist | error |
| `size`, `impact`, `priority` or `created` outside its vocabulary or format | error |
| A custom key declared in `node.toml` has a value of the wrong type | error |
| A required custom key is missing | error |
| A file name that does not begin with the item's `id`, `-` and a slug | error — remedy: `ariane fix` (renames to `<id>-<current name>` and rewrites the links); a human when the name is the bare id |
| An item in `backlog/` or `archives/backlog/` | error — remedy: `ariane fix` (moves it into `tasks/` or `archives/tasks/` and rewrites the links) |
| A custom key not declared in any applicable `node.toml` | warning |

### 1.7 Examples

An item, `tree/slashome/pharmakon/tasks/T038-blessure-au-couteau-et-compte-a-rebours.md`:

```markdown
---
ariane: 1
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
release: "1.0"
milestone: "0.1"
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

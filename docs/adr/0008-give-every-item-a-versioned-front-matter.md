# ADR-0008 — Give every item a versioned front-matter, extended per node

- **Status:** Accepted
- **Date:** 2026-10-20
- **Concerns:** PLAN §2, §6, §10

## Context

§6 already says it, measured on the reference instance: **an item's state is not readable without reading the item.** The directory carries the lifecycle step; everything else — status, priority, dates, ticket, owner — lives in prose, in whatever shape the session that wrote it found natural.

A full inventory of the reference instance on 2026-10-20 makes the shapes concrete. About **120 items**, written in **six header shapes**:

| Shape | Files | Example |
|---|---|---|
| Bullet list `- **État** : … / - **Priorité** : …` | ~51 | `cmc/cmc-lib/backlog/b3-pagination-cumulee.md` |
| Monday story bullets (`Monday`, `Epic`, `Sprint`, `Estimation`, `Owner`, `Statut`) | ~8 | `…/stories/2955884643-landing-trainees.md` |
| Blockquote `> Créée : DATE — statut : …`, in five variants | ~56 | `cmc-frontend/workspace/tasks/fusionner-stores-organization.md` |
| YAML front-matter holding only `node:`, then a blockquote | 10 | `slashome/me/reflexions/pantheon.md` |
| Nothing | 2 | |

Inside those shapes the vocabulary is free. An open item is "ouvert", "⏳ ouverte", "🟡 ouverte", "à faire", "à cadrer"; a finished one "✅ LIVRÉE", "FAIT", "clos", "archivé", "Migré". Emoji carry no stable meaning: 🔴 is *high priority* in one file and *in progress* in another. Seven archived items carry **two banners that contradict each other** — a `✅ TERMINÉ ET ARCHIVÉ` stacked above the original `statut : ⏳ à faire` — which is what hand-maintained duplication produces once the item moves. Size and impact exist nowhere as fields: five stories carry `Estimation : N` copied from Monday, impact appears only in prose. Monday ids appear in 23 files under five forms — a bullet, inline prose, a filename prefix, a branch name, or "à créer".

The prototype in `tools/` had to guess, and its guess failed on 84 of 191 live items. Everything that is now asked of the method — a kanban by status, a size × impact matrix, a dashboard aggregating nodes, synchronisation with an external tracker — reads fields. None of it can be built on prose.

One node already answers the question in practice. **pharmakon** (merged into the reference instance on 2026-10-20) carries **210 items with a structured YAML front-matter**: `node`, `id`, `titre`, `statut`, `type`, `release`, `jalon`, `parent`, `refs`, `depend`, plus two keys that only make sense there — `mod` and `tests`. Its statuses (`a-tester`, `valide`) are its own. It is the first real case, and it shows both what the header must allow (node-specific keys, a node-specific workflow) and what it must fix (keys in one natural language, which no tool outside that language can read).

## Decision

Every item carries a **YAML front-matter**, versioned per file, whose core keys are defined by Ariane and whose extensions are declared by the node.

**1. Versioned, per file.** The first key is `ariane: 1`, the schema version of that file. A file without it is version 0. Migration is the subject of ADR-0009; this ADR only requires that every file says which schema it follows.

**2. English keys, any-language content.** Keys and the vocabulary the method defines — status categories, sizes, impact levels — are English. Titles, labels and bodies are written in the owner's language. A key is an interface; a title is content.

**3. Core keys**, defined once by Ariane:

| Key | Meaning |
|---|---|
| `node` | Back-pointer to the node, as for `INDEX.md` (ADR-0001) |
| `id` | Identifier, unique **within the node** |
| `title` | Human title |
| `status` | A status id from the node's workflow |
| `type` | A node-declared classification (`code`, `design`, `research`…) |
| `parent`, `depends_on` | References to other items |
| `size` | `XS`, `S`, `M`, `L`, `XL` |
| `impact` | `low`, `medium`, `high` |
| `priority` | `low`, `medium`, `high` |
| `created` | ISO date |
| `release`, `milestone` | Free strings naming a delivery target |
| `refs` | Pointers to ADRs, PRs, documents |
| `external` | Links to external trackers, `<connector>:<id>` (ADR-0011) |

The **kind** of an item — task, story, backlog, reflexion, ressource — comes from its directory (§6), never from the header. Writing it twice is how the two drift. `type` is something else: a classification the node chooses, which pharmakon already uses.

**4. Ids are scoped per node**, and an item outside the node is referenced as `node#id` — `slashome/pharmakon#T022`. Inside the node, the bare `T022` suffices.

**5. A `node.toml` per node**, next to `INDEX.md`, declares what the node adds:

- **custom fields** with their type — pharmakon declares `mod: string` and `tests: list<string>`;
- **the status workflow.** Each status has a **stable `id`**, a free **`label`**, and exactly one **category** among a set fixed by Ariane: `backlog`, `todo`, `in_progress`, `blocked`, `done`, `dropped`.

A child node inherits its ancestors' `node.toml`: fields accumulate, and a workflow declared lower replaces the inherited one as a whole. A node with no `node.toml` anywhere above it uses the default workflow, one status per category.

**6. Statuses change through commands, not hand edits.** Relabelling a status is free — edit `label`, no item is touched. Changing an id is `ariane status rename <node> <old> <new>`; removing a status is `ariane status remove <node> <id> --into <other>`. Both rewrite `node.toml` and every item using the status in one operation. A status still in use cannot be removed without saying where its items go.

**7. Kinds without a lifecycle get their own vocabulary.** Reflexions use `open` and `settled`; ressources carry no status.

> **Amended 2026-10-21.** Reflexions gain `trial`, for a decision applied on trial and not yet confirmed — the reference instance already wrote "en essai", and forcing it into `open` lost the nuance. Only `settled` counts as closed. Ids allocated by `ariane migrate` take the prefix of their kind: `T` tasks and backlog, `S` stories, `R` reflexions, `D` ressources. The core keys, their types, requirements and vocabularies are held in one table in the CLI (`src/schema.rs`), which `doctor` and `migrate` iterate and `ariane schema --json` exports, so no surface redeclares them.

> **Amended 2026-10-22.** `backlog` is no longer a kind. The status workflow already has a `backlog` category, so a backlog item is a task waiting in it, and the `backlog/` directory said the same thing a second time — the drift this ADR set out to remove. The reference instance had nine such items, all tasks. `backlog/` is now a former item directory: its items are read as tasks, `doctor` reports them and `ariane fix` moves them into `tasks/`, links rewritten.

> **Amended 2026-10-23.** A seventh category, `waiting`, between `in_progress` and `blocked`: an item that waits for someone — a review, an answer — named in `waiting_for`. Kept apart from `blocked` so that a board shows whom the work waits for, and on whom to follow up ([ADR-0015](0015-name-whom-an-item-waits-for.md)).

> **Amended 2026-10-31.** A core key `rank`, an integer, gives an item its place in the manual order of its node and kind ([ADR-0022](0022-order-items-by-a-sparse-rank-in-their-header.md)). It is optional, so the schema version stays `1`: every existing file remains valid, and an older CLI keeps the key as an undeclared custom key.

**8. Unknown keys are tolerated and preserved.** A tool reading a key it does not know keeps it byte for byte, so an older CLI never destroys what a newer one wrote. `doctor` reports an undeclared key as a warning, not an error.

**9. The prose header disappears on migration.** One source per fact. Context that is not metadata — "validé par … le …" — moves into the body. git keeps the history of what the header said.

## Consequences

- Every surface that was waiting on fields can be built: kanban by status, matrix by size × impact, dashboards aggregated by category, tracker synchronisation.
- `doctor` gains a whole family of checks it can perform without guessing: unknown status, dangling reference, duplicate id within a node, outdated schema version.
- **Every existing item has to be migrated**, and about 120 of them start from prose. That cost is real and is the subject of ADR-0009. pharmakon's 210 items are cheaper — a key rename — but they are 210.
- The method now owns a vocabulary: six categories, five sizes, three impact levels. Changing it later is a schema version, not an edit.
- A second file per node, `node.toml`. Most nodes will not need one; those that do gain a place for data that would otherwise leak into `INDEX.md`.
- Per-node ids make references longer across nodes (`slashome/pharmakon#T022`). Moving an item between nodes changes its reference, and every item pointing at it must follow — a job for `update`, not for hand edits.
- Fixed categories mean a node cannot express a state Ariane did not foresee except by mapping it onto the closest category. That is the price of a dashboard that compares nodes.

## Alternatives considered

### One global set of statuses

Simplest for every tool: one list, no `node.toml`. Killed by the first real case: pharmakon's workflow is `a-tester` → `valide`, its testing step has no equivalent in a generic `in_progress`, and forcing it there erases exactly the information its owner tracks. The same will happen with a Monday board whose columns were designed by a team that never heard of Ariane.

### Workflows with editable categories

Let each node declare its categories too. The aggregation the dashboard needs — "how much is blocked, everywhere" — stops being comparable the moment one node's `blocked` means something another node calls `waiting`. Categories are the one layer that must be shared for a cross-node view to mean anything.

### Globally unique ids

`T022` once in the whole tree. It needs a counter, and the counter must be shared across machines working offline on clones of the same repository — two machines allocating `T023` the same afternoon is the normal case, not an edge case. Per-node ids need no coordination beyond the node, which is already the unit of work.

### Extensions declared in the front-matter of `INDEX.md`

One file fewer. It puts data a tool parses into the file whose purpose is judgment written by hand (ADR-0010), and it reproduces the defect ADR-0002 named: two writing regimes in one file.

### Extensions declared in the central `config.toml`

Everything in one place. That place is far from the node, so moving or archiving a node leaves its workflow behind, and the configuration becomes a registry of every node's internals — the shape ADR-0001 rejected for anchors.

### TOML front-matter, or a sidecar file per item

TOML front-matter (`+++`) is unusual for Markdown and unsupported by most renderers, including GitHub's; every item would display its metadata as body text. A sidecar `item.toml` doubles the file count and splits what moves together: an item renamed or archived without its sidecar is the drift this ADR exists to end. YAML front-matter is what pharmakon, ADR-0001's back-pointer and every static site generator already use.

## Revisit if

- A node needs a state that no fixed category can carry without lying — the categories are then wrong, not the node.
- Cross-node references turn out to be common enough that moving items between nodes becomes routine; per-node ids would then cost more than a coordination mechanism.
- A second natural language enters the tree's content with readers who cannot read the first, which would put the "keys in English" rule under pressure from the other side.

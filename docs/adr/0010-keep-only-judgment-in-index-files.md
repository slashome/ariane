# ADR-0010 — Keep only judgment in `INDEX.md`; compute item lists on demand

- **Status:** Proposed
- **Date:** 2026-10-20
- **Concerns:** PLAN §2, §6, §10 (`ariane tasks`)
- **Depends on:** [ADR-0008](0008-give-every-item-a-versioned-front-matter.md) (items carry their own status, machine-readable)

## Context

§2 makes `INDEX.md` the single entry point of a node: *"current state, priorities, and pointers"*. An agent or a human launched at any node reads it first. §6 adds that *"`INDEX.md` reflects the live state at all times"*, and that changing an item's state means *"moving the file and updating the node's `INDEX.md`"*.

In practice an index carries two kinds of content that look alike and behave in opposite ways:

1. **Judgment** — what the node is for, which priorities come first **and why**, what is blocked and on whom, what was settled, where to look. No tool can produce it; it is written by hand and it is the reason the file exists.
2. **Copies of item data** — tables listing every item with its status. Each row restates something the item file already says.

The second kind drifts, and the reference content repository shows it doing so:

- `tree/cmc/cmc-lib/INDEX.md` has a *Statut* column whose cells hold **priorities** — `🔴 **PRIORITÉ ABSOLUE**`, `🔴 **priorité haute**` — not statuses. The column means whatever the last writer needed it to mean.
- When the `cmc` node was triaged on 2026-08-17, statuses were copied into the new indexes **without checking a single one**: four items announced as "à faire" work delivered since June (`tree/slashome/ariane/INDEX.md`, *Chantiers ouverts*).
- The same status therefore lives in two places — the item and its row — and the priorities across repositories in three. Nothing says which one is true when they disagree.

ADR-0008 makes every item's status machine-readable. Once it is, every row of those tables is **derivable**, and a derivable value maintained by hand is a value that will eventually be wrong.

The content repository has already arbitrated the neighbouring question. Its reflection *README comme tableau de bord* (`_ariane`, `tree/slashome/ariane/reflexions/`) settled that `tree/INDEX.md` holds **judgment written by hand** and `README.md` holds **facts recomputed** from the tree — and that the two must not be mixed.

## Decision

**`INDEX.md` keeps only what no tool can compute. Item lists and statuses leave it; they are computed on demand.**

- **What stays:** the role of the node, its priorities **with their reasons**, its blockers and on whom they wait, the decisions taken and the pointers to them, links to child nodes. Prose, written by hand, owned by the user and the Ariane agent.
- **What goes:** every table or list whose rows restate item metadata — status, priority, size, ticket, date. A pointer to a specific item remains legitimate *when it carries a judgment* ("do this one first, because…"); a row that only mirrors a field does not.
- **Where the lists come from instead:**
  - the interface (ADR-0012) aggregates items live, per node and across nodes;
  - **`ariane tasks --node <path> --json`** serves the same aggregation to agents, which have no interface. The agent reading a node reads its `INDEX.md` for the judgment, then asks `ariane tasks` for the state.
- §6's sentence *"updating the node's `INDEX.md`"* on every state change no longer holds: changing an item's state means changing the item. The index is updated when the **judgment** changes.

Existing index tables are removed as each node goes through migration `0 → 1` (ADR-0009), not before — until a node's items carry a header, its table is the only place their state is written down.

## Consequences

- An index can no longer lie about an item's state, because it no longer states it.
- Indexes get much shorter. `cmc/cmc-frontend/workspace/INDEX.md` and its siblings are mostly tables today; what remains is the part worth reading.
- **An index read without tools shows less.** Someone opening `INDEX.md` on GitHub, or an agent on a machine without the binary, sees the priorities but not the item list. Accepted: the list is one `ls tasks/` away, and the state it would have shown was unreliable.
- `ariane tasks --json` becomes a dependency of the agent's mandate (`AGENT.md`), and its output format becomes part of the specification.
- The Ariane agent's write rule — *"every write ends by updating it"* — narrows to writes that change a judgment. `AGENT.md` has to say so.
- The `README.md` dashboard of the content repository, today hand-produced, becomes a natural output of the same aggregation.

## Alternatives considered

### Generated sections inside `INDEX.md`

Keep the tables, but regenerate them between markers (`<!-- ariane:begin(items) -->`) from the item headers. Attractive: the index stays self-contained and readable on GitHub. Rejected for the reason ADR-0002 separated `HANDOFF.md` from `INDEX.md`: **two writing regimes in one file**. A region that is replaced wholesale by a tool, inside a file that a human and an agent enrich by hand, is how hand-written content ends up inside the generated region and is lost at the next regeneration — or how a hand edit to the generated region is silently overwritten. The content repository's own `README.md` already carries such a marked region (`ariane:begin(status)`); it is regenerated, and nothing else is written there, precisely because the file has a single purpose.

### Delete `INDEX.md`

If lists are computed, let the interface and `ariane tasks` be the entry point and drop the file. Rejected: the judgment has nowhere else to live. Priorities and their reasons, blockers and whom they wait on, are not derivable from item headers — a `priority: high` field says *that*, never *why*, nor *before what*. And agents lose the written entry point §2 is built on: a file at a known path, readable with no tool, that an agent launched at any node finds first.

### Status quo

Keep hand-maintained tables. Rejected on the evidence above: a column that has come to hold priorities, four statuses copied unchecked, one state in two or three places. Once ADR-0008 makes item status readable, every hand-maintained row is a second source of truth whose only possible contribution is to disagree with the first.

## Revisit if

- Agents demonstrably fail to call `ariane tasks` and act on an incomplete picture of a node. The fix would first be in `AGENT.md`; only if that fails would a generated, separate file (not a region of `INDEX.md`) be worth considering.
- Readers without the binary — collaborators, GitHub browsing — become a primary audience, which multi-user work (post-v1) could make true.

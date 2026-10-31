# ADR-0022 — Order items by a sparse rank in their header

- **Status:** Accepted
- **Date:** 2026-10-31
- **Concerns:** SPEC §1.3, §1.6; story `slashome/ariane#S028`

## Context

Nothing lets the user say "this item comes before that one". Each surface sorts in its own way: `ariane tasks` and the kanban follow the order of paths in the tree, the backlog sorts by priority, impact and size, the dashboard by priority then id, stories and epics by category then id, reflexions by recency. `priority` has three values; it cannot order 150 tasks.

The order is a judgment of prioritization. [ADR-0017](0017-keep-a-nodes-judgment-in-its-items.md) puts a node's judgment in its items, and the order has to be the same for everyone who reads the content, so it cannot be a local setting like the dashboard pins ([ADR-0013](0013-define-the-configuration-schema-in-code.md)).

The content is files in git, edited by hand, by agents and by the interface, on several machines ([ADR-0021](0021-share-content-through-team-repositories.md)). Moving one item should rewrite as few files as possible, so that two machines reordering at the same time rarely conflict.

A simulation over 150 items (story S028) measured the files rewritten per move: dense integers `1..N` rewrite 51 on average over 2,000 random moves and all 150 when the last item goes to the top. Sparse integers with local re-spacing rewrite 1.04 on average, 13 at most, and 4 at most in the worst case of always inserting below the first item. Fractional string keys always rewrite one file, but their values grow to 42 characters in that worst case.

## Decision

1. **A core key `rank`**, an integer, optional, negative values allowed. SHOULD NOT on a ressource.
2. **One order per node and per kind.** Not per status: a kanban column is the filter of one status on that order, and an item that changes status keeps its place. Not per parent: the tasks of a story are a projection of the node's order. The ranks of two nodes are never compared; a view across nodes keeps the nodes in tree order and orders the items within each.
3. **The comparator**, shared by the CLI and the interface: ranked items first by ascending `rank`, then unranked items by descending `priority`, ties broken by ids in natural order. An archived item keeps its `rank` and the order ignores it. An absolute priority, once it exists (story S024), comes before every rank.
4. **Sparse ranks.** Ranks are spaced by 1000. A move between two ranked items takes the middle of their gap; at the top, the first rank minus 1000; at the bottom of the ranked items, the last plus 1000. When the gap is exhausted, only a window of neighbours around the drop point is re-spaced.
5. **Ranks are given as items are moved.** A new or migrated item has no rank. Dropping an item among unranked ones ranks the unranked items above the drop point first, in their displayed order. Neither `migrate` nor an agent writes a rank the user did not ask for.
6. **The schema version stays `1`.** Adding an optional key leaves every file valid.

## Consequences

- One move writes one file in nearly every case, a handful when a gap is exhausted. Two machines reordering different items never conflict.
- `ariane tasks` and `GET /api/items` change order: nodes and kinds as before, but the items of each sorted by the comparator instead of by path. A script that relied on the order of ids has to sort by itself.
- `priority` no longer orders ranked items: a `low` item may be ranked above a `high` one. It still labels, filters, and orders the unranked items.
- A diff says `rank: 3000` → `rank: 2500`, not "before T005". The tool that writes the rank has to say it in its commit message.
- Ranks of archived items stay in their files, unused, so that an item brought back out of the archives finds roughly its place.

## Alternatives considered

### A list of ids per node, in `node.toml` or a dedicated file

A move is a one-line diff in one file. But every reordering of a node lands in the same array, so two edits of neighbouring lines conflict. The list becomes a second source of truth for what exists: every creation leaves an item out of it, every archive or move leaves an orphan id, and `doctor` would have to reconcile both.

### Linked order, `after: T012`

Two items inserted after the same one fork the chain, archiving a link breaks it, cycles are possible, and a move rewrites two or three files.

### Fractional string keys, LexoRank style

One file per move, always. Rejected for readability: values such as `a0V7xk` cannot be read or written by hand, grow with repeated insertions in the same place, and an agent cannot place an item without the CLI. With fewer than 200 items per node and kind, the re-spacing of sparse integers stays local and rare.

### Dense integers `1..N`

Readable, but moving the last item to the top rewrites every file of the sequence.

## Revisit if

- A node holds thousands of items of one kind, and re-spacing starts touching tens of files per move.
- An order across nodes becomes a shared need rather than a personal one.
- Concurrent reorderings on shared repositories produce conflicts in practice.

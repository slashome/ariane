# ADR-0011 — Sync external trackers through per-node connectors

- **Status:** Accepted
- **Date:** 2026-10-20
- **Concerns:** PLAN §9, §10; [ADR-0004](0004-write-or-judge.md), [ADR-0008](0008-give-every-item-a-versioned-front-matter.md), [ADR-0012](0012-serve-the-interface-from-the-binary.md)

## Context

An item does not always live alone in `_ariane`. Work done for an employer is also tracked in that employer's tool, and the two records drift the moment either one moves. For the `cmc` nodes the external tool is Monday, and that is settled. The personal `slashome` nodes are not tracked anywhere today, and will most likely be bound to something else later — Google Tasks was the example given. So the requirement is **one item, possibly one external twin, chosen per node, possibly none.**

What the content repository holds today shows what happens without a convention. Measured on `_ariane` at `main`:

- **23 files carry a monday.com URL, pointing at 18 distinct pulses, and the binding takes five different forms** across the tree: a `- **Monday** :` bullet (9 files), an inline mention in prose ("ticket monday `3151821985`"), a filename prefix (`3035690590-migrer-stores….md`), a branch name (`doc_…_3035684627`), and the placeholder "Monday : à créer". No tool can tell which item is bound to which ticket without reading each file and guessing.
- **Status is duplicated by hand.** Archived stories carry `Statut : ✅ LIVRÉE — ticket Done / Deployed le 2026-06-26`: the Monday status, copied into prose, with the date it was copied. The copy is correct until the ticket moves again.
- **The need has already been formulated once.** The reflection `tree/slashome/ariane/reflexions/cycle-de-vie-des-items-et-verification-de-fraicheur.md` proposes an `ariane check` that compares items against Monday, after the `cmc` triage found four items announcing as "to do" work delivered in June.

Reproduce from the root of the content repository: `git grep -lE 'monday\.com' main -- 'tree/*.md' | wc -l` (23) and `git grep -ohE 'pulses/[0-9]+' main -- 'tree/*.md' | sort -u | wc -l` (18).

Two forces pull against each other. The core item schema of ADR-0008 must stay tracker-neutral, or binding a second tracker becomes a schema migration of every item in every node. And the owner explicitly wants **both directions**: changing a status in Ariane must reach Monday, and a status changed in Monday must come back.

## Decision

> **A tracker is reached through a connector. A node chooses its connector. The connector owns its fields, its mapping, and its refusal to guess.**

- **Connectors live inside the `ariane` binary**, behind one trait. The command is `ariane sync <connector>`; the server of `ariane serve` (ADR-0012) calls the same code path when the interface asks for a sync. There is no second program.
- **A node chooses its connector in `node.toml`**, under `[tracker]`, and child nodes inherit it (ADR-0008 inheritance). `cmc` declares `kind = "monday"` and its board; `slashome` declares nothing and syncs nothing.
- **The binding lives in the core `external` key only** (`external: [monday:2955884643]`), the one place a ticket is visible to tools that know nothing of Monday. No connector field repeats the identifier.
- **Each connector declares its own data fields, namespaced by its name** — `monday-status`, and whatever else it needs — and **its own mapping** to the core fields of ADR-0008. Which fields travel is decided by the connector, not by the core schema.
- **Mapping is by tracker identifiers, never by labels.** A Monday status label has a stable index inside its column; a column has a stable id. `node.toml` maps `"3" = "in_progress"`, not `"Working on it" = "in_progress"`. A label renamed in Monday changes nothing.
- **Sync is bidirectional, with a three-way comparison.** For every synced field, Ariane stores the last value both sides agreed on. If only one side moved since, its value is propagated. **If both moved, the item is suspended**: nothing is written on either side.
- **A change of board structure suspends the items it touches**: a status label that has no mapping, a mapped label that disappeared, a mapped column deleted or replaced. The connector reads the board structure on every run and compares it to the last one it saw.
- **Suspended items are reported, never resolved.** They appear in `ariane doctor` as findings whose remedy is a human decision with its resolutions enumerated — keep Ariane's value, keep the tracker's, edit the mapping — exactly the `Human(Resolutions)` shape of ADR-0004. The interface shows the same findings and offers the same choices.
- **v1 ships one connector: Monday.** The trait is designed against Monday alone and is expected to bend when the second one arrives.
- **Secrets never live in a file of the repository.** Where the Monday token is stored is deferred; the lead is the OS keychain through the `keyring` crate. Tracked as `slashome/ariane#T001` in the content repository.

`ariane sync` writes item files and `node.toml`, never commits (ADR-0012), and judges nothing beyond refusing to write: the findings it produces are re-observed by `doctor`, not passed to it.

## Consequences

- Binding a second tracker means writing a connector and declaring it in a node. No item outside that node changes, no schema version moves.
- The five forms of Monday reference collapse into one, but **only through migration 0→1 (ADR-0009)**: until an item has been migrated, its ticket is invisible to `sync`. The 23 files have to be bound one by one, and the five forms mean the parser will not catch all of them.
- An owner who edits a lot in both places will see suspended items regularly. This is the price of never overwriting, and it is intended — but if the list grows faster than it is cleared, sync becomes noise.
- The last-agreed value has to be stored somewhere per item and per field. Storing it in the item's front-matter keeps it versioned and visible, at the cost of a sync touching every bound file even when nothing user-visible changed.
- A trait designed against one implementation is a guess about the second. Google Tasks has no status column at all — only completed or not — so the mapping model will have to accept a connector that maps several Ariane statuses onto one external value, and decide which one comes back.
- The `cmc` content is an employer's data synced into a personal repository. That was already true by hand; sync makes it continuous. Nothing here changes who may read the content repository, but the connector must never push personal-node content to the employer's board, which inheritance guarantees only as long as `[tracker]` is declared at `cmc` and nowhere above it.

## Alternatives considered

### Monday-specific fields in the core schema

The simplest thing: `monday_id` and `monday_status` as core keys of ADR-0008. It dies on the second tracker. The personal nodes are expected to bind to something else; adding `gtasks_id` then means a schema version bump, a migration of every item in every node, and a core schema that grows by one employer's tool at a time. The namespaced connector fields cost one level of indirection and never touch the core.

### Last writer wins

Compare modification dates and keep the newest. Simple, and it **silently overwrites**. Monday's `updated_at` moves on any change to the pulse, including a comment or a subscriber, so an unrelated edit on Monday would erase a status change made in Ariane a minute earlier. The owner chose, explicitly, to be told rather than overwritten.

### One side always wins

Monday authoritative for bound items, or Ariane authoritative. Either makes the sync one-directional in practice: the losing side becomes a read-only mirror whose edits are discarded on the next run. The owner asked for both directions; this alternative is the refusal of the requirement, not a way of meeting it.

### Mapping by label

Readable in `node.toml` — `"Done / Deployed" = "done"` says what it means. It breaks the day someone renames the label on the board, which on a shared employer board is not under the owner's control. With identifiers, a rename is invisible; with labels, every bound item in that status suspends at once, or worse, stops matching and is treated as unmapped.

### A separate sync program

A dedicated tool, called by `ariane` or run on its own. Rejected by the owner: sync belongs in `ariane`, and has to be callable from the interface. A second program also means a second release, a second configuration reader, and a second implementation of the item parser that ADR-0008 makes normative — two parsers of one format drift.

## Revisit if

- The second connector cannot be expressed through the trait without a special case in the core — which would mean the trait was Monday's shape, not a tracker's.
- Suspended items accumulate faster than the owner clears them over several weeks, which would mean the three-way comparison is too strict for real use, and a per-field policy (for example "tracker wins on status, Ariane wins on title") is needed.
- Monday's API stops exposing stable label indexes, which would remove the ground the identifier mapping stands on.

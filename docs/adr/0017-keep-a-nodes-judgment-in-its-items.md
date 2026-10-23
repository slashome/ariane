# ADR-0017 — Keep a node's judgment in its items, drop `INDEX.md`

- **Status:** Accepted
- **Date:** 2026-10-23
- **Concerns:** PLAN §2, §6; SPEC §1.1, §1.5, §1.6
- **Supersedes:** [ADR-0010](archives/0010-keep-only-judgment-in-index-files.md)

## Context

ADR-0010 took the item tables out of `INDEX.md` and kept the file for **judgment**: the node's role, its priorities and their reasons, its blockers, its pointers. It rejected deleting the file because *"the judgment has nowhere else to live"*, and because agents needed a written entry point at a known path.

Both reasons have since fallen:

- **The interface carries the entry point.** `ariane serve` (ADR-0012) shows every node, its items, its board and its health; `ariane tasks --json` and `ariane show --json` give agents the same view. Nobody opens a node by reading its `INDEX.md` any more: the owner opens the interface, the agent asks the CLI.
- **The judgment has a place in the items.** A priority's reason concerns an item and reads best in that item's body; a blocker is a status of the `blocked` or `waiting` category with `waiting_for` (ADR-0015); a rule of the node, its role or how its work is organised is a **ressource** (`D` id) of that node, versioned, searchable with `ariane search` and shown by the interface like any other item.
- **What is left in the indexes drifts.** The content repository holds 21 `INDEX.md` files (`find tree -name INDEX.md | wc -l` in `_ariane`). They are the only Markdown of the tree that no command reads, checks or shows, so they rot quietly, and an agent reading one cannot tell its stale sentences from its true ones.

## Decision

> **A node has no `INDEX.md`. Its judgment lives in its items: in the item it concerns, or in a ressource of the node when it concerns the node as a whole.**

- **What makes a node.** A directory under `tree/` is a node when it has a `node.toml` or holds items; `tree/` itself is always the home node. An `INDEX.md` no longer makes a directory a node.
- **Where the judgment goes.** The reason a task comes first goes into that task (its body, its `priority`); a blocker is a status and `waiting_for`; a role, a rule or a convention of the node goes into a ressource of the node, `D` id. Pointers to child nodes are the tree itself.
- **What `doctor` says.** A node without `INDEX.md` is normal and no longer reported. An `INDEX.md` still in the tree is a **warning**, `leftover_index`, whose resolutions are to move each piece of its judgment into the item it concerns or into a ressource of the node, then delete the file. It is never read as an item, even inside an item directory.
- **How an agent reads a node.** State with `ariane tasks --node <node> --json`; judgment from the items — their bodies through `ariane show` or `ariane tasks --body`, and the node's ressources.
- **Bootstrapping a node.** Create its directory under `tree/`, with a `node.toml` when it needs a workflow, fields, people, personas or a tracker of its own, then materialize the `_ariane` symlink. No file registers it in its parent. (Amended below: the `node.toml` is always written, with a `description`.)

> **Amended 2026-10-23: the node's description lives in `node.toml`.** Since a `node.toml` is what makes a directory a node, the description of the node — its role, its host repository, its stack, its sources of truth, such as "design system named flow by ChooseMyCompany, CI aligned on cmc-lib" — goes into a top-level `description` key of that file: a Markdown string, multi-line with `"""`, **not inherited**. `/api/nodes` serves it and the dashboard card shows it under *About*. Durable rules and conventions of the node stay in its ressources, and the judgment about an item stays in that item. Bootstrapping a node writes its `node.toml` with a `description`. This narrows the alternative *Fold the judgment into `node.toml`* below: the one-paragraph identity of the node is data about the node and sits with the rest of it, while rules, which grow and are argued over, stay out of a file a tool parses.

## Consequences

- One file fewer per node, and no Markdown left in the tree that no tool checks.
- The judgment becomes searchable and visible in the interface, next to the items it concerns.
- **A node without items and without `node.toml` is not a node.** An intermediate directory such as `tree/slashome`, which held only an `INDEX.md` and child nodes, disappears from the node list; its children stay, and the interface's tree draws them at the top level. A directory that must be a node in its own right gets a `node.toml`, even an empty one.
- **The existing indexes have to be migrated by hand.** No tool can tell which sentence of an index belongs to which item; `doctor` lists them until each is emptied into the items and deleted.
- **A reader without the binary sees less.** Someone browsing the content on GitHub no longer finds a summary per node. Accepted, as ADR-0010 already accepted it for the item lists: that summary was the part that drifted.

## Alternatives considered

### Keep `INDEX.md` for judgment only (ADR-0010)

The status quo. Rejected: the file keeps a second place for judgment that no command reads or checks, which is the drift ADR-0010 removed for item lists, one level up. The interface and the CLI are now the entry point it was kept for.

### Generate `INDEX.md` from the items

A read-only summary per node, rebuilt by a command. Rejected: it restates what the interface already shows, and a generated file in the tree is one more thing to keep in step, committed with every change of every item.

### Fold the judgment into `node.toml`

A `description` or `rules` key in the node's configuration. Rejected: `node.toml` is data a tool parses (ADR-0008); prose there mixes two writing regimes in one file, the defect ADR-0002 named.

## Revisit if

- The interface stops being the way the owner opens a node — a reader without the binary becomes a primary audience, which multi-user work could make true.
- Ressources prove too coarse for a node's judgment, and its rules end up scattered across items where nobody finds them.

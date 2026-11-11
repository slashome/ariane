# ADR-0024 — Remove `milestone` from the header

- **Status:** Proposed
- **Date:** 2026-11-11
- **Concerns:** SPEC §1.3, §1.6, §1.7; [ADR-0008](0008-give-every-item-a-versioned-front-matter.md), [ADR-0009](0009-migrate-content-with-versioned-migrations.md), [ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md)

## Context

[ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md) keeps one level of target. The `milestone` key of SPEC §1.3 has no meaning left, but it is in use: the `0 → 1` migration of ADR-0009 maps `pharmakon`'s `jalon` to it (`src/migrate/v1/mod.rs`), and 200 files of `pharmakon` carry one, live items, stories and archives together (`grep -rl '^milestone:' tree/slashome/pharmakon | wc -l` in `_ariane`). No other node uses it (`grep -rl '^milestone:' tree | grep -v pharmakon` is empty).

In `pharmakon` every milestone sits under release `1.0`, and milestone `1.0` is the release itself. The pairs read as one ordered list: `0.1`, `0.2` … `0.9`, `1.0`, then `2.0`, `3.0`, `plus-tard`.

Two programs outside Ariane read the key: `pharmakon`'s `atelier/build.py` (as `jalon`, to close a milestone through its "Jouer et juger" task, to archive and to generate `PLAN.md`), and its `conventions.md`, which makes `jalon` mandatory under release `1.0`.

## Decision

> **`milestone` leaves the core keys. The schema goes to version `2`, and migration `1 → 2` turns every milestone into the item's release.**

1. **Migration `1 → 2`**, deterministic and idempotent like every migration after `0 → 1`: on an item carrying `milestone`, `release` takes the value of `milestone` and `milestone` is removed (`release: '1.0'` + `milestone: '0.3'` → `release: '0.3'`). An item without `milestone` only has its version raised. It runs on live items and on `archives/` alike, on every kind.
2. **Release values are normalized to strings.** A release written as a YAML number (`release: 1.0`, one file in `pharmakon`) is rewritten quoted, since a float loses `1.10` to `1.1`.
3. **The node's release list is written by hand, in the same change.** Migrations transform item files; `node.toml` has no version. `ariane migrate` prints the distinct release values it produced per node, so the owner can declare them in order. For `pharmakon`: `0.1` … `0.9`, `1.0`, `2.0`, `3.0`, `plus-tard` (parking).
4. **After the migration, `milestone` is a custom key.** On a version `2` file it is reported like any undeclared key; it is never read as a target.
5. **`pharmakon` moves in the same change**: `conventions.md` speaks of releases; `build.py` reads `release`, stops archiving ([ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md) §6) and only sets `valide`.

## Consequences

- The first versioned migration after `0 → 1`. It tests what ADR-0009 promised: a mixed repository where `pharmakon` is at `1` while the rest is at `2`, read correctly by every tool until it is migrated.
- Every reader of the header drops `milestone`: `src/tasks.rs`, `src/schema.rs`, `/api/items`, the interface's types and its filter. Readers keep accepting version `1` files, where `milestone` still appears.
- 21 stories of `pharmakon` carry a `release` and a `milestone`: migrated like tasks. Under ADR-0023 §3 their value becomes the provisional forecast of their tasks that declare none, which is legitimate; the owner may drop it where every task carries its own. The migration keeps them, since it cannot know whether every task of the story carries the same value.
- A release list of thirteen entries for one node, of which nine are internal. Accepted: they are what the owner plays and judges.
- `build.py`'s notion of a closed milestone becomes `ariane release close`; until both are shipped, the pending `pharmakon` change that archives on milestone close must not be merged against a node already at version `2`.

## Alternatives considered

### Keep the key, deprecated

Stop documenting `milestone` but keep reading it. Rejected: a key the roadmap ignores but the files still carry is the kind of silent drift ADR-0009 exists to prevent, and readers would keep two code paths forever.

### Concatenate into one release name (`1.0/0.3`)

Preserves both values exactly. Rejected: it invents a hierarchy inside a free string that no tool can order without parsing it, which is ADR-0023's rejected sort by another route.

### Let the agent migrate `pharmakon` by hand

200 files, one node. Rejected for the reasons of ADR-0009: not repeatable, not testable on a fixture, and the second machine would need the same run. The transformation is a one-liner; it belongs in the chain.

## Revisit if

- A node needs a second ordered level of targets under a release and ADR-0023 is revisited. The key would then come back with a declared list, not as a free string.

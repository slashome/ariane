# ADR-0030 — Let a release depend on releases of other nodes

- **Status:** Accepted
- **Date:** 2026-11-18
- **Concerns:** SPEC §1.5, §1.6, §3.2, §3.3, §3.7; task `slashome/ariane#T150`, story `slashome/ariane#S041`; amends [ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md) and [ADR-0028](0028-compute-the-forecast-in-one-place.md)

## Context

[ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md) orders the releases of one list by their place in it. [ADR-0028](0028-compute-the-forecast-in-one-place.md), as implemented by T121, pools the lists of the carrier's descendants on the owner's shared hours and interleaves them "by their place in their list, then by the path of the node declaring it". Within one list the order is the owner's; across lists it is an accident of numbering and spelling.

`cmc` is the case that breaks it. Each repository node declares its own semver releases (`cmc/cmc-lib`, `cmc/cmc-frontend/commercial`, …) and the owner's hours are shared through the calendar's carrier `cmc`. A release of `commercial` that needs a feature of `cmc-lib` is real: `commercial 0.11.0` cannot ship before `cmc-lib 1.1.0` is published. Today the forecast places them by position, then path: `0.11.0` is second in its list like `1.1.0`, and `cmc/cmc-frontend/commercial` sorts before `cmc/cmc-lib`, so the forecast schedules the application before the library it needs, and nothing in the files can say otherwise. Moving entries within one list cannot fix an order between two lists.

Item-level `depends_on` exists (SPEC §1.3) and the scheduler honours it, but writing it on every task of `commercial 0.11.0` towards every task of `cmc-lib 1.1.0` is unmaintainable, and a task planned later in either release silently escapes the constraint. The dependency is between versions, so it belongs where versions are declared.

What the forges offer, checked before deciding: a GitHub milestone carries no dependency on another (`gh api 'repos/microsoft/vscode/milestones?state=open&per_page=1' --jq '.[0] | keys'` lists `closed_at`, `description`, `due_on`, `state`, `title` and counters, nothing pointing to another milestone), so the order between two repositories' versions lives nowhere a tool reads.

## Decision

> **A `[[release]]` entry MAY name, in `depends_on`, releases of its own list or of any other node's, as `<node>@<release>`. No work of a release starts before the releases it depends on are delivered, and the forecast places it after them. `doctor` errors on a reference that names nothing schedulable and on a cycle.**

1. **Syntax.** `depends_on = ["cmc/cmc-lib@1.1.0"]` on the entry, a list of strings. `<node>` is the path of a node, `<release>` a release of the list that node plans against, whoever declares it, so a reference written through a child that inherits its list names the ancestor's release. `@<release>`, the node left out, names a release of the same list. The text before the first `@` is the node, the rest the release, both trimmed. A reference without `@`, or with nothing after it, makes the `node.toml` invalid: a bare `1.0` is refused rather than guessed, since a typo in a node path would otherwise read as a release name.
2. **Delivered.** A release is delivered when it is closed (ADR-0029), or when all its scheduled work is delivered — its last delivery, validation by others included — and the releases it depends on are delivered in turn, up to a closed one. A release with no work left is delivered as soon as its own dependencies are.
3. **Forecast order.** The pooled order of ADR-0028 is computed as before, then rebuilt by taking, each time, the first release of that order whose dependencies are all placed. Without `depends_on` the order is exactly the old one; with it, a dependent release moves after its dependencies even when it is listed earlier or its node sorts earlier, and nothing else moves. When no release can be taken, the dependencies left loop: the first release left is placed anyway and its unplaced dependencies are ignored, so the forecast stays total and deterministic, and it warns (`dependency_cycle`, on `release <name>`).
4. **Forecast gate.** Release dependencies are constraints like item dependencies: no unit of a release starts before every release it waits for (transitively, up to a closed one) has no work left to place and is delivered. Started work waits too; work handed to someone (waiting, or in testing by others) ignores them as it ignores its own dependencies. Work that waits through its release on work that can never be scheduled stays unscheduled, `cycle` or `blocked_by_cycle`. A dependency on a release the forecast does not schedule — a list outside the carrier's scope, a parking release — is ignored and warned about (`unscheduled_dependency`).
5. **`doctor`.** A reference to a node that does not exist, to a release the named node's list does not declare (or a node no list applies to), or to a parking release is an error, `unknown_release_dependency`; a cycle between releases, a release depending on itself included, is an error, `release_dependency_cycle`, reported once with its path (`a@1 -> b@2 -> a@1`). Both need a human (exit `2`): the fix is choosing which reference is wrong, which no command can know. A malformed reference is the existing `invalid_node_toml` error.
6. **Writes.** `ariane release set <node> <name> --depends-on <node>@<release>` (repeatable) replaces the list and `--no-depends-on` removes it; `ariane release add` takes `--depends-on` too; `PATCH` and `POST /api/releases` take `depends_on`. They refuse what `doctor` would report. `release rename` rewrites the references to the renamed release in every `node.toml`, keeping their node part; `release remove`, and `release set --parking`, are refused on a release another one waits for.
7. **Reading.** `/api/roadmap` and `ariane roadmap --json` give each release its `depends_on`, canonical `<node>@<release>`; the text prints `(after cmc/cmc-lib@1.1.0)` on the release's line. `/api/nodes` and `/api/releases` serve the entries with `depends_on` as written.

## Consequences

- `ariane roadmap cmc` places `commercial 0.11.0` after `cmc-lib 1.1.0`, from one line in `commercial`'s `node.toml`, and keeps doing so when tasks are added to either release.
- Every existing tree forecasts exactly as before: no list carries the key, and the order rebuild is the identity without it.
- A dependency crosses `node.toml` files, which so far only inherited downwards. Renaming or removing a release now reads every `node.toml` of the tree, and a hand edit of one file can break a reference in another; `doctor` catches it, the commands keep it right.
- A dependency is only as strong as the forecast's scope. `ariane roadmap cmc/cmc-frontend/commercial` without a calendar carrier runs on `commercial` alone, cannot see `cmc-lib` and warns that the dependency is ignored; with `cmc` as carrier it holds.
- The gate is coarse: all of a release waits for all of another. A release that needs only part of a library release has to say so with item-level `depends_on` instead.
- `depends_on` serialises as an empty list on every release of `/api/nodes`, a change of shape the interface's types absorb as optional.

## Alternatives considered

### Item-level `depends_on` only

No new key. Rejected: the constraint is between versions, and an item-level edge covers only the tasks that exist when it is written. A task added to `cmc-lib 1.1.0` next week would not hold `commercial 0.11.0` back, and the plan would be silently wrong.

### Order lists against each other by a global position

A `position` or `rank` on releases across nodes, so the owner orders every release of `cmc` in one sequence. Rejected: it couples every list to every other one, so adding a release to one repository renumbers or reorders the others, and it states an order without its reason: nothing says that `commercial 0.11.0` *needs* `cmc-lib 1.1.0`, so nothing warns when the order is changed against it.

### A bare release name for the same list

`depends_on = ["1.0"]` reads naturally. Rejected: `cmc/cmc-lib 1.1.0`, a typo for a cross-node reference, would parse as a release named so in the same list, and the error would be "unknown release" instead of the reference being refused at the syntax. `@1.0` costs one character and is unambiguous.

### Gate on the dependency's span end rather than on delivery

The owner's last hour on the release, ignoring validation by others. Rejected: a library release validated by someone else is not published until that validation ends, and the dependent release cannot use it before then. Delivery is what the scheduler already uses for item dependencies.

## Revisit if

- Dependencies need a lag or a kind (start-to-start, "two weeks after"), which a list of references cannot carry.
- Several people plan the same carrier in parallel, so that "after" no longer means "later in one sequence".
- Partial dependencies — a release needing only some epics of another — become common enough that the coarse gate is wrong more often than right.

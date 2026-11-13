# ADR-0025 — Estimate in hours of focused work, declared per node

- **Status:** Accepted
- **Date:** 2026-11-11
- **Concerns:** SPEC §1.3, §1.5; reflexion `slashome/ariane#R010`; [ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md), [ADR-0026](0026-keep-the-work-calendar-personal.md)

## Context

A forecast turns work into dates, so it needs a unit of work. The header has `size`, `XS` to `XL` (SPEC §1.3), with no quantity behind it. It is also rarely set: none of `pharmakon`'s 211 tasks carry one (`grep -rh '^size:' tree/slashome/pharmakon | wc -l` in `_ariane` prints `0`).

Testing is real work that nobody estimates. In `pharmakon` the owner plays the build to validate a task (`a-tester`, an `in_progress` status). On `cmc` nodes a product manager validates while the task waits (`waiting_on: validation`, [ADR-0015](0015-name-whom-an-item-waits-for.md)): the owner's hours are free, but the release is not delivered.

There is no measured velocity yet. Whatever unit is chosen will be recalibrated once one exists.

## Decision

> **A size stands for hours of focused work. Each node declares the hours of each size, a default size, and a test ratio.**

1. **Declaration.** An `[estimates]` table in `node.toml`:

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

   Hours are positive numbers. Keys accumulate through inheritance like `[fields]`; the nearest declaration of a key wins. A node with no table anywhere above it uses the values above.
2. **Items keep `size`.** The header does not change: an item says `M`, the node says what `M` costs. Recalibrating a node rewrites one table, never its items.
3. **A task without `size` counts as the default size.** The roadmap draws it hatched and counts it among its warnings ([ADR-0028](0028-compute-the-forecast-in-one-place.md)).
4. **An epic or a story without tasks MAY carry a provisional `size`**, priced by the same table. It stands for the whole of the work not yet split, and the roadmap draws it hatched, as a rough estimate. Once it has tasks, its size is the sum of its tasks' and its own `size` is ignored. Without a `size` and without tasks, it counts as the default size, like an unsized task.
5. **Testing is added, never estimated.** A task of `h` hours adds `h × test_ratio` of testing after it. With `tester = "owner"`, the testing consumes the owner's hours like the task. With `tester = "others"`, it is a delay: the task's release cannot be delivered before it elapses, but the owner's hours go to the next task. A ratio of `0` removes it.
6. **The sizes stay the five of SPEC §1.3.** A table gives hours to those five; it cannot add a sixth.

> **Amended 2026-11-13: when a story or an epic counts as split, and what dropped work leaves.** §4 reads "tasks" for both kinds; the implementation of S031 made it exact. A story is split as soon as it has tasks, and weighs the sum of its tasks. An **epic is split as soon as it has stories**, and weighs the sum of its stories, each one weighed by the same rule: a split story by its tasks, an unsplit story by its own provisional `size`, else by the default size. An epic's own `size` is then ignored, whether or not its stories have tasks. Dropped children leave every sum. A story or an epic **whose children are all dropped weighs 0 hours**: it was split, and nothing of it is left to do; it does not fall back to its provisional `size` or to the default size. A sum is marked guessed as soon as one of its parts is a default or provisional size.

## Consequences

- Every unsized task still weighs something, so a forecast exists from the first day, and the warnings say how much of it is a guess.
- An epic planned in a release before it is split ([ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md) §3) weighs its provisional size, so a far release has a date before its work is broken down. The size left on a split epic stays in its header, unread; the interface shows the sum of its stories instead.
- The hours shown are focused hours. Meetings, reviews and interruptions are taken out once, by the calendar's focus coefficient ([ADR-0026](0026-keep-the-work-calendar-personal.md)), not padded into each estimate.
- One table per node means two nodes may price `M` differently, which is correct when their work differs and confusing when it does not. The interface shows the hours next to the size wherever a size is shown.
- `tester` is a node-wide setting. A node where some tasks are validated by the owner and some by someone else gets one rule for all.
- Velocity (later) will rewrite the table and the focus coefficient from measured durations, using `closed` dates ([ADR-0027](0027-record-when-an-item-closes.md)). Nothing in the items has to change for it.

## Alternatives considered

### Hours in each item (`estimate: 6h`)

Precise, per task. Rejected: the precision is false before any measurement, and every recalibration would rewrite every item. A size is a judgment the owner can make in a second; an hour count is not.

### Story points

The usual agile unit. Rejected: points become dates only through a velocity, which does not exist yet. Until then a forecast in points is no forecast.

### Ideal days

Closer to how people talk. Rejected: a day means nine hours on `cmc` and three evening hours on `pharmakon`, so the same "two days" would be a different amount of work on each node. Hours mean the same everywhere; the calendar says when they happen.

### Sizes on tasks only

One estimate per piece of work, never two that overlap. Rejected: an epic not yet split would weigh only the default size of one task, so a release planned with whole epics would look far shorter than it is. A provisional size that yields to the tasks as soon as they exist keeps a single estimate at any time.

### Leave unsized tasks out of the forecast

Only count what is estimated. Rejected: the forecast would be silently optimistic, and on `pharmakon` empty, since no task carries a size.

## Revisit if

- Measured durations show that sizes do not separate work at all (an `S` takes as long as an `L`). Then the unit, not the table, is wrong.
- A node needs per-task testers, which would make `tester` a property of the task or of its workflow status rather than of the node.

# ADR-0015 — Name whom an item waits for

- **Status:** Accepted
- **Date:** 2026-10-23
- **Concerns:** SPEC §1.3, §1.5, §1.6; [ADR-0008](0008-give-every-item-a-versioned-front-matter.md), [ADR-0011](0011-sync-external-trackers-through-connectors.md), [ADR-0014](0014-record-branches-and-pull-requests-through-the-agent-harness.md)

## Context

Much of the owner's work stops on someone else: a review, an answer, a decision. The six categories had one place for it, `blocked` — "cannot move until something else happens" — which says neither who nor since when. A board of blocked items is a list of excuses; a board of items waiting for Keyvan for four days is a list of people to chase.

The `cmc` nodes are bound to the team's monday board, which has no "waiting" status — the closest are "⏳ Code review waiting" and "Bloqué" — and no column for the person awaited. In monday a person is a numeric user id; in a header that id is unreadable, and only means something in one tracker.

## Decision

> **Waiting is a category of its own, and an item in it names the people it waits for by handles of a directory its node declares.**

- **A seventh category, `waiting`,** between `in_progress` and `blocked`. The default workflow gains a `waiting` status; a node with its own workflow adds one if it wants the column.
- **Three core keys,** meaningful only while the item waits: `waiting_for`, a list of handles (several people may be awaited); `waiting_on`, why — `review`, `validation`, `answer` or `other`; `waiting_since`, a date. `doctor` requires `waiting_for` in the category, checks every handle, and warns about keys left after the wait. The interface asks for them when a card enters the waiting column and removes them when it leaves.
- **A directory, `[people.<handle>]` in `node.toml`,** inherited like fields: a `name`, and one key per tracker with the person's id there (`monday = 58294966`). The header stays readable and tracker-neutral (ADR-0011); a connector resolves a handle to its own id.
- **monday, by its reason.** `[tracker.waiting]` maps `waiting_on` to a status label: a `review` goes to "⏳ Code review waiting", anything else to "Bloqué"; the people awaited are @mentioned in an update of the ticket. For now the agent does it, by the skill, through the monday connector of its harness; `ariane sync monday` (ADR-0011) takes it over next, so that the interface moves the ticket too.
- **Validation is its own reason** (added the same day): a ticket in monday's "To validate" waits for the product manager to check it in preprod, which is neither a code review nor an answer; it maps both ways to that label.
- **Leaving the wait is the owner's choice.** The previous status is not remembered: the agent asks where the item goes; in the interface, the column it is dropped in decides.

## Consequences

- Boards gain a waiting column everywhere, the "All nodes" Kanban included, and cards say who and for how long.
- A seventh category touches every view that lists categories, and every workflow that wants the column must declare a status for it.
- Two waiting reasons share one monday status ("Bloqué"), and "Bloqué" also maps back to `blocked`: a ticket that comes back from monday as "Bloqué" does not say whether it waited or was blocked. The header keeps the truth; monday keeps the closest label.
- Until the connector exists, monday follows only what goes through the agent; a card moved in the interface does not reach monday.

## Alternatives considered

### A status of the `blocked` category

No schema change. But the "All nodes" Kanban would merge waiting into blocked, and nothing would require saying for whom — the very information that makes the column useful.

### Free names, or tracker ids, in `waiting_for`

Free names drift with every spelling and link to nothing; `monday:<id>` is unreadable and means nothing in a node bound to another tracker, or to none. A handle declared once is both readable and resolvable.

### A new "En attente" status on the monday board

The faithful mapping, but the board belongs to the whole team; changing its statuses is the team's call, not one member's tool's.

### Remembering the status left, to return to it

Automatic, but the status before the wait is rarely the right one after it: a review done sends the work forward, not back to "in progress".

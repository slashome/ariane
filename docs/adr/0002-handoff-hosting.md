# ADR-0002 — `HANDOFF.md` is hosted centrally, not in the host repository

- **Status:** Accepted
- **Date:** 2026-08-15
- **Concerns:** PLAN §4, §5, §11

## Context

The tree records what a node **is**. Nothing records what is **in flight** — the half-finished change, the question waiting on an answer, the approach abandoned an hour ago and the reason why. That knowledge exists only in the session that produced it and dies with it.

The first draft of §11 answered this with a file committed **at the root of the host repository**, justified as a deliberate exception to the zero-footprint rule (§4, §5): an exchange surface readable by a collaborator, a fresh agent session, or the user on another machine before `ariane init` — anyone arriving with nothing but a clone.

## Decision

`HANDOFF.md` is hosted like every other Ariane artifact: **in the central content repository at its node**, materialized through `_ariane/`, and never committed into the host repository. §4 and §5 apply to it unchanged, and **the zero-footprint rule has no exception at all**.

Three properties come with it:

1. **Its resting state is a receipt.** Three states must stay distinguishable: absent (this node never handed anything over), a receipt line only (nothing in flight), content (work is in flight). A consumed handoff is emptied down to *who picked it up, when, and at which commit*. The commit is the load-bearing part — "picked up on 14 August" says nothing about whether work happened since, while a commit lets `doctor` state that twenty-three commits have landed since the last handoff was consumed.
2. **It stays a separate file from `INDEX.md`.**
3. **Its structure is taken from clinical shift handoff** (SBAR): situation, background *with pointers rather than copies*, assessment — what is blocked, on whose decision, and what is verified versus asserted — recommendation.

Keeping it current is a line of the Ariane agent's mandate (`AGENT.md`), not a separate skill.

## Consequences

- The method has no exception to zero-footprint. Anyone reading §4 can rely on it without qualification.
- A collaborator who does not use Ariane gets nothing. Accepted for v1; if multi-user work later requires an exchange surface, it is a new decision superseding this one, not a reopening of §4.
- `doctor` gains a staleness check: a handoff whose receipt names a commit older than the node's current head means work has landed since the last one was picked up.
- The receipt line's format (machine, date, commit) becomes part of the specification, since `doctor` parses it.
- The handoff template of roadmap step 4 has a proven structure to borrow from rather than one to invent.

## Alternatives considered

### Committed at the root of the host repository (the original §11)

The exception was argued from one sentence: *for a reader arriving cold with nothing but a clone*. It named three readers — a collaborator, a fresh agent session, the user on another machine.

Rejected because the cost is a permanent exception to the method's most valuable structural property, bought for a reader who is out of scope: **multi-user is post-v1**. Of the three readers, two are the user themself, and both are one command away from having the central repository. Dropping the exception makes zero-footprint absolute, which is a stronger claim to defend than "one exception, and only one" — a formulation that invites the second.

The evidence in favour was real and is recorded here rather than lost: a fresh agent session cloned this repository cold and had to reconstruct the state of the work from `gh pr list` and three branch diffs. It establishes that the *problem* exists. It does not establish that the *file must live in the host repository*, since handing the agent the central repository path solves it in one gesture.

### Read it, then empty it and commit a `recovered by <machine> on <date>` marker in the host repository

The mechanism first proposed alongside the original §11. It does not survive more than one contributor: it is a hand-rolled mutex over a git branch, producing conflicts on a file that carries no value and history entries that describe no work. Dropped before the hosting question was even settled.

### A section of `INDEX.md`

The obvious simplification once the file moved into the central repository: same location, same reader, why two files?

Rejected because they are not two contents but **two incompatible writing disciplines**. `INDEX.md` is edited, accumulates, and is curated; its lifetime is the node's; it is never emptied. `HANDOFF.md` is replaced wholesale, and being emptied is its resting state. Putting a *replace-this-whole-block-every-time* region inside a *preserve-and-enrich* file, and handing it to an agent, is how durable memory eventually leaves with the replacement. Two files means two writing regimes, and a truncation can then never reach the memory. This is a safety argument, not a taxonomic one.

### A dedicated skill describing the protocol

Rejected because §7 ships exactly one agent and puts skills under project ownership. A method-level skill would be a third kind of shipped artifact, whose authority relative to `AGENT.md` would have to be defined — the same defect already identified in the Claude adapter. The protocol is a mandate line instead.

### Keeping the name `RESUME.md`

At the root of an English repository it reads as a CV before it reads as "resume the work". `HANDOFF` was already the section's own word — *the baton handed over at the boundary* — and it names an existing protocol family (relay handoff, telecom handoff, clinical shift handoff) rather than inventing one. Renaming cost nothing before the template of roadmap step 4 exists.

## Revisit if

- Multi-user projects land and require an exchange surface with people who do not use Ariane.
- The receipt turns out to be unusable in practice — e.g. the commit is meaningless for a node that is not a git repository, which is the case at the home level.

# ADR-0007 — Write `HANDOFF.md` on explicit request, never on session end

- **Status:** Accepted
- **Date:** 2026-08-20
- **Concerns:** PLAN §11 (*Whose job it is*), §7, roadmap step 5
- **Supersedes:** nothing. [ADR-0002](0002-handoff-hosting.md) settles *where* the handoff
  lives and stays in force unchanged; this record settles only *when it is written*.

## Context

§11 assigned three movements to the Ariane agent — quoted here as it stood before this
decision, which amends it:

> Reading the handoff when a session opens, consuming it down to its receipt, and **writing it before a session ends** belong to the mandate of the Ariane agent (§7)

The first two are sound. The third names a trigger **the agent cannot observe**.

Two facts have to hold for "before a session ends" to be actionable, and neither does:

1. **That the session is ending.** An agent does not see a session end. It sees a turn stop. Nothing distinguishes a five-minute pause from a close, and there is no signal — no platform among the adapters of §8 exposes one — that fires before the last turn rather than after it.
2. **That what follows justifies a handoff.** §11 exists for a specific discontinuity: *"the user on another machine, a fresh agent session, the same user in three weeks"*. Whether the next session happens on the same machine an hour later or on a different one tomorrow is **information only the user holds**. The agent has no access to it, and cannot derive it — a laptop closing looks exactly like a coffee break.

So the trigger, as written, degrades in practice to *write by default*, and that breaks two things §11 itself requires.

**It contradicts the resting state.** §11 states that the file's normal state is empty and that *"a receipt line only"* means nothing is in flight. But its condition for writing — *"whenever something is left unfinished, blocked, or waiting on a decision"* — is the **permanent** condition of a living project. Something is always unfinished. A file asked to be empty at rest, under a condition that is always true, is never at rest.

**It contradicts rule 2.** §11: *"It never becomes a second source of truth. Decisions, stories and conventions live in `_ariane/` and are pointed at, never copied."* Writing on every session end means writing whenever the session produced something worth recording — which is exactly when that something belongs in an item file. The handoff then duplicates the durable artifact, the two drift, and the reader can no longer tell which one is true. §11's own rule 3 names the cost: *"a stale handoff is worse than none: it is trusted, and it lies."*

## Decision

**Ariane writes `HANDOFF.md` only when the user asks for it. Never spontaneously, and never as a side effect of other work.**

The other two movements are unchanged: she still reads it first when a session opens, acts on what is in flight, and empties it down to its receipt once picked up. Reading has an observable trigger; writing does not.

Concretely, this forecloses three behaviours: writing it because a session appears to be ending, refreshing it to "keep it current", and updating it because the session happened to produce something worth recording. Durable knowledge goes into the node's own items — `reflexions/`, `backlog/`, `tasks/`, `stories/`, `INDEX.md`. The handoff carries only what is **in flight at the moment of departure**, and the user is the only party who knows when that moment is.

## Consequences

- The user must remember to ask. This is the real cost, and it is not small: forget, switch machines, and there is no handoff. It is accepted because the alternative failure is worse — a file written on every session end is trusted and stale, where an absent file is merely absent. §11 already ranks these two: *"a stale handoff is worse than none"*.
- The file's resting state becomes reachable. A receipt means what §11 says it means, because nothing else writes into it.
- **`doctor` loses a check it never had a basis for.** §11 proposes that a receipt lets `doctor` report *"twenty-three commits have landed since the last handoff was consumed, and the node has been running blind ever since"*. Under a request-only trigger that is no longer an anomaly: commits landing with no handoff is the normal case, since most sessions do not end in a departure. The receipt keeps its forensic value — *who picked up what, at which commit* — but the derived warning has to go, or it fires constantly and gets ignored. ADR-0004's exit-code contract is unaffected; nothing here is a `Remedy`.
- Rule 3 of §11 still holds where it applies: a handoff written on request is written **in the same commit as the work it describes**.
- The agent's mandate loses a trigger and gains nothing automatic in its place. That is the point: the method stops asserting a capability its runtime does not have.

## Alternatives considered

### Keep the session-end trigger as §11 stated it

Rejected because the trigger is not observable, which is not a matter of degree. An agent instructed to act "before a session ends" either writes on every turn that might be the last — that is *write by default*, with the two contradictions above — or it never fires, because the moment it was told to watch for never announces itself. There is no third behaviour, and the first is what happens in practice.

### Infer the departure — heuristics on the state of the work

Write it when the working tree is dirty, when N files were touched, when a question was left unanswered. Rejected on §11's own terms: rule 4 requires the handoff to separate *what was verified* from *what was merely asserted*, and a heuristic trigger asserts the one thing that matters most — that a departure is happening. It would produce confident handoffs about sessions that simply paused. ADR-0003 already refuses this class of design in one line: *what must hold invariably is the CLI's job to verify*, not the agent's to guess.

### Make it configurable — `always` / `never` / `ask`

Rejected because it puts in a config file a decision that belongs to a moment. The value would be set once, in the abstract, and then be wrong for whichever session it mattered in: `always` reproduces the failure above, `never` removes the feature, and `ask` is this decision with an extra key to maintain. It also adds a second place where behaviour is specified, next to `AGENT.md` — with no rule saying which wins.

### Split the file — an automatic log plus a hand-written handoff

Rejected: it is the *"second source of truth"* rule 2 forbids, with the drift built in by construction. §11 has already arbitrated the neighbouring version of this question — why `HANDOFF.md` is not a section of `INDEX.md` — on the ground that two writing disciplines must not share a file. Two files with the same purpose and two triggers fails the same test.

## Revisit if

- A platform among the adapters (§8) exposes a **reliable session-end signal** that fires before the last turn. The objection here is capability, not principle: given the signal, the trigger becomes observable and this decision should be re-argued.
- Forgetting to ask causes a **measured** loss — a resumption that cost real time because nothing was handed over. That is the failure mode this decision accepts, and if it turns out to dominate, the arbitration was wrong. Note what would count as evidence: a session that had to reconstruct state from the code, not an occasion where a handoff would merely have been nice.
- The receipt line proves useless once nothing writes the file automatically. §11 justifies the receipt partly by a `doctor` warning this ADR removes; if what remains does not earn a conventional file, the receipt itself is worth revisiting.

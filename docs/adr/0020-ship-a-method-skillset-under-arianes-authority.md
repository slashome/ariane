# ADR-0020 — Ship a method skillset under Ariane's authority

- **Status:** Proposed
- **Date:** 2026-10-28
- **Concerns:** PLAN §7, §8; [ADR-0002](0002-handoff-hosting.md); [ADR-0006](0006-skill-pack-placement.md); [ADR-0018](0018-name-the-personas-a-story-is-written-for.md); [ADR-0019](0019-group-stories-into-epics.md)

## Context

Since [ADR-0019](0019-group-stories-into-epics.md) the method has four kinds of work item: reflexion, task, story, epic. Choosing between them is the first act of managing any request — *can this be fixed in one ticket, or does it need a reflexion, a story and its tickets, an epic and its stories?* — and nothing in the method helps with it. Once the kind is chosen, three more procedures follow: writing a story for a persona ([ADR-0018](0018-name-the-personas-a-story-is-written-for.md)), splitting a story into tasks, framing an epic into stories.

PLAN §7 says the method ships exactly one agent and that skills belong to projects. [ADR-0002](0002-handoff-hosting.md) rejected a method-level skill for the handoff because it would be *"a third kind of shipped artifact, whose authority relative to `AGENT.md` would have to be defined"*. That objection stands, and this ADR answers it instead of setting it aside.

What differs from the handoff case: the handoff protocol is a few lines read at every session; these four procedures are long, step by step, interactive, and needed only when work is being framed. Folded into `AGENT.md` (81 lines today), they would make it a manual loaded into every session to serve a minority of them.

## Decision

> **The method ships a small set of skills — procedures Ariane runs — next to its single agent. `AGENT.md` stays the only authority: a skill defines *how* to carry out a mandate line, never *what* the mandate is, and where a skill disagrees with `AGENT.md`, the skill is the bug.**

- **Still one agent.** The skills have no persona of their own: each is Ariane carrying out a procedure. PLAN §7's "exactly one agent" holds; its "skills are owned per project" gains one exception, the method's own skillset.
- **Mandate first.** Each skill is named by a mandate line of `AGENT.md` ("frame a request before producing it — see `size`"). A skill no mandate line names is not shipped.
- **Neutral, then adapted** (PLAN §8). Each procedure lives in `agents/ariane/skills/<name>/PROCEDURE.md`, platform-neutral; `adapters/claude/skills/ariane-<name>/SKILL.md` adapts it, the same way the `ariane` adapter does `AGENT.md`.
- **Prefixed names.** Every method skill is named `ariane-<name>`, so it cannot collide with a team's skill (the failure [ADR-0006](0006-skill-pack-placement.md) describes); `doctor` reports a same-named skill anyway.
- **Installed like the agent's adapter**, by symlink at user level, never in a host repository ([ADR-0006](0006-skill-pack-placement.md)).
- **The first set:** `ariane-size` (choose reflexion / task / story + tasks / epic + stories + tasks), `ariane-write-story`, `ariane-split-tasks`, `ariane-frame-epic`.

## Consequences

- `AGENT.md` stays short and authoritative; the long procedures load only when they are needed.
- Supersedes [ADR-0002](0002-handoff-hosting.md)'s rejection of method-level skills, not its decision: the handoff stays a mandate line, because it is short and read at every session.
- One more kind of shipped artifact to keep consistent: a change to the SPEC or to `AGENT.md` must now also check the procedures, in the same commit. Nothing verifies this automatically, and drift is the expected failure.
- Four more entries in the skill listing, which spends the shared context budget [ADR-0006](0006-skill-pack-placement.md) warns about.
- The boundary between a mandate line and a procedure is a judgment call, and it will be made again for every new skill.

## Alternatives considered

### Fold the procedures into `AGENT.md`

Keeps one artifact and ADR-0002 untouched. It fails on size and on load: four interactive procedures would roughly triple `AGENT.md`, and the whole of it is loaded into every session, most of which do not frame any work.

### A second agent, a "planner"

Gives the procedures a home of their own. It breaks PLAN §7 for real, and splits management between two personas who would both claim "where does this belong?" — the very question Ariane exists to answer.

### Leave them to each project

Consistent with PLAN §7 as written. But choosing the kind of item depends only on the method's kinds, not on any project; every project would rewrite the same procedure, and the copies would drift from the SPEC.

### Namespace through a plugin (`ariane:size`)

Makes collisions impossible by construction. Rejected in favour of the `ariane-` prefix, for the reason ADR-0006 gives: plugins handle symlinks differently, and the method skills would no longer install the way the agent's adapter does. Worth revisiting with the plugin question it left open.

## Revisit if

- A procedure turns out to be short enough to fit in a mandate line — fold it back.
- The method needs a procedure no mandate line can name — the authority rule is then wrong.
- The skill listing budget starts dropping skills from sessions.

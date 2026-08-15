# Architecture Decision Records (ADR)

A log of Ariane's design decisions. One file per decision: what prompted it, what was chosen, what it costs, and what was ruled out.

`PLAN.md` says where the method is going, and `SPEC.md` will say what it requires. Neither says **why**, nor what was considered and rejected — the part which, once lost, gets re-litigated in good faith a year later by someone with no way of knowing the question was already settled.

Same format and same lifecycle as the sibling project [dotflies](https://github.com/slashome/dotflies), deliberately: a method about conventions should not invent a second one for itself.

## Lifecycle

An ADR at the root of this folder is in force. When a decision is revisited we do not edit the old file and we do not delete it: we write a new ADR, flip the old one to `Superseded` with a link to its successor, and move it into [`archives/`](archives/). Obsolete reasoning stays readable — it is usually what explains a constraint that has since become mysterious.

| Status | Meaning |
| --- | --- |
| `Proposed` | Written, not yet settled. |
| `Accepted` | In force. |
| `Superseded` | Replaced by a later ADR. Archived. |
| `Deprecated` | No longer relevant, no successor. Archived. |

## Writing convention

[Michael Nygard](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions) format: **Status**, **Context**, **Decision**, **Consequences**, **Alternatives considered**.

Files are named `NNNN-imperative-title.md`, four-digit sequential numbering, never reused after archiving.

Three rules Ariane adds, because they are what make the record worth keeping:

- The *Consequences* section carries the **negative** effects as much as the positive ones. An ADR that costs nothing is usually an ADR that settled nothing.
- *Alternatives considered* carries the **reason** each one was rejected, and the reason has to be a failure case rather than a preference — the collision that exists, the precedent that shows the option does not survive contact, the command whose output contradicts the assumption. Without it, anyone can re-propose the rejected option and nothing in the repository will stop them.
- **Facts carry their proof.** A claim about a tool's behaviour carries the command that establishes it, so the next reader re-runs rather than believes. Same rule as `HANDOFF.md`'s (PLAN §11), for the same reason.

A closing *Revisit if* section names the conditions under which the decision would become wrong. If none can be named, the decision was not a real arbitration.

## Decisions in force

| № | Decision | Status | Date |
| --- | --- | --- | --- |
| [0001](0001-central-repository-layout.md) | Mirror the logical tree in the central repository, under a reserved prefix | Accepted | 2026-08-15 |
| [0002](0002-handoff-hosting.md) | Host `HANDOFF.md` centrally, not in the host repository | Accepted | 2026-08-15 |
| [0003](0003-no-git-hooks.md) | Use no git hooks, and enumerate worktrees on demand | Accepted | 2026-08-15 |
| [0004](0004-write-or-judge.md) | Separate the commands that write from the command that judges | Proposed | 2026-08-15 |

## Adding one

Take the next number (archives included), write the five sections, add the row above. If the decision replaces another, archive the old one and drop it from the table. [`0000-template.md`](0000-template.md) is the starting point.

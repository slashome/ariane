# Architecture Decision Records

`PLAN.md` says where the method is going. `SPEC.md` will say what it requires. Neither says **why**, nor what was considered and rejected — and that is the part which, once lost, gets re-litigated in good faith a year later by someone who has no way of knowing the question was already settled.

An ADR is one file per decision: the context that forced it, the decision itself, the alternatives that were rejected **and why**, and the consequences accepted along with it.

## Rules

- **One decision per file**, numbered, named `NNNN-kebab-case-title.md`.
- **Never edited once accepted.** A decision that no longer holds is *superseded*: a new ADR is written, and the old one's status becomes `Superseded by ADR-NNNN`. The record of having been wrong is part of the record.
- **Rejected alternatives are mandatory**, with the reason. An ADR without them is a summary, not a decision record — anyone can re-propose the rejected option and nothing in the repository will stop them.
- **Facts carry their proof.** A claim about a tool's behaviour carries the command that establishes it, so the next reader re-runs rather than believes. This is the same rule as `HANDOFF.md`'s (PLAN §11), for the same reason.
- **Status** is one of `Proposed`, `Accepted`, `Superseded by ADR-NNNN`, `Rejected`.

## Relation to the rest

| | Answers |
|---|---|
| `PLAN.md` | where we are going |
| `SPEC.md` | what the method requires |
| `docs/adr/` | why it requires that, and what was rejected |
| `HANDOFF.md` (§11) | what is in flight right now |

An ADR is written **when the decision is made**, not when the code lands — the reasoning is at its most complete the day the arbitration happens, and it decays from there.

## Index

| # | Title | Status |
|---|---|---|
| [0001](0001-central-repository-layout.md) | Central repository layout: mirror tree under a reserved prefix | Accepted |
| [0002](0002-handoff-hosting.md) | `HANDOFF.md` is hosted centrally, not in the host repository | Accepted |
| [0003](0003-no-git-hooks.md) | No git hooks; worktrees are enumerated on demand | Accepted |
| [0004](0004-write-or-judge.md) | A command writes, or a command judges — never both | Proposed |

New ADR: copy [`0000-template.md`](0000-template.md).

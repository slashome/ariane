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
| [0004](0004-write-or-judge.md) | Separate the commands that write from the command that judges | Accepted | 2026-08-15 |
| [0005](0005-write-ariane-in-rust.md) | Write the `ariane` CLI in Rust | Accepted | 2026-08-15 |
| [0006](0006-skill-pack-placement.md) | Declare skill packs per node, install them once at user level | Accepted | 2026-08-15 |
| [0007](0007-write-the-handoff-on-request.md) | Write `HANDOFF.md` on explicit request, never on session end | Accepted | 2026-08-20 |
| [0008](0008-give-every-item-a-versioned-front-matter.md) | Give every item a versioned front-matter, extended per node | Accepted | 2026-10-20 |
| [0009](0009-migrate-content-with-versioned-migrations.md) | Migrate content and configuration with versioned migrations | Accepted | 2026-10-20 |
| [0011](0011-sync-external-trackers-through-connectors.md) | Sync external trackers through per-node connectors | Accepted | 2026-10-20 |
| [0012](0012-serve-the-interface-from-the-binary.md) | Serve the interface from the `ariane` binary | Accepted | 2026-10-20 |
| [0013](0013-define-the-configuration-schema-in-code.md) | Define the configuration schema in code | Accepted | 2026-10-20 |
| [0014](0014-record-branches-and-pull-requests-through-the-agent-harness.md) | Record branches and pull requests through the agent's harness | Accepted | 2026-10-23 |
| [0015](0015-name-whom-an-item-waits-for.md) | Name whom an item waits for | Accepted | 2026-10-23 |
| [0016](0016-name-local-servers-through-a-front-door.md) | Name local servers through a front door in `ariane` | Accepted | 2026-10-23 |
| [0017](0017-keep-a-nodes-judgment-in-its-items.md) | Keep a node's judgment in its items, drop `INDEX.md` | Accepted | 2026-10-23 |
| [0018](0018-name-the-personas-a-story-is-written-for.md) | Name the personas a story is written for | Accepted | 2026-10-23 |
| [0019](0019-group-stories-into-epics.md) | Group stories into epics | Accepted | 2026-10-27 |

## Archived

| № | Decision | Status | Date |
| --- | --- | --- | --- |
| [0010](archives/0010-keep-only-judgment-in-index-files.md) | Keep only judgment in `INDEX.md`, compute aggregates on demand | Superseded by [0017](0017-keep-a-nodes-judgment-in-its-items.md) | 2026-10-20 |

## Adding one

Take the next number (archives included), write the five sections, add the row above. If the decision replaces another, archive the old one and drop it from the table. [`0000-template.md`](0000-template.md) is the starting point.

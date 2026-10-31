# ADR-0021 — Share content through team content repositories, never through the host repository

- **Status:** Proposed
- **Date:** 2026-10-31
- **Concerns:** PLAN §3, §4, §5; epic `slashome/ariane#E001`

## Context

Ariane was designed for one person: PLAN §3 gives each user **one single private repository** of content. That repository now holds work that is not only the user's. The `cmc` subtree — the employer's projects — lives in `slashome/_ariane`, which [ADR-0011](0011-sync-external-trackers-through-connectors.md) already records as a debt, and colleagues who should see and edit those items cannot. Epic E001 sets the goal: a colleague clones a shared content repository on its own and sees the same `cmc` items as the user, while the user keeps a single tree across personal and shared content.

Before anything gets mounted, one question has to be settled: **what is the unit of sharing?** One candidate keeps coming back because it looks natural: move each node's items into the repository they describe, committed next to the code (`cmc-frontend/.ariane/`). This ADR settles that question. *How* a user's tree composes several repositories (local configuration, git submodules, a versioned registry), cross-repository inheritance and references, and the split between personal and shared state are left to the rest of E001.

[ADR-0002](0002-handoff-hosting.md) anticipated this moment. When it made zero-footprint absolute, it noted: *"if multi-user work later requires an exchange surface, it is a new decision superseding this one, not a reopening of §4."* Multi-user work is that case.

Facts the decision rests on, from the content repository on 2026-10-31:

- **45 of 764 items live on nodes that have no repository**: home (14), `cmc` (17), `atlas-project` (11), `dreamcatchr-app`, `llm`, `slashome` (1 each). `ariane tasks --all --json`, counted by `node`.
- **The `cmc` subtree is 170 items spread over seven host repositories plus the project node**. The project node `cmc` carries what every repository below it inherits: `[personas]`, `[tracker]`, `[tracker.status]`, `[tracker.push]` (`grep -n '^\[' tree/cmc/node.toml`).
- The host repositories of `cmc` are team repositories. Their default branch is protected and takes changes through reviewed pull requests.

## Decision

1. **Zero-footprint holds for shared content too.** An item, a `node.toml` or a `HANDOFF.md` is never committed into a host repository, whoever it is shared with. PLAN §4 keeps no exception.
2. **The unit of sharing is a content repository per audience**, not per host repository: one repository holds the subtree a group of people works on. For CMC that is `choosemycompany/cmc-ariane`, holding `cmc` and every node below it, with the same `tree/` layout as [ADR-0001](0001-central-repository-layout.md). A personal content repository (`<identity>/_ariane`) remains each user's home.
3. **A user's tree is composed of mounted content repositories.** Each one is grafted at a point of the logical tree (`cmc` → `choosemycompany/cmc-ariane`). Commands, the interface and the hook keep seeing one tree. PLAN §3 changes from "one single private repository" to "one personal repository, plus the shared repositories mounted into it".
4. **A host repository finds its node through its git remote** (the `repository` key of story `S017`), not through files placed in it. That remote is the only link between code and content, and it requires nothing to be committed on the code side.

## Consequences

- Project-level and home-level nodes keep a place to live, and so does their judgment. The roadmaps, cross-repository epics, people, personas, tracker and workflows that every repository below them inherits stay where they are.
- An item has one state, whatever branch the code is on. Changing a status is a commit to a content repository, not a pull request through a code review.
- Teammates who do not use Ariane see nothing in their repositories. That is the zero-footprint promise. It is also a limit: the exchange surface for them is the tracker connector ([ADR-0011](0011-sync-external-trackers-through-connectors.md)), not files.
- Git writes now target several repositories. Commit, pull, push and uncommitted state have to be handled per mounted repository, in the CLI and in `ariane serve`. A shared repository that is behind or has diverged must not block the personal one.
- Everything the user's single repository used to hold by default now needs an owner. Focus, `HANDOFF.md`, `settings.toml` (theme, pins) and the monday sync state are personal today and live in the content. Mounting a shared repository forces a decision on each one (the fifth story planned in E001, and `T079` of E007).
- Items and code no longer travel together. A pull request cannot carry its task's status change. Accepted: `branches` and `pull_requests` on the item ([ADR-0014](0014-record-branches-and-pull-requests-through-the-agent-harness.md)) already link the two, in the direction that does not touch the code.
- Moving `cmc` out of `slashome/_ariane` is a cross-repository move: files, relative links, references `cmc#T…` from personal nodes, and the question of git history (the third story planned in E001).

## Alternatives considered

### Items committed inside each host repository (`<host>/.ariane/`)

It is attractive: content is cloned with the code, readable by every collaborator without any tool, and a pull request could carry both the change and its task's status.

It fails on four counts:

- **Repository-less nodes have no home.** 45 items live on home and on project nodes, and `cmc` alone carries the tracker, the personas and the workflows that its seven repositories inherit. To keep them, you would need a further repository per project level, which is a content repository by another name. Without one, `node.toml` inheritance breaks.
- **State depends on the branch.** A task marked `done` on a feature branch is `todo` on `dev` until the merge. The interface, `doctor` and the monday sync would each need a rule for which branch is the truth. On the protected `cmc` repositories, a status change would wait for a code review.
- **The tree depends on what is cloned.** `ariane tasks --all`, search and the dashboard would scan every clone on the machine. On a machine without `cmc-ui`, its 19 items vanish without a warning, which breaks the "nothing ever lost" property of PLAN §3.
- **Personal judgment leaks into team history.** The reasons behind a priority, reflexions and handoffs ([ADR-0017](0017-keep-a-nodes-judgment-in-its-items.md) puts the node's judgment in its items) would become permanent history in the employer's repositories.

### Opt-in embedding, node by node, for solo or public repositories

This is a softer version of the above: the shared-repository model by default, plus the ability to mount `<host>/.ariane/` on a node where the user *wants* items to travel with the code, such as a public tool or a solo project.

It is rejected because it brings back an exception to zero-footprint. [ADR-0002](0002-handoff-hosting.md) refused exactly that, on the grounds that "one exception, and only one" invites the second. It would also give `doctor`, `serve` and the hook two hosting regimes to reason about: a node whose state is versioned with the code and a node whose state is not. That cost is real for every node, and the gain exists only on a handful of them.

### Keep everything in the personal repository and grant colleagues access

This is the status quo, plus collaborators on `slashome/_ariane`. It is rejected because access to a git repository cannot be limited to a subtree. Colleagues would read the user's personal projects, and the employer's content would stay in a repository the employer does not own, which is the debt ADR-0011 records.

## Revisit if

- A team adopts Ariane with no project-level nodes at all: every node maps to exactly one repository, and nothing is inherited. The first count against in-repository hosting then disappears.
- Git forges offer per-path read access within a repository. The third alternative then becomes viable for small teams.
- The collaborators who need the items are mostly people who will not install Ariane, so the tracker connector stops being enough as their exchange surface. Then files next to the code are the only surface they read.

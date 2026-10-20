# ADR-0009 — Migrate content with versioned migrations shipped in the CLI

- **Status:** Proposed
- **Date:** 2026-10-20
- **Concerns:** PLAN §6, §9, §10, roadmap steps 3–4
- **Depends on:** [ADR-0008](0008-give-every-item-a-versioned-front-matter.md) (the item header and its `ariane` version key), [ADR-0004](0004-write-or-judge.md) (a command writes or judges)

## Context

ADR-0008 gives every item a machine-readable header. It does not say how the existing content reaches it, nor how it follows the next change to that header — and there will be one: the header is the part of the method most exposed to use, and the first version is being written before a single tool reads it.

Two facts about the content as it stands make the question concrete.

**The starting point is prose, not a format.** An inventory of the reference content repository (`_ariane`, ~120 items outside `pharmakon`) found six header shapes: bullet lists (~51 files), Monday story bullets (~8), a `> Créée : … — statut : …` blockquote in five variants (~56), a YAML front-matter carrying only `node:` (10), and none at all (2). Status is free text — "ouvert", "⏳ ouverte", "🟡 ouverte", "à faire", "à cadrer" for the same state; "✅ LIVRÉE", "FAIT", "clos", "archivé", "Migré" for another — and the same emoji means opposite things in different files (🔴 is *high priority* in one, *in progress* in `3035690590-migrer-stores…md`).

**Hand edits already produce contradictions.** Seven archived items carry two stacked banners that disagree — a `✅ TERMINÉ ET ARCHIVÉ` written over an older `statut : à faire` that nobody removed (e.g. `tree/cmc/cmc-frontend/workspace/archives/tasks/steiger-lot6-shared-lib-grouping.md`). That is what a format change looks like when it is carried out by whoever happens to touch the file.

Against that, one node is already structured: `pharmakon`'s 210 items (merged in `_ariane` PR #10) all carry a YAML front-matter — with **French keys** (`titre`, `statut`, `jalon`, `depend`). It is a real, mechanical migration waiting to happen, and the first test of whatever is decided here.

## Decision

**Content evolves through `ariane migrate`: ordered, idempotent migrations `N → N+1`, shipped in the binary.**

1. **The version is per file.** Each item carries `ariane: N` (ADR-0008). A repository may therefore hold files at different versions, and that mixed state is **legal and visible** — it is how a migration proceeds node by node instead of all at once.
2. **Migrations are code, in the CLI, in order.** Each one transforms exactly one version into the next, is idempotent (running it on an already-migrated file changes nothing), and is tested against fixtures — a before file, an after file, and a byte-for-byte comparison. `migrate` chains them: a file at `1` reaching a binary at `3` goes `1 → 2 → 3`.
3. **Dry-run is the default.** `ariane migrate [path…]` prints the diff it would apply and writes nothing. `--apply` writes the **working tree** and stops there: **it never commits.** Committing belongs to the user — through git, or through the interface's commit indicator (ADR-0012), which lights up precisely because the working tree changed.
4. **`doctor` reports, `migrate` repairs.** A file below the binary's current version is a `doctor` finding whose remedy is `ariane migrate --apply <path>`. `doctor` never rewrites it — ADR-0004 unchanged.
5. **A newer file than the binary is never touched.** A file at a version the binary does not know is reported, not downgraded: the user's other machine simply runs a newer `ariane`.
6. **The same engine migrates `config.toml`** (ADR-0013), which carries its own `version`.

**Migration `0 → 1` is the exception**, because it starts from prose rather than from a version. It runs in two stages:

- **A deterministic parser** for the known shapes — the bullet lists, the blockquote variants, the partial front-matter — with an explicit **vocabulary table** mapping observed values to the workflow of the node (`⏳ à faire` → a `todo`-category status, `✅ LIVRÉE` → `done`…). Everything it recognizes, it converts; it leaves the rest alone and says so.
- **An agent proposes headers for what remains** — free prose, ambiguous values, the double banners. Every field it inferred rather than read is flagged as such in the proposal.
- **The owner validates, node by node, in pull requests.** The old prose header is removed in the same change (ADR-0008); context that is not metadata — "validé par … le 2026-08-17" — moves into the body.

`pharmakon`'s French keys (`titre → title`, `statut → status`, `jalon → milestone`, `depend → depends_on`) are handled by the deterministic stage: they are the first fixture of `0 → 1`.

## Consequences

- A header change stops being a hand edit on 300 files. It becomes one migration, reviewed once as code, applied the same way everywhere.
- **Every future header change costs a migration and its fixtures.** That is the price, and it is deliberate: a change that cannot be written as a migration is a change that was not understood well enough to make.
- Mixed versions are a normal state, so every reader of the content — the interface, `ariane tasks`, the agent — must handle at least *the current version and below*, or refuse explicitly. Silently misreading an old file is the failure this rules out.
- `0 → 1` is the only migration involving an agent, and therefore the only one that is not reproducible. It is bounded: once the content is at `1`, it never runs again.
- The vocabulary table is itself a record: it documents what the free-text statuses meant, which is otherwise lost the moment the prose header is deleted.
- The binary grows a migration chain it must keep forever, or until a documented floor ("files below `N` must first be migrated with release `X`").

## Alternatives considered

### One schema version for the whole repository

A single `schema = N` in `config.toml`. Simpler to read, and it makes every file's version implicit. Rejected because it makes migration **all-or-nothing**: the repository is at `N` or at `N+1`, and there is no legal state in between. The `0 → 1` migration is explicitly planned node by node, in pull requests the owner reviews one at a time; a repository-level version would declare the whole tree migrated after the first node, or forbid the first PR until the last one is ready. Per-file versions cost one key per file and make the intermediate state honest.

### Migrate by hand, or by an agent, every time

No migration code — ask an agent to update the headers when the format changes. Rejected on the evidence already in the repository: the seven archived items with contradicting banners are what hand edits produce, and nothing caught them. An agent run is not repeatable, not testable on a fixture, and cannot be replayed on the user's other machine or on a node that was not materialized at the time. Kept for exactly one job — the prose of `0 → 1` — where there is no format to write a migration *from*.

### Let `doctor` fix outdated files

`doctor` already detects them; letting it rewrite them looks like a shortcut. Rejected by ADR-0004, whose argument applies unchanged: a `doctor` that writes can no longer run under a standing permission, in a loop, from an agent. The remedy is printed, and it is one command.

### Commit after migrating

`--apply` could commit its own change, one commit per migration. Rejected because the commit belongs to a branch and a review flow the CLI does not own — the content repository works through branches and pull requests, and `0 → 1` is explicitly reviewed node by node. A command that commits decides the branch, the granularity and the message on the user's behalf. Writing the working tree and stopping leaves all three where they belong, and the interface's indicator makes the pending change impossible to miss.

## Revisit if

- A header change turns out to be impossible to express as a deterministic `N → N+1` transformation. That would mean the format moved from data to meaning, and the agent stage would no longer be an exception.
- The migration chain becomes a maintenance burden out of proportion with the content — many versions, few files. The answer is a floor version, not removing the mechanism.
- Mixed versions cause real misreads in a tool despite the rule. That would argue for refusing to operate on a repository below the current version, at the cost of node-by-node migration.

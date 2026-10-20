# ADR-0013 — Define the configuration schema in code, and generate the example from it

- **Status:** Accepted
- **Date:** 2026-10-20
- **Concerns:** PLAN §9, §10
- **Depends on:** [ADR-0009](0009-migrate-content-with-versioned-migrations.md) (versioned migrations), [ADR-0005](0005-write-ariane-in-rust.md) (Rust)

## Context

§9 defines a logical contract: a TOML file at `~/.config/ariane/config.toml`, first key `dir_name = "_ariane"`, either versioned in the content repository and symlinked, or owned by a dotfiles manager.

**That file has never existed.** Not in the method repository, not in the content repository, not in `dotflies`:

```sh
find ~/workspace ~/.config -maxdepth 6 -name 'config.toml' | grep -i ariane   # nothing
ls ~/.config/ariane                                                           # README.md  tree
git -C ~/.config/ariane remote get-url origin                                 # git@github.com:slashome/_ariane.git
```

What sits at `~/.config/ariane` on the reference machine is a **second full clone of the content repository**, last updated 2026-08-29 with uncommitted changes. The prototype in `tools/` scans for content repositories, finds both clones, and lists every item they share **twice**. The contract of §9 was written; nothing ever produced a file conforming to it, and the gap filled itself with the wrong thing.

The configuration is also about to grow. The served interface (ADR-0012) needs a port and a list of pinned nodes with their order on the dashboard; tracker connectors (ADR-0011) need per-tracker settings. Each new key raises the same question for existing users: how does a file written against last month's schema learn about this month's keys?

The obvious answer — ship a `config.example.toml`, have `ariane config` copy it — fails at exactly that question. A copy is a snapshot. The day the example gains a key, every existing copy is behind, and nothing says so.

## Decision

**The configuration schema is defined once, in Rust, with its defaults. Everything else is derived from it.**

1. **Single definition.** The schema is a typed structure in the CLI: each key, its type, its default, its documentation string. The binary reads the file against it; an unknown key is reported, not ignored.
2. **The example is generated, not written.** `config.example.toml` is produced from the schema — every key, its default, its documentation as a TOML comment — and **committed** to the method repository, so it can be read without building anything. **CI regenerates it and fails if the committed file differs.** It cannot fall behind, because nobody edits it.
3. **`ariane config`** operates the file:
   - `init` writes a new `config.toml` from the schema (the same output as the example, with the user's answers where `init` asks);
   - `edit` opens it in `$EDITOR`, then validates it;
   - `get <key>` / `set <key> <value>` read and write one key, validated against the schema.
4. **The file carries a `version`.** When the schema changes, the change ships as a migration and reaches existing files through **`ariane migrate`** — the same engine as items (ADR-0009). Migrations edit the file with `toml_edit`, which preserves the user's comments, ordering and formatting: a new key appears, nothing else moves.
5. **What it holds, for now:** `dir_name` (§9); the server's address, default `127.0.0.1:4317`; the **pinned nodes of the dashboard and their order** — so reordering cards in the interface's edit mode writes `config.toml`, and the commit indicator lights up like for any other change; per-tracker, non-secret settings (ADR-0011).
6. **Secrets never go in it.** The file is versioned in a repository, and the first secret in sight — a Monday API token — belongs to an employer's account while the repository is personal. Token storage is deferred to the connector's implementation (content task `slashome/ariane#T001`); the keychain is the candidate.

## Consequences

- The schema, the validation, the documentation of each key and the example are one artifact. Adding a key is one change in one place, plus its migration.
- A user's configuration follows the schema forward without being regenerated, and without losing a comment.
- **Every schema change costs a migration**, like item header changes. Adding a key with a default is the cheap case; renaming or restructuring one is not, which is the right incentive.
- The example's documentation lives in Rust doc strings. Whoever writes the schema writes the user documentation; there is no other place to put it.
- Pinned nodes and card order are **shared across machines**, because the file is versioned. Someone who wants a different dashboard at work and at home cannot have it in v1.
- `doctor` gains checks with exact remedies: file missing (`ariane config init`), version behind (`ariane migrate --apply`), unknown key, value out of range. The stale clone at `~/.config/ariane` is a finding of its own — a real directory where §9 expects a file or a symlink — and falls under ADR-0004's *real directory* rule: reported, never deleted.

## Alternatives considered

### A hand-maintained example, copied by `ariane config`

What the obvious design does, and what the user explicitly wanted to avoid: changing the example and having the configuration *adapt*. A copy does not adapt; it ages. Every key added to the example after a user's `init` is a key that user does not have, with no signal. Fixing that requires a merge between the example and the user's file — which is a migration, written without a version to say what to migrate from. Generating the example from the schema and migrating the user's file by version is the same idea done once, properly.

### JSON or YAML instead of TOML

JSON has no comments, which removes the documentation the example exists to carry, and makes a hand-edited file hostile. YAML has comments but its implicit typing is a known hazard in a hand-edited file (`no` as a boolean, `3.10` as a float), and §9 already chose TOML. Neither offers what decides it here: a format-preserving editor. `toml_edit` round-trips a file byte for byte except for the keys it changes; without it, every migration would rewrite the user's file in the serializer's style.

### Local-only configuration

Keep `config.toml` out of the content repository, one file per machine. It would allow different dashboards at work and at home. Rejected for v1 because the configuration is otherwise identical across machines — `dir_name`, trackers, pins — and a local file is a file to recreate and keep in sync by hand on every machine, which is the class of drift `doctor` exists to fight. Machine-specific overrides, if they become needed, are an additive layer on top, not a reason to stop versioning the shared part.

## Revisit if

- Users need **per-machine** values often enough that overriding them becomes the norm — an argument for a local overlay file, read after the shared one.
- A key turns out to need computation or conditionals to express. The configuration would then be turning into code, and the answer is to move that behaviour into the binary, not to make the format smarter.
- `toml_edit` fails to preserve a user's formatting in a way that matters. Format preservation is load-bearing for decision 4.

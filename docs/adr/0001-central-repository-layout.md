# ADR-0001 — Central repository layout: mirror tree under a reserved prefix

- **Status:** Accepted
- **Date:** 2026-08-15
- **Concerns:** PLAN §3, §5, §9, §10

## Context

A node on disk materializes its `_ariane/` directory as a symlink into the central content repository (§4). Something has to decide **which directory** of the central repository that link points to.

The mapping is needed in both directions. Node → directory is what `init`, `update` and a human creating a node by hand need. Directory → node is what `ariane doctor` needs, since §10 requires it to diagnose hand-made setups as well as CLI-made ones: a diagnostic that cannot say which node a central directory belongs to cannot detect that a link is wrong.

One constraint applies to every candidate and is easy to miss: **the key can never be the node's absolute path.** The central repository is shared across machines, and `~/workspace/projects/atlas-project/loop` here is `~/code/atlas-project/loop` elsewhere. The key is a *logical* path, resolved against an anchor that is by nature **local to a machine**.

## Decision

The central repository mirrors the **logical** tree — the levels of §1, not the directory layout of any particular disk:

```
<central>/
  config/                 §9, versioned configuration
  registry.toml           §7, skill pack registry
  tree/                   the reserved prefix — the node tree, and only it
    INDEX.md              home level: the registry of projects
    slashome/
      INDEX.md
      dotflies/
        INDEX.md
```

Four parts, none optional:

1. **Mirror of the logical path.** `~/workspace/projects/slashome/dotflies` maps to `tree/slashome/dotflies/`. `workspace/projects/` is a local filing convention and stays in the anchor.
2. **A reserved prefix, `tree/`.** Without it the node tree occupies the root of the central repository, and a project named `config` overwrites `<central>/config/`.
3. **A back-pointer.** Each `INDEX.md` carries `node: slashome/dotflies` in its front-matter. It is redundant with the file's location, and that is the point: redundancy is what lets `doctor` distinguish *moved deliberately* from *moved by accident*.
4. **Anchors live in the local configuration**, not in the central repository. They are the only registry, they are a handful of lines, and being per-machine they can never conflict.

On node names: **forbid rather than encode.** A node is a project or a repository; there is no legitimate reason for it to be named `.github`, contain a space, or run to 200 characters. `doctor` refuses such names with an explicit error rather than transforming them silently, and flags sibling nodes differing only in case (APFS is case-insensitive here — verified: `/Users/ALUCARD/workspace` resolves). chezmoi encodes because it *must* accept pre-existing dotfiles; Ariane has no such debt and must not adopt it.

## Consequences

- The central repository is readable without any tool: an agent dropped into it understands the tree by reading it, which is the `INDEX.md` principle applied to the directory structure itself.
- The reverse lookup `doctor` depends on is a prefix subtraction — exact, O(1), and impossible to desynchronize from the forward mapping.
- Renaming or moving a node moves files and produces rename commits. Accepted: renames are rare, and `update` handles the relink.
- Intermediate nodes always exist as directories. They are never empty in git's sense, since §2 requires an `INDEX.md` per node.
- Reserved names at the root of the central repository (`tree/`, `config/`, `registry.toml`) must be specified, along with the behaviour when a project carries one of those names.
- macOS case-insensitivity and NFD/NFC normalization become `doctor`'s problem (`core.precomposeunicode`), not a naming scheme's.

## Alternatives considered

### Flat directories, name derived from the path (file-based-routing style)

`slashome/dotflies` → `slashome-dotflies/`, computed by a transformation rule, no registry at all. Attractive because everything is deducible and nothing has to be maintained.

It breaks on ambiguity, and **the collision already exists in this user's real tree**: `slashome/dotflies-rust-legacy` and a future `slashome/dotflies/rust-legacy` — filing the legacy repository under the main one, a plausible refactor at any time — both produce `slashome-dotflies-rust-legacy`. The separator is also a legal character in node names.

An escaping scheme fixes it *mathematically*: escape the separator before joining (systemd's `systemd-escape` turns a literal `-` into `\x2d`), and escape case the way Go's module cache does (`Azure` → `!azure`, precisely so the tree can be served from a case-insensitive filesystem). But the escaping destroys the option's only selling point. Nobody reads `slashome-dotflies--rust--legacy`, and more importantly **nobody creates it by hand** — while §10 requires `doctor` to validate hand-made setups.

Two further observations settled it:

- **The file-based-routing precedent is misapplied.** Nuxt never flattens the tree: `pages/blog/[slug].vue` stays hierarchical on disk, and it is the *route* (a string) that is derived from the *tree*. The real analogue of file-based routing in this debate is the mirror.
- **Every derived flattening ends up carrying a back-pointer**, i.e. a decentralized registry — converging on the option below, less readably. VS Code stores per-project state in `workspaceStorage/<hash>/` and had to slip a `workspace.json` holding the original URI *inside* each directory because the reverse lookup was otherwise impossible; the community still had to write a garbage collector for the orphans left by moved projects. direnv (`allow/<sha256 of path>`) made the same choice without a back-pointer, and the result is not auditable at all.

### Flat directories with an explicit registry

Short names, and a written node ↔ directory mapping. It wins on exactly one thing, and wins it convincingly: renaming or moving a node edits one line and moves no files.

It loses on everything else. It is the `git worktree` model — `.git/worktrees/<id>/gitdir` holds an absolute path, the worktree holds a `.git` pointing back — and git had to add `git worktree prune`, then in 2020 `git worktree repair`, including a multi-directional variant, **because humans move directories by hand**. That is twenty years of hindsight on the maintenance cost of a second source of truth.

It also contradicts §10 directly: a setup made by hand is invalid by default, because the symlink is right, the content is right, and the registry line is missing. `doctor` must then either raise a false positive or guess — and if it guesses, the mirror has been reimplemented as a fallback.

Finally, it loses the hierarchy inside the central repository. §1 makes the tree the founding concept and §2 makes agent navigation the target use; a flat central repository forces an agent to read a mapping file before it can understand anything, which is the inverse of the `INDEX.md` principle.

### Mirror of the full disk path

`tree/workspace/projects/slashome/dotflies/`. Rejected because it encodes, in an artifact **shared between machines**, information that is **local to one machine**. A second machine that files its repositories elsewhere writes into a different subtree for the same node, silently duplicating content — a literal violation of §3's "one clone, one history". Fixing that requires raising the anchor, which is to say rebuilding the option above. It also mirrors `$HOME` in the end, `Documents/` and the rest included, and cannot represent a worktree created outside the projects tree at all.

### The "history is preserved" argument for flat layouts

Rejected on a factual basis: git does not store renames, it detects them by similarity. `git mv` of a subtree followed by `git log --follow` restores the history. Renaming a node under the mirror costs one `git mv` and one `ln -sfn`, both of which `ariane update` performs on its own.

## Revisit if

- **A single Ariane content subtree must serve several *logically distinct* nodes.** The mirror is 1:1 by construction. Note that git worktrees are **not** this case: several materializations of the *same* logical node share one central subtree through several symlinks, which the mirror handles natively (see ADR-0003).
- **Multi-user projects** (post-v1) require one central repository to hold several users' trees.
- **A node's content must outlive the node with a citable external identity** (permalinks, archival), which would call for an opaque ID.
- **Scale changes**: beyond a few hundred nodes with weekly renames, the churn of rename commits would become the dominant cost. Currently around 45 nodes with rare renames.

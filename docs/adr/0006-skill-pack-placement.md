# ADR-0006 — Declare skill packs per node, install them once at user level

- **Status:** Accepted
- **Date:** 2026-08-15
- **Concerns:** PLAN §5, §7, §8, §10

## Context

§7 says the CLI installs registered skill packs "into the platform's skill location". That phrase assumes a single place. It is not one.

Verified against the Claude Code documentation and on a real machine:

| Scope | Path | Reach |
|---|---|---|
| Enterprise | managed settings | organization |
| Personal | `~/.claude/skills/<name>/` | every project |
| Project | `<repo>/.claude/skills/<name>/` | that repository |
| Plugin | `<plugin>/skills/<name>/` | wherever the plugin is enabled |

Four facts carry the whole decision, each verified rather than assumed:

1. **Precedence is `enterprise > personal > project`** — the personal scope *overrides* the project scope, the inverse of every cascading-configuration tradition.
2. **Project skills climb to the repository root and stop there**, while `CLAUDE.md` climbs past it. Observed directly: a skill sitting in a non-repository parent directory is loaded when the session starts there, and silently absent when the session starts in any repository below it — even though the `CLAUDE.md` of that same parent *is* loaded and instructs the agent to use that skill.
3. **No arbitrary skill path is declaratively configurable.** Symlinks, however, are officially supported at enterprise, personal and project scope, and are deduplicated when the same target is reachable twice.
4. **The same two-tier shape holds on Cursor** (`~/.agents/skills/`, `~/.cursor/skills/`, plus `.claude/skills/` compatibility) — user and repository, no intermediate scope.

Two consequences follow that the plan did not anticipate. The project scope lives **inside a host repository**, which ADR-0003 forbids Ariane from writing to, without exception. And on this machine no directory between a repository root and `$HOME` is a git repository — so **Ariane's project level is structurally incapable of hosting skills**, on either platform.

The proposal that prompted this record was to hoist a pack to the nearest common ancestor of the nodes that need it, the way a package manager does.

## Decision

> **Each node declares the packs it needs, at that node, and that declaration never moves. `ariane update` installs the union of the packs declared by the nodes materialized on this machine, deduplicated by name, into the platform's user-level skill location, by symlink.**

**No hoisting.** A package manager hoists because its resolution is *path-relative*: `require` walks up from the calling file, so where a package sits determines who can resolve it, and moving it to an ancestor is an act. Skill visibility is not path-relative on any agent platform — every declared pack resolves to the same installation target no matter which node declared it. **There is nothing to hoist.** What the resolved set answers is not *where to install* but *which nodes are exposed to what*, which is what `doctor` reports.

**The registry maps node → packs.** No installation location appears in a declaration; the field does not exist. A pack later needed by a second node is declared a second time, never relocated.

**Never inside a host repository.** A pack is never installed into `<repo>/.claude/skills/`. A host repository's own skills belong to its team: Ariane never reads them as its own, never writes there, and never reports their contents as a finding.

**Installation is a symlink, never a copy** — the same mechanism as §4, so skills become one more materialization rather than one more concept. Content is never duplicated in either direction:

- a **third-party pack** is fetched from its external source into a disposable local cache outside both the central repository and every host repository (`~/.local/state/ariane/packs/<name>/<version>/`, consistent with §9's XDG choice), and never copied into the user's `_ariane` repository;
- an **owned pack** lives in the central repository at its node (`<central>/tree/<node>/skills/<name>/`), materialized there through `_ariane/` like any other content, and never copied out. The author edits it in place at the node.

**One pack, one version, per machine.** Two nodes declaring different versions is refused: `update` changes nothing, `doctor` exits `2`. Name collisions between distinct packs are refused the same way, and Ariane never renames a pack silently.

**Materialization is Ariane's only scoping mechanism**, and its granularity is the machine. Packs declared by nodes absent from this machine are not installed and are reported as information, exit `0`. The result stays a pure function of *(central declaration, machine configuration)* — the machine has an effect, but a **declared** one, never an accidental one, since the anchors are themselves configuration.

**"Personal" is not a scope.** Authorship is an attribute of a pack's source and never determines placement. Applicability does. The test is one question: *would this pack still mean something if the node disappeared?* Yes, it belongs to an ancestor; no, it belongs to the node. A pack serving all of a user's work is declared at home whoever wrote it; a pack serving one node is declared there even if a third party wrote it.

## Consequences

- Skills reuse §4's mechanism entirely. No new concept, no second installer, and the platform deduplicates symlinks for us.
- The registry travels: the same central tree yields the same declarations on every machine, and `update` is the only thing that varies with the local configuration.
- **A pack that Ariane installs at user level overrides a same-named skill a team committed in their repository** — a consequence of the platform's inverted precedence, not of Ariane. `doctor` can see both and must report the overlap; the resolutions (rename the pack, stop declaring it on this machine, accept the override knowingly) are laid out for a human, per ADR-0004.
- **On a machine materializing two clients' work, Ariane cannot isolate them.** User scope is global by the platform's definition. The answer is one machine — or one profile, if the platform ever offers a documented way to relocate its configuration directory — not a feature. Stated plainly rather than papered over.
- The skill listing consumes a measurable context budget, so installing packs no materialized node declares would starve the rest. The scoping rule and the context budget reinforce each other, which is a good sign.
- Collisions between two third-party packs are unsolvable by Ariane and stop automation at exit `2`, by design.

## Alternatives considered

### Hoisting to the nearest common ancestor

The proposal that opened this record. Rejected not because it is wrong but because **it is empty**: it computes a destination that was already the only one available. The observation behind it — *two projects need it, so it is global* — is a true description that calls for no action.

The case that settles it is the pack needed by a second node later. Under hoisting-as-declaration, the declaration would have to **move** from one node's entry up to the tree root: a modification to a third party's node, a merge-conflict magnet, and the loss of the fact actually known (*this node needs this pack*). Under the rule adopted, nothing moves and the second node adds a line to its own entry — an addition stays an addition. Note the corollary: **hoisting, where it does exist, is computed and never declared.** Nobody edits a `package.json` to hoist.

### Installing everything declared, on every machine

Attractive for determinism: the same central tree would yield the same installed set anywhere, and the machine would have no effect at all.

Rejected because it discards the only scoping mechanism available. Every pack of every client and every side project would be loaded in every session, spending the shared skill-listing budget and putting one client's tooling in front of an agent working for another. The determinism gained is also smaller than it looks: the adopted rule is *already* deterministic, since the materialized set is derived from declared configuration rather than from whatever happens to be lying on the disk.

### Installing into `<repo>/.claude/skills/`, the platform's project scope

The scope that would map onto Ariane's repository level. Forbidden outright by ADR-0003: it writes inside a host repository. It would also be *overridden* by anything at user level, given the inverted precedence — so it would be both illegal and unreliable.

### Nested installation, the way `node_modules` resolves version conflicts

Impossible rather than undesirable: there is no nested user-level location, and the only nested location that exists is inside a host repository.

### Renaming on collision (`fsd`, `fsd-2`)

Rejected: it breaks a pack's internal references and pushes the version number into the command name the user types. The resolver would leak into the user's fingers.

Behind it sits the reason the whole package-manager analogy stops here. A library can be versioned in parallel because its callers are code with lexical scope. **A skill's caller is a model, with one flat command namespace and one context window.** Two `/fsd` in the picker is not a resolved conflict, it is a coin toss — and that is literally what the platform does with same-named skills: both stay available and the model chooses. Ariane must not build a versioning scheme whose failure mode is an LLM picking. This is a monorepo's single-version rule, not a package manager's resolution.

### Installing packs as plugins rather than as bare skills

Genuinely attractive: the `plugin:skill` namespace makes collisions impossible by construction, which is the platform's only anti-collision mechanism. Not retained for v1 because plugins treat symlinks differently, which costs the deduplication and the simplicity of a single link. Worth instructing separately.

## Revisit if

- **A documented way to relocate the platform's configuration directory turns out to exist.** That would be the only per-profile isolation available, and the answer to cross-client exposure. Suspected, unverified — nothing may be built on it until confirmed.
- **Someone actually needs the full declared set installed regardless of materialization.** The escape hatch is a single configuration key and this decision is a strict subset of it, so it can be added without revisiting anything else. It is deliberately not shipped: a second behaviour doubles what has to be specified, tested and diagnosed — including ADR-0004's testable pairing of `doctor` findings with `update`'s plan — and no one has asked for it.
- **A platform gains an intermediate scope** between user and repository, which would give Ariane's project level somewhere to land and reopen the hoisting question properly.
- **A pack that deploys many skills** (BMAD-scale) makes the collision check ambiguous: the registry declares a pack, but collisions occur between *skills*. If the mapping is not one-to-one, the check has to be specified over the deployed set — and that is a specification gap, not a decision to revisit.

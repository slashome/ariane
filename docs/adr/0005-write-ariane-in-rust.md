# ADR-0005 — Write the `ariane` CLI in Rust

- **Status:** Accepted
- **Date:** 2026-08-15
- **Concerns:** PLAN §10, roadmap step 7

## Context

§10 picks Go, and justifies it like this:

> a single static binary with no runtime dependency, because the tool that diagnoses a machine must not depend on that machine being healthy (or on Node being installed at all)

**That justification is true of Go, Rust, Zig, C and OCaml alike.** It rules out Node and Python; it does not separate the two candidates actually under consideration, and it never did. The choice was therefore never arbitrated.

Meanwhile the sibling project [dotflies](https://github.com/slashome/dotflies) moved from Go to Rust before writing a line of code (its ADR-0006), on the grounds that *the core of the product is a sum type, and Rust is the only serious candidate that makes a missed case a compile error*. Whether that argument transfers to Ariane is the question this record settles, and the answer is not the obvious one.

## Decision

**The `ariane` CLI is written in Rust.**

One design rule is a **condition** of this decision rather than a detail, and it has to hold from the first commit:

> **Raw `OsStr` / `PathBuf` at the git boundary and for every observed path; `Utf8PathBuf` only in the declared layer** — configuration, anchors, the mirror tree.

Notably, and unlike dotflies, **`camino` is not adopted globally.** It is the ergonomic choice that made Rust comfortable there, and here it is a trap: a diagnostic tool's job includes *reporting* the badly-named path, and `camino` refuses to construct exactly those paths.

Libraries, aligned on dotflies so that consistency means something concrete: `clap` (derive), `serde`, `toml`, `serde_json`, `anyhow` everywhere except the convergence engine, `thiserror` where errors are asserted in tests, `dirs`, `owo-colors`, `tempfile`, `assert_cmd` + `predicates`, and **`insta`** — a `doctor` report is a snapshot by nature. Front-matter parsing uses a maintained library, never a hand-rolled parser: the `node:` back-pointer of ADR-0001 is human-written, and it is the field the *moved deliberately* versus *moved by accident* distinction depends on.

## Consequences

- One systems language across the sibling projects, one release pipeline pattern, one set of idioms. `classify_link` — the link classification Ariane needs — already exists in dotflies as thirty lines of `match`, and becomes a paste that compiles rather than a rewrite with two places to fix the same edge-case bug.
- The output contract becomes structurally enforced rather than test-enforced (see below).
- The git boundary is less comfortable than it would be in Go, permanently. The rule above is the price, and it is a real one — it must be documented in `CONTRIBUTING.md`, not discovered.
- A contributor without a toolchain faces `rustup` and a cold build, which is a higher barrier than `go build`. The direction is certain, the magnitude is not measurable in advance.
- The `go install` channel is given up. `cargo install --git … --locked` works but is not discoverable, and publishing to crates.io means an irreversible name reservation.

## Alternatives considered

### Go, as §10 currently states

Rejected, but the reasoning has to be exact, because the obvious version of it is wrong.

**Where the sum-type argument does *not* transfer.** The states of a materialized link — conformant, absent, pointing elsewhere, a real directory, tracked by the host repository, not ignored by git, worktree without a link — are **not one closed set**. The first four are mutually exclusive: that is an `lstat` plus a `readlink`. The next three are **orthogonal** — a link can be conformant *and* tracked *and* unignored. That is a product of independent predicates, not a sum. And the core of `doctor` is an **open list of checks** that will grow; adding the fourteenth check costs the same in either language. On this ground the dotflies argument is *weaker* for Ariane, not stronger.

**Where it does transfer, and decisively.** Ariane's closed set is not on the input side, it is on the **output** side, and ADR-0004 makes it a requirement:

> The set of `doctor` findings whose remedy is `ariane update` **is** `update`'s plan.

Encoded as `enum Remedy { Update(Action), Adopt(..), Human(Resolutions) }`, where `Action` is exactly what the convergence engine consumes through an exhaustive `match`, that requirement stops being a property to test and becomes **an unrepresentable state**: a finding can no longer claim `update` repairs it without naming the action `update` will run, and the day a new action appears the compiler names every site — the renderer, the JSON serializer, the fold that computes the exit code, the engine itself. In Go it is a string constant and a `switch` with `default`, and the invariant falls back to a fixture test. **The fixture test catches the cases someone wrote; the type catches all of them.** The same holds for `--json`, which is a *published interface consumed by an agent* rather than a debug format: serde distinguishes absent from empty and makes a schema change break at compile time instead of at the consumer.

**The honest concession.** In dotflies a missed case means writing into a user's `.zshrc` — its documented number-one failure mode. In Ariane, `doctor` is read-only by construction of the binary (ADR-0004, no `--fix`), and `update`'s destructive surface is already mechanically bounded. A missed case costs **a false report, not lost files**. Rust's margin here is not about safety, it is about the integrity of the output contract — which ADR-0004 holds to be the deliverable. Narrower than in dotflies, and real.

**Go's genuine advantages, and why they do not carry.**

- *Bytes at the git boundary.* Real, and the best argument against this decision. `os/exec` plus a NUL split keeps bytes as bytes; Rust needs a deliberate path-type policy — hence the condition above.
- *The Linux port.* This was Go's one decisive point in dotflies, and **it nearly vanishes here.** dotflies has a package-manager layer to write from scratch; Ariane has POSIX symlinks, `lstat`, an XDG path and git subprocesses, all identical. The macOS-specific parts named in ADR-0001 — APFS case-insensitivity, NFD/NFC, `core.precomposeunicode` — are what a Linux port *removes*, not what it adds. What an outside contributor will actually write is one more check returning findings, which is the most approachable subset of Rust.
- *Distribution.* This is the criterion §10 assumed favoured Go, and **it points the other way.** The tap `slashome/homebrew-tap` contains `redlight.rb`, a **Rust** formula that already passes the `brew audit --strict --online` its CI runs on every formula on every PR. There is no Go formula, and no precompiled-binary machinery for any language. Rust copies an audited template; Go means inventing the house's first Go formula. Precompiled binaries stay a later optimization, where goreleaser is genuinely more mature than cargo-dist — worth about a day, not worth a language.
- *`go install`.* A real advantage: no registry, the module proxy serves the git tag. But that channel only serves people who already have Go, and **`go` is not installed on the author's machine** while `cargo` and `node` are. §10 currently offers a fallback channel its own author cannot run.
- *Author fluency.* dotflies' ADR-0006 named this as its principal risk — *the author writes faster in TypeScript* — with an explicit falsification window of three weeks. The bet was settled in under 24 hours: 1,704 lines of Rust across ten modules with 30 tests, `doctor` and `apply` working. The cost that was named is measured, and it is nil.

### TypeScript compiled to a single binary

The strongest runner-up, and stronger here than it was for dotflies: it is the author's fastest language, the consumer of `--json` is an agent, and the rest of the method already lives in that ecosystem (`adapters/claude/skills/…`, BMAD packs, MCP) — a contributor to the *method* could read the CLI. `bun build --compile` and `deno compile` remove the installed-runtime dependency, so §10's objection no longer applies as written.

Rejected on three grounds, in order. **Exhaustiveness is a discipline there, not a property**: `default: assertNever(x)` must be written at every site, therefore remembered — which is precisely the class of failure ADR-0003 forbids in one sentence, *what must hold invariably is the CLI's job to verify*. **Packaging**: a 60–100 MB unnotarized binary, in a tap that has two formula patterns and none for this one, reopening the Gatekeeper question entirely. And §10's *intent* is right even though its justification is badly written: the tool that diagnoses a machine should not depend on that machine's health, and a self-extracting runtime reduces that dependency without removing it.

### Zig

Rejected for the same reasons dotflies rejected it: no mature TOML parser, no `clap` equivalent, a moving standard library — and nothing to gain here, since Ariane has neither a binary-size constraint nor an allocation-control problem.

## Revisit if

Four facts, observable within three months, in order of strength:

1. **The one that targets this decision's own core.** If, by the time `doctor` has a dozen checks, the history contains **no** commit where adding a `Remedy`/`Action` variant forced simultaneous changes in at least two other modules, then exhaustiveness never caught anything, ADR-0004's fixture test was sufficient, and this choice rested on habit rather than on its argument. Checkable with `git log -S` on the enum.
2. **The mirror of dotflies' own condition.** If `ariane doctor` has no green tests against a real machine within three weeks, the bet is lost — a window dotflies cleared in a day, so it is calibrated rather than generous.
3. **Collapse of the path layer.** If more than roughly 30% of the code has to handle `OsStr` or raw bytes because git's output forces it, Rust's ergonomic premise fails and Go's advantage on subprocesses becomes the dominant criterion. Measurable with `grep -c OsStr`.
4. **The cost of contribution.** If at least two of the first five external issues or pull requests are about failing to build rather than about the method, then the Go talent pool was worth more than this record estimates.

# ADR-0012 — Serve the interface from the `ariane` binary

- **Status:** Accepted
- **Date:** 2026-10-20
- **Concerns:** PLAN §10, roadmap step 7; [ADR-0004](0004-write-or-judge.md), [ADR-0005](0005-write-ariane-in-rust.md), [ADR-0008](0008-give-every-item-a-versioned-front-matter.md), [ADR-0010](0010-keep-only-judgment-in-index-files.md), [ADR-0013](0013-define-the-configuration-schema-in-code.md)

## Context

The only interface Ariane has today is `tools/ariane-tasks.py`, which `tools/README.md` declares **temporary by contract**: it stands in for `ariane tasks` and is to be deleted when that command ships. What it is, verified in the file:

- 826 lines of Python standard library, one `http.server` with a single `do_GET` handler — **read-only**;
- `port = int(os.environ.get("ARIANE_PORT", "0"))` — port `0` by default, so the OS picks a **different port on every run**;
- a manual **Rescan** button: the page shows the tree as it was when the process started, until someone clicks;
- one table. No dashboard, no kanban, no way to act on an item.

It has nonetheless proved the need: a cross-cutting view of the items is the one thing no `INDEX.md` gives (PLAN §10), and with ADR-0010 removing item tables from the index files, it becomes the *only* place the aggregate is seen by a human.

What the owner asked for goes well beyond a table: an admin-style application — header and left navigation, in the manner of Sentry or CircleCI — with a dashboard of pinned nodes, the task list, a kanban by workflow status, and a size × impact matrix; callable from anywhere as `ariane serve`; optionally kept up permanently; and able to **act**, not only show.

Two constraints are already settled elsewhere. ADR-0005 puts the CLI in Rust, as a single binary with no runtime dependency. ADR-0004 splits commands into those that write and those that judge. An interface that moves cards writes; it has to find its place in that split rather than blur it.

## Decision

> **`ariane serve` is one process: the binary serves an embedded web application over the files it reads and writes. Files stay the source of truth. Nothing is committed behind the owner's back.**

**Architecture.**

- **A single process, no client/server split beyond the browser.** An `axum` server inside the `ariane` binary. There is no daemon to talk to, no database, no API meant for anything but the embedded front.
- **Files are the source of truth.** The server builds an in-memory index of the tree at start (front-matter of ADR-0008, `node.toml`, git history for dates) and keeps it current with **`notify`** file watching. Changes are pushed to the browser over **Server-Sent Events**. There is no Rescan button: an edit made in an editor, by an agent, or by `git pull` appears on its own.
- **The front is React + Vite + TypeScript, organised in Feature-Sliced Design and checked by Steiger.** It is built at release time and embedded in the binary with **`rust-embed`**. Node is required to *build* Ariane, never to *run* it — the no-runtime-dependency rule of ADR-0005 holds for the user.
- **Bound to `127.0.0.1:4317` by default**, configurable through `config.toml` (ADR-0013) and `--port`. Never bound to another interface, so there is **no authentication**, deliberately: anyone who can reach the port is already on the machine as the user.

> **Amended 2026-10-23.** The default port is **27317**: 4317 is OpenTelemetry's OTLP/gRPC port. The server also opens the front door of `ariane proxy` on `127.0.0.1:27380`, which serves the interface as `http://ariane.local` ([ADR-0016](0016-name-local-servers-through-a-front-door.md)); both stay on loopback.

**Writing.**

- **The interface writes files.** Moving a kanban card rewrites the item's `status`; editing a field rewrites the front-matter; reordering dashboard cards rewrites `config.toml`. Writes go through the same front-matter writer the CLI uses, never a second one.
- **It never commits on its own.** A **header indicator** shows that the working tree has uncommitted changes. Hovering it opens a menu of commit actions (commit everything, commit per node, push…); a modal for finer control comes later. What is committed, when and with which message stays the owner's act.
- **Position against ADR-0004.** The server is a *writing surface*, in the same family as `update`, `migrate` or `sync`: it changes files and claims nothing about their validity. **Judgment stays in `doctor`.** The interface displays `doctor`'s findings — invalid headers, suspended sync items (ADR-0011) — by running the same checks, and offers their enumerated resolutions; it does not get a private notion of what is valid.

**Layout and views.**

- Admin layout: a header (node switcher, search, the commit indicator) and a left navigation.
- **Dashboard** of pinned nodes. Each card shows, compactly: counts by status category, last movement, quick wins (small size × high impact), and blockers. Cards are reordered in an **edit mode** that enables drag and drop; pins and order live in `config.toml`, so they follow the owner across machines and light the commit indicator when they change.
- **Task list**, the successor of the prototype's table.
- **Kanban**, one column per status of the node's workflow (ADR-0008); on a multi-node view, one column per status category.
- **Size × impact matrix**, sizes XS–XL by impact low/medium/high, for tasks and stories.

> **Amended 2026-10-23.** Pins and their order, and the interface's theme, live in `settings.toml` at the root of the content repository, not in `config.toml`, which is versioned nowhere ([ADR-0013](0013-define-the-configuration-schema-in-code.md), amended). The interface's git actions cover `settings.toml` next to `tree/`.

**Permanent availability.**

- `ariane service install | uninstall | status | logs` generates and loads a **launchd agent** on macOS, a **systemd `--user` unit** on Linux, pointing at `ariane serve`. The binary never daemonizes itself.

## Consequences

- The prototype is deleted when `ariane serve` reaches parity with its table, as `tools/README.md` promised.
- Building Ariane now requires Node and a JavaScript toolchain next to `cargo`. The release pipeline grows a front build step, and a contributor touching only the CLI still needs the embedded assets to exist — a stub or a prebuilt bundle has to be provided, or every `cargo build` pays for Vite.
- Two languages in one repository, each with its own linter: Steiger for the front's layering, Clippy for the rest. The FSD layering is a real cost for an interface this size; it is accepted because the owner's daily front-end practice uses it, and an unfamiliar structure would cost more.
- File watching has edge cases that the prototype never met: editors that write through a temporary file and a rename, a `git checkout` that changes hundreds of files at once, network or synced folders where events are unreliable. Debouncing and a full rebuild on burst are required, and a missed event shows a stale board until the next one.
- Writing from the interface means two writers on the same files: the owner's editor or agent, and the board. A card moved while the file is open in an editor produces a conflict the server must detect (content changed since read) and refuse, never merge.
- No authentication is safe only on loopback. The day someone wants the board on their phone or shared with a colleague, this decision has to be reopened rather than patched with a flag.
- A fixed default port can collide with something else on the machine. The error has to name the port and the configuration key, not just fail to bind.

## Alternatives considered

### Keep Python and grow the prototype

It works today, and it is 826 lines. ADR-0005 already ruled out runtime-dependent languages for the CLI: the tool that diagnoses a machine must not depend on that machine having the right interpreter. And `tools/README.md` makes the prototype throwaway by contract — "a tool that needs a lockfile belongs in the CLI, not here". A dashboard, a kanban with drag and drop, file watching and a writer would need exactly the dependencies that rule forbids.

### Server-rendered HTML with htmx

No front build, no Node, everything in Rust templates. The core of this interface is precisely what htmx makes expensive: a kanban with drag and drop across columns, a sortable and filterable table, a dashboard reordered by dragging. Each of those is a solved component in the React ecosystem and a hand-written client script under htmx. The build step saved is paid back in the first view.

### Svelte

Less code per component and a lighter bundle. The two components the interface depends on most — a rich data table and drag and drop — have their mature implementations in React (TanStack Table, dnd-kit) and younger ports in Svelte. React is also the owner's daily stack, and FSD with Steiger is a practice the owner already runs on it. Concision does not buy back an ecosystem gap on the exact pieces that matter.

### A desktop application (Tauri)

A native window instead of a browser tab. It is not callable from anywhere as a subcommand, it cannot be kept up as a service and reached from any browser on the machine, and it adds a second packaging and signing pipeline per platform. The requirement is `ariane serve`, not an app.

### `ariane serve --daemon`, the binary detaching itself

One flag, no system integration. It does not restart after a crash, does not start at login, and its logs go wherever the detached process last pointed them. Those three properties are what launchd and systemd exist for; reimplementing them in the binary is reimplementing them badly. `service install` writes ten lines of plist or unit file and delegates.

### A read-only interface

The prototype's posture, and the conservative reading of ADR-0004. The owner wants to act from the board — move a card, set a size, pin a node — and a read-only board sends every such act back to an editor, which is the friction the interface is meant to remove. The ADR-0004 boundary is kept where it matters: the interface writes, `doctor` judges.

### Commit on every action

The board would always be in sync with git. It floods the history with one commit per card moved, removes any chance of reviewing a batch of changes before they land, and decides on the owner's behalf which branch receives them — in a content repository whose flow is branches and pull requests. The indicator keeps the commit an explicit act.

### A random port, as the prototype does

No collisions, ever. It is incompatible with a permanent service — the URL changes on every restart — and with a bookmark, which is how a permanent dashboard is opened.

## Revisit if

- The board needs to be reached from another device, which would require authentication and a different binding decision.
- Conflicts between the board and an editor turn out to be frequent in practice, which would argue for locking or for a different write model.
- The front build makes CLI contributions noticeably harder — measured as contributors blocked by the asset step — which would argue for shipping the front separately and loading it at runtime.
- File watching proves unreliable on a platform Ariane supports, which would bring back an explicit rescan, at least as a fallback.

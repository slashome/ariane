#!/usr/bin/env python3
"""Cross-cutting view of the items of an Ariane tree — throwaway prototype.

Stands in for a future `ariane tasks` command of the Rust CLI (PLAN §10, ADR-0005),
and is meant to be deleted once that exists.

Discovers `_ariane` paths under $HOME, walks up to the central content repository,
inventories every lifecycle item (reflexions/backlog/tasks/stories/archives) and
serves them as one sortable table on a local ephemeral HTTP server. Clicking an
item opens its file rendered as markdown.

Item files are not normalized yet (SPEC.md and templates/ are roadmap steps 3-4),
so status, priority and creation date are recovered by best-effort heuristics. An
empty cell means the file does not say it in a recognizable form — not that the
item has no status.
"""

from __future__ import annotations

import http.server
import json
import os
import re
import socketserver
import subprocess
import sys
import urllib.parse
import webbrowser
from pathlib import Path

# ---------------------------------------------------------------- configuration

DIR_NAME = "_ariane"  # config.toml: dir_name
LIFECYCLE = ("reflexions", "backlog", "tasks", "stories")
ARCHIVES = "archives"
MAX_DEPTH = 7

# Never traversed while scanning $HOME: either they cannot hold a node, or they
# cost hundreds of thousands of inodes to walk.
SKIP_DIRS = {
    ".git", ".hg", ".svn", "node_modules", "__pycache__", ".venv", "venv",
    "vendor", "target", "dist", "build", ".next", ".nuxt", ".output", ".cache",
    "Library", "Applications", ".Trash", "Movies", "Music", "Pictures",
    ".npm", ".pnpm-store", ".yarn", ".nvm", ".cargo", ".rustup", ".gradle",
    ".m2", ".docker", ".android", ".local", "go", ".bun", ".deno", "Photos Library",
}


# ------------------------------------------------------------------- discovery

def scan_home(home: Path) -> list[Path]:
    """Every path named `_ariane` under $HOME, directory or symlink alike."""
    found: list[Path] = []
    stack: list[tuple[Path, int]] = [(home, 0)]
    while stack:
        current, depth = stack.pop()
        try:
            entries = list(os.scandir(current))
        except (PermissionError, OSError):
            continue
        for entry in entries:
            name = entry.name
            if name == DIR_NAME:
                found.append(Path(entry.path))
                continue  # a node never holds another node under that name
            if depth >= MAX_DEPTH or name in SKIP_DIRS:
                continue
            # follow_symlinks=False: links are not traversed, so the walk cannot
            # loop, nor reach the central clone again through a materialization.
            try:
                if entry.is_dir(follow_symlinks=False):
                    stack.append((Path(entry.path), depth + 1))
            except OSError:
                continue
    return sorted(found)


def resolve_central(node_path: Path) -> Path | None:
    """Walk up from an `_ariane` path to the root of the central clone.

    The central repository is the ancestor — or the path itself — carrying both
    `tree/` and `.git`. Materializations point into its subtree.
    """
    try:
        real = node_path.resolve(strict=True)
    except OSError:
        return None
    for candidate in [real, *real.parents]:
        if (candidate / "tree").is_dir() and (candidate / ".git").exists():
            return candidate
    return None


def materializations(paths: list[Path],
                     central: Path) -> tuple[dict[str, list[str]], list[str]]:
    """Map each logical node of the central clone to where it is materialized.

    Also returns the suspect links: an `_ariane` path must resolve inside `tree/`
    onto a directory carrying an `INDEX.md`. Anything else is deployment drift —
    `ariane doctor`'s territory, reported here only in passing.
    """
    tree = central / "tree"
    result: dict[str, list[str]] = {}
    warnings: list[str] = []
    for path in paths:
        try:
            real = path.resolve(strict=True)
        except OSError:
            warnings.append(f"{path} — broken link")
            continue
        if real == central:
            # The central clone itself is not a materialization: it carries that
            # name because it *is* the repository. A *link* to it, however, exposes
            # the repository root (`.git`, `tree/`) where a node is expected.
            if path.is_symlink():
                warnings.append(
                    f"{path} → the root of the central clone, not a node: "
                    f"no INDEX.md there (expected: {tree})")
            continue
        try:
            key = str(real.relative_to(tree))
        except ValueError:
            warnings.append(f"{path} → {real}, outside {tree}")
            continue
        if not (real / "INDEX.md").is_file():
            warnings.append(f"{path} → {key}, which has no INDEX.md")
        result.setdefault(key, []).append(str(path))
    return result, warnings


# --------------------------------------------------------------------- reading

FRONTMATTER_RE = re.compile(r"\A---\r?\n(.*?)\r?\n---\r?\n", re.DOTALL)
H1_RE = re.compile(r"^#\s+(.+?)\s*$", re.MULTILINE)
STATUS_RE = re.compile(r"stat[ue][st]?\s*[:=]\s*(.+)", re.IGNORECASE)
PRIORITY_RE = re.compile(
    r"\*\*\s*(?:priorit[ée]|priority)\s*\*\*\s*[:=]\s*(.+)", re.IGNORECASE)
CREATED_RE = re.compile(
    r"cr[ée]{1,2}e?\s*(?:le)?\s*[:=]?\s*(\d{4}-\d{2}-\d{2})", re.IGNORECASE)
DATE_RE = re.compile(r"(\d{4}-\d{2}-\d{2})")
MONDAY_ID_RE = re.compile(r"^(\d{9,})[-_]")


def clean(text: str, limit: int = 90) -> str:
    """Reduce a markdown fragment to text readable in a table cell."""
    text = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"[*_`]", "", text)
    text = re.sub(r"^>+\s*", "", text).strip(" -—:.")
    text = re.sub(r"\s+", " ", text)
    return text[:limit] + ("…" if len(text) > limit else "")


def parse_frontmatter(raw: str) -> dict[str, str]:
    """Flat YAML front-matter, enough for `node:` (ADR-0001) and its neighbours."""
    match = FRONTMATTER_RE.match(raw)
    if not match:
        return {}
    fields: dict[str, str] = {}
    for line in match.group(1).splitlines():
        if ":" in line and not line.lstrip().startswith("#"):
            key, _, value = line.partition(":")
            fields[key.strip().lower()] = value.strip().strip("\"'")
    return fields


def read_item(path: Path, central: Path, kind: str, archived: bool,
              node: str, last_commit: dict[str, str]) -> dict:
    try:
        raw = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        raw = ""
    head = "\n".join(raw.splitlines()[:40])  # metadata lives at the top of a file
    front = parse_frontmatter(raw)

    title_match = H1_RE.search(raw)
    title = clean(title_match.group(1), 120) if title_match else path.stem

    status = front.get("status") or front.get("statut") or ""
    if not status:
        for pattern in (STATUS_RE, PRIORITY_RE):
            found = pattern.search(head)
            if found:
                status = found.group(1)
                break

    created = front.get("created") or front.get("date") or ""
    if not created:
        found = CREATED_RE.search(head)
        created = found.group(1) if found else ""

    monday = MONDAY_ID_RE.match(path.name)
    rel = str(path.relative_to(central))
    dates = DATE_RE.findall(head)
    return {
        "node": node or "(home)",
        "kind": kind,
        "archived": archived,
        "slug": path.stem,
        "title": title,
        "status": clean(status) if status else "",
        "created": created,
        "monday": monday.group(1) if monday else "",
        "mentioned": max(dates) if dates else "",
        "commit": last_commit.get(rel, ""),
        "path": rel,
        "lines": raw.count("\n") + 1 if raw else 0,
    }


def git_last_commit_dates(central: Path) -> dict[str, str]:
    """Last commit date per file — in a clone, mtimes are the checkout's and lie."""
    try:
        out = subprocess.run(
            ["git", "-C", str(central), "log", "--no-merges",
             "--date=short", "--format=@%cd", "--name-only"],
            capture_output=True, text=True, timeout=60, check=True).stdout
    except (subprocess.SubprocessError, OSError):
        return {}
    dates: dict[str, str] = {}
    current = ""
    for line in out.splitlines():
        if line.startswith("@"):
            current = line[1:]
        elif line and current:
            dates.setdefault(line, current)  # log is reverse-chronological
    return dates


def collect(central: Path, links: dict[str, list[str]]) -> tuple[list[dict], list[dict]]:
    """Inventory the items and the nodes of the central clone."""
    tree = central / "tree"
    last_commit = git_last_commit_dates(central)
    items: list[dict] = []
    nodes: list[dict] = []

    candidates = [tree, *(p for p in tree.rglob("*") if p.is_dir())]
    for directory in sorted(candidates):
        # A node is a directory carrying an INDEX.md; the lifecycle and archive
        # directories are its content, never nodes themselves.
        parts = directory.relative_to(tree).parts
        if any(part in (*LIFECYCLE, ARCHIVES) for part in parts):
            continue
        if not (directory / "INDEX.md").is_file():
            continue

        node = str(directory.relative_to(tree)) if directory != tree else ""
        node_items = 0
        for kind in LIFECYCLE:
            for archived, base in ((False, directory / kind),
                                   (True, directory / ARCHIVES / kind)):
                if not base.is_dir():
                    continue
                for md in sorted(base.glob("*.md")):
                    items.append(read_item(md, central, kind, archived, node, last_commit))
                    node_items += 1
        # Flat `archives/`: observed in practice, sometimes with no kind subdirectory
        flat = directory / ARCHIVES
        if flat.is_dir():
            for md in sorted(flat.glob("*.md")):
                items.append(read_item(md, central, "?", True, node, last_commit))
                node_items += 1

        nodes.append({
            "node": node or "(home)",
            "depth": len(parts),
            "items": node_items,
            "materialized": links.get(node, []),
            "index": str((directory / "INDEX.md").relative_to(central)),
        })
    return items, nodes


def inventory(home: Path) -> dict:
    paths = scan_home(home)
    centrals: dict[Path, list[Path]] = {}
    for path in paths:
        central = resolve_central(path)
        if central:
            centrals.setdefault(central, []).append(path)
    if not centrals:
        return {"central": "no central clone found", "home": str(home),
                "discovered": len(paths), "materialized": 0, "items": [],
                "nodes": [], "warnings": [f"no central clone under {home}"]}

    items: list[dict] = []
    nodes: list[dict] = []
    warnings: list[str] = []
    links_total = 0
    for central, found in centrals.items():
        links, warned = materializations(found, central)
        links_total += len(links)
        warnings += warned
        got_items, got_nodes = collect(central, links)
        items += got_items
        nodes += got_nodes
    return {
        "central": " + ".join(str(c) for c in centrals),
        "home": str(home),
        "discovered": len(paths),
        "materialized": links_total,
        "warnings": warnings,
        "items": items,
        "nodes": nodes,
    }


def read_source(home: Path, rel: str) -> tuple[str, str] | None:
    """Raw markdown of one item, addressed by its path relative to a central clone.

    The path comes from the client, so it is resolved and then *proved* to sit
    inside a central clone discovered by this process — a prefix comparison on the
    resolved path, never on the requested string.
    """
    if not rel.endswith(".md"):
        return None
    for path in scan_home(home):
        central = resolve_central(path)
        if not central:
            continue
        try:
            target = (central / rel).resolve(strict=True)
            target.relative_to(central.resolve())
        except (OSError, ValueError):
            continue
        if target.is_file():
            return target.read_text(encoding="utf-8", errors="replace"), str(target)
    return None


# --------------------------------------------------------------------------- web

PAGE = r"""<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Ariane — items</title>
<style>
  :root {
    color-scheme: light dark;
    --bg: #fbfaf8; --panel: #fff; --fg: #1c1a17; --muted: #6b655c;
    --line: #e5e0d8; --accent: #9a5b2c; --chip: #f1ece4; --veil: rgba(28,26,23,.5);
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #161513; --panel: #1e1c1a; --fg: #ece7e0; --muted: #96908a;
      --line: #302d29; --accent: #d99a63; --chip: #2a2724; --veil: rgba(0,0,0,.66);
    }
  }
  * { box-sizing: border-box; }
  body {
    margin: 0; background: var(--bg); color: var(--fg);
    font: 14px/1.5 ui-sans-serif, -apple-system, "Segoe UI", sans-serif;
  }
  header {
    padding: 20px 24px 12px; border-bottom: 1px solid var(--line);
    position: sticky; top: 0; background: var(--bg); z-index: 3;
  }
  h1 { margin: 0 0 4px; font-size: 17px; letter-spacing: -.01em; }
  h1 span { color: var(--accent); }
  .sub { color: var(--muted); font-size: 12.5px; margin-bottom: 14px; }
  .sub code { background: var(--chip); padding: 1px 5px; border-radius: 4px; }
  .controls { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
  input[type=search], select {
    font: inherit; padding: 6px 10px; border: 1px solid var(--line);
    border-radius: 7px; background: var(--panel); color: var(--fg);
  }
  input[type=search] { min-width: 260px; flex: 1 1 260px; }
  label.check {
    display: inline-flex; gap: 6px; align-items: center; color: var(--muted);
    background: var(--panel); border: 1px solid var(--line);
    padding: 6px 10px; border-radius: 7px; cursor: pointer;
  }
  button {
    font: inherit; padding: 6px 12px; border-radius: 7px; cursor: pointer;
    border: 1px solid var(--line); background: var(--panel); color: var(--fg);
  }
  button:hover { border-color: var(--accent); color: var(--accent); }
  .count { color: var(--muted); font-size: 12.5px; margin-left: auto; }
  .wrap { overflow-x: auto; padding: 0 24px 40px; }
  table { border-collapse: collapse; width: 100%; min-width: 940px; }
  th, td {
    text-align: left; padding: 8px 10px; border-bottom: 1px solid var(--line);
    vertical-align: top;
  }
  thead th {
    position: sticky; top: 0; background: var(--bg); cursor: pointer;
    font-size: 11.5px; text-transform: uppercase; letter-spacing: .06em;
    color: var(--muted); font-weight: 600; white-space: nowrap; z-index: 2;
  }
  thead th:hover { color: var(--accent); }
  thead th[data-dir]::after { content: " ↑"; color: var(--accent); }
  thead th[data-dir="desc"]::after { content: " ↓"; }
  tbody tr:hover { background: var(--panel); }
  tr.arch { opacity: .5; }
  .t {
    cursor: pointer; text-align: left; font: inherit; font-weight: 500;
    background: none; border: 0; padding: 0; color: inherit;
  }
  .t:hover { color: var(--accent); text-decoration: underline; }
  .p { color: var(--muted); font-size: 11.5px; font-family: ui-monospace, monospace; }
  .k {
    font-size: 11px; padding: 2px 7px; border-radius: 20px;
    background: var(--chip); white-space: nowrap;
  }
  .k-tasks { color: #2f7d5c; } .k-stories { color: #3b6ea8; }
  .k-backlog { color: #8a6a1f; } .k-reflexions { color: #7a5aa8; }
  @media (prefers-color-scheme: dark) {
    .k-tasks { color: #6cc79b; } .k-stories { color: #7fb0e6; }
    .k-backlog { color: #d8b45c; } .k-reflexions { color: #b79ae0; }
  }
  td.n { font-family: ui-monospace, monospace; font-size: 12px; color: var(--muted); }
  td.d { white-space: nowrap; font-variant-numeric: tabular-nums; color: var(--muted); }
  .none { color: var(--line); }
  a { color: var(--accent); }
  .empty { padding: 40px 24px; color: var(--muted); }
  .warn {
    margin: 10px 0 0; padding: 9px 12px; border-radius: 7px; font-size: 12.5px;
    background: color-mix(in srgb, var(--accent) 12%, var(--panel));
    border: 1px solid color-mix(in srgb, var(--accent) 40%, var(--line));
  }
  .warn b { color: var(--accent); }
  .warn ul { margin: 5px 0 0; padding-left: 18px; }
  .warn li { font-family: ui-monospace, monospace; font-size: 11.5px; }

  /* ------------------------------------------------------------- item viewer */
  dialog#view {
    width: 100vw; max-width: 100vw; height: 100vh; max-height: 100vh;
    margin: 0; padding: 0; border: 0; background: var(--bg); color: var(--fg);
    display: none; flex-direction: column;
  }
  dialog#view[open] { display: flex; }
  dialog#view::backdrop { background: var(--veil); }
  .vhead {
    display: flex; gap: 12px; align-items: flex-start; padding: 16px 24px;
    border-bottom: 1px solid var(--line); background: var(--panel);
    position: sticky; top: 0;
  }
  .vhead h2 { margin: 0 0 3px; font-size: 15px; letter-spacing: -.01em; }
  .vhead .p { display: block; }
  .vnav { display: flex; gap: 6px; margin-left: auto; flex-shrink: 0; }
  .vbody { overflow-y: auto; flex: 1; padding: 28px 24px 64px; }
  .md {
    max-width: 74ch; margin: 0 auto; font-size: 14.5px; line-height: 1.62;
    overflow-wrap: break-word;
  }
  .md > :first-child { margin-top: 0; }
  .md h1 { font-size: 22px; margin: 0 0 18px; letter-spacing: -.02em; }
  .md h2 {
    font-size: 17px; margin: 32px 0 10px; padding-bottom: 5px;
    border-bottom: 1px solid var(--line);
  }
  .md h3 { font-size: 14.5px; margin: 24px 0 8px; }
  .md h4 { font-size: 13.5px; margin: 20px 0 6px; color: var(--muted); }
  .md p, .md ul, .md ol { margin: 0 0 14px; }
  .md li { margin: 3px 0; }
  .md li > input { margin-right: 6px; }
  .md blockquote {
    margin: 0 0 16px; padding: 2px 0 2px 14px; color: var(--muted);
    border-left: 3px solid var(--accent);
  }
  .md blockquote p:last-child { margin-bottom: 0; }
  .md code {
    background: var(--chip); padding: 1.5px 5px; border-radius: 4px;
    font-family: ui-monospace, monospace; font-size: .88em;
  }
  .md pre {
    background: var(--chip); padding: 12px 14px; border-radius: 8px;
    overflow-x: auto; margin: 0 0 16px; border: 1px solid var(--line);
  }
  .md pre code { background: none; padding: 0; font-size: 12.5px; line-height: 1.5; }
  .md hr { border: 0; border-top: 1px solid var(--line); margin: 26px 0; }
  .md .tw { overflow-x: auto; margin: 0 0 16px; }
  .md table { min-width: 0; width: auto; font-size: 13px; }
  .md th, .md td { padding: 6px 10px; }
  .md th { text-transform: none; letter-spacing: 0; position: static; }
  .md .fm {
    font-family: ui-monospace, monospace; font-size: 12px; color: var(--muted);
    background: var(--chip); border: 1px solid var(--line); border-radius: 7px;
    padding: 8px 12px; margin: 0 0 22px; white-space: pre-wrap;
  }
  .md .loading { color: var(--muted); }
</style>
<header>
  <h1>Ariane · <span>items</span></h1>
  <div class="sub" id="meta">loading…</div>
  <div id="warn"></div>
  <div class="controls">
    <input type="search" id="q" placeholder="Filter (title, node, status, path…)">
    <select id="kind"><option value="">All types</option></select>
    <select id="node"><option value="">All nodes</option></select>
    <label class="check"><input type="checkbox" id="arch"> Include archives</label>
    <button id="reload">Rescan</button>
    <span class="count" id="count"></span>
  </div>
</header>
<div class="wrap">
  <table>
    <thead><tr>
      <th data-k="node">Node</th>
      <th data-k="kind">Type</th>
      <th data-k="title">Item</th>
      <th data-k="status">Status (heuristic)</th>
      <th data-k="created">Created</th>
      <th data-k="commit">Last commit</th>
      <th data-k="lines">Lines</th>
    </tr></thead>
    <tbody id="rows"></tbody>
  </table>
  <div class="empty" id="empty" hidden>No item matches the filter.</div>
</div>

<dialog id="view">
  <div class="vhead">
    <div>
      <h2 id="vtitle"></h2>
      <span class="p" id="vpath"></span>
    </div>
    <div class="vnav">
      <button id="vprev" title="Previous item (←)">←</button>
      <button id="vnext" title="Next item (→)">→</button>
      <button id="vclose" title="Close (Esc)">Close</button>
    </div>
  </div>
  <div class="vbody"><div class="md" id="vbody"></div></div>
</dialog>

<script>
let ITEMS = [], NODES = [], VISIBLE = [], CURSOR = -1;
let SORT = { k: "commit", dir: "desc" };
const el = (id) => document.getElementById(id);
const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) =>
  ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

/* ----------------------------------------------------------------- inventory */

async function load() {
  el("meta").textContent = "scanning…";
  const data = await (await fetch("/api/items")).json();
  ITEMS = data.items; NODES = data.nodes;
  el("meta").innerHTML =
    `<code>${esc(data.central)}</code> — ${NODES.length} nodes, ${ITEMS.length} items · ` +
    `scan of <code>${esc(data.home)}</code>: ${data.discovered} <code>_ariane</code> path(s), ` +
    `${data.materialized} materialized node(s)`;
  const w = data.warnings ?? [];
  el("warn").innerHTML = w.length
    ? `<div class="warn"><b>${w.length} suspect materialization(s)</b> — an
       <code>_ariane</code> path must resolve inside <code>tree/</code> onto a
       directory carrying an <code>INDEX.md</code>. This is
       <code>ariane doctor</code>'s territory:
       <ul>${w.map((x) => `<li>${esc(x)}</li>`).join("")}</ul></div>`
    : "";
  fill("kind", [...new Set(ITEMS.map((i) => i.kind))].sort());
  fill("node", [...new Set(ITEMS.map((i) => i.node))].sort());
  render();
}

function fill(id, values) {
  const sel = el(id), keep = sel.value;
  sel.length = 1;
  for (const v of values) sel.add(new Option(v, v));
  sel.value = values.includes(keep) ? keep : "";
}

function render() {
  const q = el("q").value.toLowerCase().trim();
  const kind = el("kind").value, node = el("node").value, arch = el("arch").checked;
  VISIBLE = ITEMS.filter((i) =>
    (arch || !i.archived) && (!kind || i.kind === kind) && (!node || i.node === node) &&
    (!q || [i.title, i.node, i.status, i.path, i.slug, i.monday]
      .join(" ").toLowerCase().includes(q)));

  const { k, dir } = SORT, mul = dir === "desc" ? -1 : 1;
  VISIBLE.sort((a, b) => {
    const [x, y] = [a[k], b[k]];
    if (typeof x === "number") return (x - y) * mul;
    // Empty cells always sink to the bottom, whichever way the column is sorted.
    if (!x !== !y) return !x ? 1 : -1;
    return String(x).localeCompare(String(y), "fr") * mul;
  });

  const blank = VISIBLE.filter((i) => !i.status).length;
  el("count").textContent =
    `${VISIBLE.length} / ${ITEMS.length} items` +
    (arch ? "" : ` · ${ITEMS.filter((i) => i.archived).length} archived hidden`) +
    (blank ? ` · ${blank} with no readable status` : "");
  el("empty").hidden = VISIBLE.length > 0;
  el("rows").innerHTML = VISIBLE.map((i, n) => `<tr class="${i.archived ? "arch" : ""}">
    <td class="n">${esc(i.node)}</td>
    <td><span class="k k-${esc(i.kind)}">${esc(i.kind)}${i.archived ? " · arch." : ""}</span></td>
    <td><button class="t" data-n="${n}">${esc(i.title)}</button>
        <div class="p">${esc(i.path)}</div></td>
    <td>${i.status ? esc(i.status) : '<span class="none">—</span>'}</td>
    <td class="d">${i.created ? esc(i.created) : '<span class="none">—</span>'}</td>
    <td class="d">${i.commit ? esc(i.commit) : '<span class="none">—</span>'}</td>
    <td class="d">${i.lines}</td></tr>`).join("");
  for (const b of document.querySelectorAll("button.t"))
    b.onclick = () => openItem(+b.dataset.n);

  for (const th of document.querySelectorAll("thead th"))
    th.dataset.k === k ? (th.dataset.dir = dir) : delete th.dataset.dir;
}

/* -------------------------------------------------------------- item viewer */

async function openItem(n) {
  if (n < 0 || n >= VISIBLE.length) return;
  CURSOR = n;
  const item = VISIBLE[n];
  el("vtitle").textContent = item.title;
  el("vpath").textContent = `${item.path}  ·  ${n + 1} / ${VISIBLE.length}`;
  el("vbody").innerHTML = '<p class="loading">reading…</p>';
  el("vprev").disabled = n === 0;
  el("vnext").disabled = n === VISIBLE.length - 1;
  if (!el("view").open) el("view").showModal();
  document.querySelector(".vbody").scrollTop = 0;

  const res = await fetch("/api/source?path=" + encodeURIComponent(item.path));
  if (CURSOR !== n) return;  // a faster click already moved on
  el("vbody").innerHTML = res.ok
    ? md(await res.text())
    : `<p class="loading">unreadable: ${esc(item.path)}</p>`;
}

el("vclose").onclick = () => el("view").close();
el("vprev").onclick = () => openItem(CURSOR - 1);
el("vnext").onclick = () => openItem(CURSOR + 1);
el("view").onclick = (e) => { if (e.target === el("view")) el("view").close(); };
document.addEventListener("keydown", (e) => {
  if (!el("view").open) return;
  if (e.key === "ArrowLeft") openItem(CURSOR - 1);
  if (e.key === "ArrowRight") openItem(CURSOR + 1);
});

/* ------------------------------------------------------------------ markdown
   Just enough of it for item files: front-matter, headings, fenced code,
   tables, quotes, lists with checkboxes, rules, and inline spans. Everything is
   HTML-escaped first, so file content can never inject markup. */

function md(src) {
  const fences = [];
  let out = esc(src.replace(/\r\n/g, "\n"));

  // Front-matter, shown as the metadata block it is rather than as a rule.
  let fm = "";
  out = out.replace(/^---\n([\s\S]*?)\n---\n/, (_, body) => {
    fm = `<div class="fm">${body.trim()}</div>`;
    return "";
  });

  // Fenced code is parked before anything else can touch its content.
  out = out.replace(/```[^\n]*\n([\s\S]*?)```/g, (_, code) =>
    `\ue000${fences.push(`<pre><code>${code.replace(/\n$/, "")}</code></pre>`) - 1}\ue001`);

  const inline = (s) => s
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[\s(])\*([^*\n]+)\*/g, "$1<em>$2</em>")
    .replace(/~~([^~]+)~~/g, "<del>$1</del>")
    .replace(/\[([^\]]+)\]\(([^)\s]+)[^)]*\)/g,
      '<a href="$2" target="_blank" rel="noreferrer">$1</a>');

  const blocks = [];
  const lines = out.split("\n");
  let i = 0;

  const isRow = (s) => /^\s*\|.*\|\s*$/.test(s);
  const cells = (s) => s.trim().replace(/^\||\|$/g, "").split("|").map((c) => c.trim());

  while (i < lines.length) {
    const line = lines[i];

    if (!line.trim()) { i++; continue; }

    if (/^\ue000\d+\ue001$/.test(line.trim())) { blocks.push(line.trim()); i++; continue; }

    if (/^(-{3,}|\*{3,}|_{3,})\s*$/.test(line)) { blocks.push("<hr>"); i++; continue; }

    const head = line.match(/^(#{1,6})\s+(.*)$/);
    if (head) {
      const n = head[1].length;
      blocks.push(`<h${n}>${inline(head[2])}</h${n}>`);
      i++; continue;
    }

    // Table: a row, a delimiter row, then rows until something else.
    if (isRow(line) && i + 1 < lines.length && /^\s*\|[\s:|-]+\|\s*$/.test(lines[i + 1])) {
      const header = cells(line);
      i += 2;
      const body = [];
      while (i < lines.length && isRow(lines[i])) body.push(cells(lines[i++]));
      blocks.push(`<div class="tw"><table><thead><tr>${
        header.map((c) => `<th>${inline(c)}</th>`).join("")}</tr></thead><tbody>${
        body.map((r) => `<tr>${r.map((c) => `<td>${inline(c)}</td>`).join("")}</tr>`)
          .join("")}</tbody></table></div>`);
      continue;
    }

    // `>` is already `&gt;` here: escaping happens before parsing, on purpose.
    if (/^\s*&gt;/.test(line)) {
      const buf = [];
      while (i < lines.length && /^\s*&gt;/.test(lines[i]))
        buf.push(lines[i++].replace(/^\s*&gt;\s?/, ""));
      blocks.push(`<blockquote>${mdParagraphs(buf, inline)}</blockquote>`);
      continue;
    }

    const bullet = /^\s*([-*+]|\d+[.)])\s+/;
    if (bullet.test(line)) {
      const ordered = /^\s*\d/.test(line);
      const buf = [];
      while (i < lines.length && bullet.test(lines[i])) {
        let text = lines[i++].replace(bullet, "");
        // Continuation lines: indented, and not the start of another item.
        while (i < lines.length && /^\s{2,}\S/.test(lines[i]) && !bullet.test(lines[i]))
          text += " " + lines[i++].trim();
        const box = text.match(/^\[([ xX])\]\s*(.*)$/);
        buf.push(box
          ? `<li><input type="checkbox" disabled${
              box[1] === " " ? "" : " checked"}>${inline(box[2])}</li>`
          : `<li>${inline(text)}</li>`);
      }
      blocks.push(`<${ordered ? "ol" : "ul"}>${buf.join("")}</${ordered ? "ol" : "ul"}>`);
      continue;
    }

    const buf = [];
    while (i < lines.length && lines[i].trim() && !/^\s*&gt;/.test(lines[i]) &&
           !bullet.test(lines[i]) && !/^#{1,6}\s/.test(lines[i]) && !isRow(lines[i]) &&
           !/^\ue000\d+\ue001$/.test(lines[i].trim()))
      buf.push(lines[i++]);
    if (buf.length) blocks.push(`<p>${inline(buf.join(" "))}</p>`);
    else i++;
  }

  return (fm + blocks.join("\n"))
    .replace(/\ue000(\d+)\ue001/g, (_, n) => fences[+n]);
}

function mdParagraphs(lines, inline) {
  return lines.join("\n").split(/\n{2,}/).filter((p) => p.trim())
    .map((p) => `<p>${inline(p.replace(/\n/g, " "))}</p>`).join("");
}

/* ------------------------------------------------------------------- wiring */

for (const th of document.querySelectorAll("thead th"))
  th.onclick = () => {
    const k = th.dataset.k;
    SORT = { k, dir: SORT.k === k && SORT.dir === "asc" ? "desc" : "asc" };
    render();
  };
for (const id of ["q", "kind", "node", "arch"]) el(id).oninput = render;
el("reload").onclick = load;
load();
</script>
"""


class Handler(http.server.BaseHTTPRequestHandler):
    home: Path
    protocol_version = "HTTP/1.1"

    def do_GET(self) -> None:  # noqa: N802
        route, _, query = self.path.partition("?")
        if route == "/api/items":
            self.send_payload(json.dumps(inventory(self.home)).encode(),
                              "application/json; charset=utf-8")
        elif route == "/api/source":
            rel = urllib.parse.parse_qs(query).get("path", [""])[0]
            found = read_source(self.home, rel)
            if found is None:
                self.send_error(404)
            else:
                self.send_payload(found[0].encode(), "text/plain; charset=utf-8")
        elif route in ("/", "/index.html"):
            self.send_payload(PAGE.encode(), "text/html; charset=utf-8")
        else:
            self.send_error(404)

    def send_payload(self, body: bytes, ctype: str) -> None:
        self.send_response(200)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args) -> None:
        pass  # the server is ephemeral; nobody wants its access log


class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


def main() -> int:
    home = Path(sys.argv[1]).expanduser() if len(sys.argv) > 1 else Path.home()
    port = int(os.environ.get("ARIANE_PORT", "0"))

    data = inventory(home)
    if not data["items"]:
        print(f"No item found under {home} (central: {data['central']}).",
              file=sys.stderr)
    else:
        live = sum(1 for i in data["items"] if not i["archived"])
        print(f"{data['central']}\n"
              f"{len(data['nodes'])} nodes · {len(data['items'])} items "
              f"({live} live, {len(data['items']) - live} archived)")
    for warning in data["warnings"]:
        print(f"  ! {warning}", file=sys.stderr)

    Handler.home = home
    with Server(("127.0.0.1", port), Handler) as server:
        url = f"http://127.0.0.1:{server.server_address[1]}"
        print(f"→ {url}   (Ctrl-C to stop)", flush=True)
        if os.environ.get("ARIANE_NO_BROWSER") != "1":
            webbrowser.open(url)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            print("\nstopped.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

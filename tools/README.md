# `tools/` — throwaway tooling

Scripts kept here are **temporary by contract**. They exist because the `ariane`
CLI (PLAN §10, roadmap step 7) does not exist yet, and each one is expected to be
**deleted** once the command it stands in for is implemented. Nothing in the
method depends on them; nothing here is a published interface.

Rules for this directory:

- One file, no dependencies beyond the language's standard library. A tool that
  needs a lockfile belongs in the CLI, not here.
- The header of each script names the command that will replace it.
- Read-only on the user's content. These are observation tools; the writing side
  of the method is the agent's job and the CLI's.

## `ariane-tasks.py` — cross-cutting view of the items

Stands in for a future **`ariane tasks`**. Lists every item of the tree in one
sortable table, served on a local ephemeral HTTP server.

```sh
python3 tools/ariane-tasks.py              # scan $HOME
python3 tools/ariane-tasks.py ~/workspace  # scan a given root
ARIANE_PORT=8765 ARIANE_NO_BROWSER=1 python3 tools/ariane-tasks.py
```

It scans for `_ariane` paths, walks **up** from any of them to the central
content repository — the ancestor carrying both `tree/` and `.git` — and
inventories that repository. This matters: the central clone is the complete
source, *including nodes that are not materialized anywhere*, whereas a scan of
the working directories only ever sees what happens to be linked. The discovered
links are then used the other way round, to annotate **where** each node is
materialized.

Dates come from `git log`, not from `mtime`: in a clone, mtimes are those of the
checkout and mean nothing.

### Two things it reports beyond the table

**Suspect materializations.** A `_ariane` path must resolve inside `tree/` onto a
directory carrying an `INDEX.md`. Anything else is deployment drift, shown in a
banner. This overlaps `ariane doctor` (ADR-0004) by accident, not by design —
`doctor` is the real answer, and it is read-only there too.

**Items whose status is not machine-readable.** The count is displayed next to the
filters. It is not a defect of the parser: item files are not normalized yet
(`SPEC.md` and `templates/` are roadmap steps 3 and 4), so status lives in free
prose — `> Créée : … — statut : …` in one file, `- **Priorité** : …` in another,
nothing at all in a third. The parser covers those two shapes plus a YAML
front-matter. **An empty cell means the file does not say it in a recognizable
form, not that the item has no status.** That number is the argument for
normalizing the item header, measured rather than asserted.

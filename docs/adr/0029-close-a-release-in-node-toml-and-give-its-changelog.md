# ADR-0029 — Close a release in `node.toml`, and give its changelog entry

- **Status:** Accepted
- **Date:** 2026-11-14
- **Concerns:** SPEC §1.5, §3; story `slashome/ariane#S039`, `slashome/ariane#S043`; [ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md) §6, [ADR-0027](0027-record-when-an-item-closes.md), [ADR-0028](0028-compute-the-forecast-in-one-place.md)

## Context

[ADR-0023](0023-plan-by-releases-declared-in-order-per-node.md) §6 names `ariane release close` and says that whether a release is closed "is read from the tasks whose effective release it is: none open, none left outside `archives/`". Implementing it (task `T143`) shows three gaps:

- **That rule cannot tell a closed release from an empty one.** A release declared yesterday with no work yet is "closed" by it, and so is the release `0.1` of a node whose finished tasks were moved by hand. `pharmakon` has the case today: its `atelier/build.py` archives the tasks of a release as soon as the release's "Jouer et juger" task is validated (`closed_releases` in `atelier/build.py`), without writing anything that says so.
- **A closed release must refuse new work.** Planning a task in a release already delivered is a mistake `release assign` should catch, and it can only catch it if "closed" is written somewhere.
- **The forecast has to draw it as delivered**, at the day it closed, not at the last closing date of its tasks.

Story S043 adds that closing a release gives its changelog entry, in a format shared by every node and modelled on the `CHANGELOG.md` of `choosemycompany/cmc-ui`, with headings in the language `[changelog] language` gives, inherited; the skill rewrites the entry for the reader `[changelog] audience` names.

Archiving moves files, and a moved file breaks the relative links to it and its own relative links to files that stay. `ariane fix` already rewrites the first kind; `build.py` already rebases the second (`rebase_links`).

## Decision

> **`ariane release close <node> <name>` writes `closed = YYYY-MM-DD` on the release's `[[release]]` entry, after archiving its finished work. A closed release takes no more work. Closing prints the release's raw changelog entry and writes it nowhere.**

1. **The key.** `closed`, a date, MAY on a `[[release]]` entry; a parking release MUST NOT carry it. It replaces the derived rule of ADR-0023 §6: a release is closed when its entry says so.
2. **Refusals.** Closing refuses an unknown, parking or already closed release, a node that inherits its list, and, unless asked to carry it over, a release with open work: a task, or a story or an epic not split yet, whose effective release it is and whose status is neither `done` nor `dropped`. The refusal lists them.
3. **Carry-over.** `--carry-over` (the CLI flag that replaces ADR-0023's `--move-open-to`) writes `release` on each open item itself, even when its release came from its story or its epic, with the next release of the list that is neither parking nor closed; it refuses when there is none. The slip is one header change per item, visible in git.
4. **Archiving.** The finished work of the release (`done` and `dropped`) moves under `archives/tasks/`, `archives/stories/` or `archives/epics/`. A story or an epic whose effective release is the release, or one of whose children is archived by the close, follows when its own status is closed and nothing of it is left outside `archives/`; otherwise it stays, and the plan says why. An epic spanning two releases therefore stays until its last release closes. Links are rewritten with the code of `ariane fix`: references to a moved file, and the relative link targets of a moved file to files that stay.
5. **A closed release takes no more work.** `release assign`, and `release` in `PATCH /api/items`, refuse it; `release set --parking` refuses it; `doctor` warns on open work whose effective release is closed (`open_in_closed_release`).
6. **The forecast.** A closed release keeps its place in the order and gives its `closed` date; its finished work is shown, nothing is scheduled in it, and open work left in it is unscheduled with the reason `closed_release`.
7. **The changelog entry.** `## <name>`, then the sections that have lines: `Added` (`Nouveautés`), then `Fixed` (`Corrections`) for work whose `type` is `bug` or `fix`. One line per subject, in bold: a task's story, else its epic, else the work itself; the subject of a story or an epic not split yet is itself. The work of a subject is merged on its line, its ids at the end. The entry is computed from the finished work whose effective release is the release, archived or not, so `ariane release changelog <node> <name>` prints it again any time. `[changelog] language` (`en` by default, or `fr`) and `audience` (`po`, `designer` or `user`, read by the skill only) are inherited key by key.
8. **Nothing is committed or written to the host repository**, as for every write of the CLI. `--dry-run` writes nothing and reports the open work instead of refusing.

## Consequences

- "Closed" is one line in `node.toml`, read by `doctor`, the forecast and the interface without walking the tasks.
- `pharmakon`'s `build.py` stops archiving (task `pharmakon#T254`): it only writes statuses and `closed`, and closing a release becomes an explicit command.
- A release closed by hand before this change carries no `closed` until someone writes it; nothing back-fills it.
- A story or an epic whose work is all archived but whose own status was never closed stays outside `archives/`. `pharmakon`'s blocks, kept `a-faire`, are in that case: the plan names them, a human closes them.
- No `type` names a removal yet, so the `Removed` (`Retraits`) section of the shared format is never filled by the CLI. The skill can add it; a vocabulary for it is a later decision.
- The subject is the story, not the epic: lines are short and many. The skill regroups them for its reader.

## Alternatives considered

### Keep deriving "closed" from the tasks

No new key. Rejected for the reasons of the context: an empty release looks closed, the moment of closing is lost, and nothing can refuse new work in a release no file calls closed.

### Mark closed on a "release review" task

`pharmakon` closes a release when its "Jouer et juger" task is validated. Rejected as a rule of Ariane: it is one node's convention, and it ties a release to a task title. The node can still run `ariane release close` when that task is validated.

### Use the epic as the subject of a line

The sample of S043 reads like one (`Page entreprise`). Rejected for the raw entry: an epic spans releases and groups unrelated stories, and the work of a release often has no epic at all. The story is the nearest unit a reader recognises; the skill merges lines under their epic when its reader needs fewer.

### Write the entry into the host repository's `CHANGELOG.md`

One command less. Rejected by S043: Ariane never writes outside the content repository, and the entry belongs in the release's pull request, reviewed with it.

## Revisit if

- Work starts landing in closed releases routinely (a hotfix line), which would call for reopening a release or for patch releases.
- A node needs the `Removed` section from the CLI: then a `type` vocabulary for removals is decided.
- Readers find story-level lines too fragmented even in the raw entry.

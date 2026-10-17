---
name: ariane-session
description: Updates the registry of work threads (sessions/) of an Ariane node from the current conversation — files what was settled, opens a thread when a topic has none, flags the threads whose archiving condition is met. Invoke at the end of a session, or when a conversation has clearly drifted across several topics.
---

# ariane-session

A node's `sessions/` directory answers a question no other one addresses:
**"in which conversation do I pick this up?"**

A thread is a **lineage of conversations**, not a topic. It has a boundary, a
state, and an archiving condition. This skill maintains it.

## What this directory is not

Copy **nothing** here. The node already has three places, and they keep their role:

| | Answers |
|---|---|
| `reflexions/` | *what do we decide on this topic?* |
| `tasks/`, `stories/` | *what do we do?* |
| `HANDOFF.md` | *where are we right now?* |

A thread file **points** to them and says what remains open. If you catch
yourself explaining reasoning in `sessions/`, it belongs in `reflexions/`.

## Procedure

### 1. Locate

Read the node's `sessions/INDEX.md`. If it does not exist, go to step 5.

Determine which thread what was just done belongs to. **A conversation can touch
several** — that is the normal case, and it is exactly what must be recorded
separately rather than as a single summary.

### 2. File what was settled

For each thread touched, in its file:

- move to **"Settled"** what was settled, in one line, with its reason. Not the
  reasoning — the reason;
- remove from **"Still open"** what no longer is;
- add what opened during the session;
- update the state in the header **and** in the `INDEX.md` table.

**A decision that was costly to reach deserves an ADR or a reflection, not a line
in a thread file.** Write it there, and point the thread file to it.

### 3. Check archiving conditions

Every thread file carries a condition in its last section. It must be
**verifiable**: "the six screens exist", not "the design is done".

For every thread, including those the session did not touch: is the condition
met? If so, **do not archive it yourself** — report it to the owner with the
evidence. Archiving is a decision.

Once decided: the file moves to the node's `archives/sessions/`, and its
`INDEX.md` line follows with its date. **Nothing is deleted.**

### 4. Spot a thread that should have existed

If the session dealt with a topic that belongs to no thread, that signals a
missing thread. **There is a single criterion:**

> A thread deserves to exist when **it can move forward without the others**.

It is verifiable. If opening a topic forces another thread to reopen, the two are
one — do not create a second. And a one-way dependency is not a merge: a screen
needs a model, a model never needs a screen.

Propose the thread; do not create it without agreement.

### 5. Create the directory if it does not exist

`sessions/INDEX.md` with the table of threads and the separation rule, then one
file per thread. The thread file template:

```markdown
---
node: <path/of/the/node>
---

# <Thread name>

> <state + date>. <one sentence saying why it is in that state>

## What it covers
## Where the work lives
## Settled
## Still open
## Archiving condition
```

## Rules

- **Invent nothing.** What was not settled stays "open". A session that decided
  nothing produces no "settled" line.
- **Write down blockers, not only progress.** A thread blocked on an action by the
  owner must say so, and say which action.
- **Flag what was not verified.** A decision made on reasoning rather than on a
  measurement is marked as such.
- **Keep the file short.** Fifty lines. Beyond that, it explains instead of
  pointing.
- English. Identifiers and paths stay as they are.

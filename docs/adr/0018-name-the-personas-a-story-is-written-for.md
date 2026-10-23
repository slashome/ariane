# ADR-0018 — Name the personas a story is written for

- **Status:** Accepted
- **Date:** 2026-10-23
- **Concerns:** SPEC §1.3, §1.5, §1.6; [ADR-0008](0008-give-every-item-a-versioned-front-matter.md), [ADR-0015](0015-name-whom-an-item-waits-for.md), [ADR-0017](0017-keep-a-nodes-judgment-in-its-items.md)

## Context

A user story is written for someone: *as a CMC developer*, *as an HR manager*. Today that someone is a phrase in the story's body, spelled differently from one story to the next, so nothing can list the stories of one persona, and a persona renamed in one story keeps its old name in the others.

The `cmc` nodes serve several audiences at once — the company's own developers, its clients' HR teams, their employees — and the same personas recur across their repositories. With `INDEX.md` gone (ADR-0017), the node has no other place to say who its stories are for.

## Decision

> **A node declares its personas in `node.toml`, and a story names the ones it is written for by id.**

- **A sixth table, `[personas."<id>"]`,** with one key, `name`, the readable label. An id reads `<group>:<role>` — `cmc:dev`, `user:rh` — so it is a quoted TOML key. Personas are inherited and accumulate like `[people]` (ADR-0015): declared once at `cmc`, they apply to every repository below it, and a nearer declaration wins.
- **A core key, `personas`,** a list of ids, meant for stories. `doctor` reports an id declared under no applicable `[personas]` as an error, and `personas` on any other kind as a warning: the key is not forbidden there, but it is out of place.
- **Exposed where it costs nothing:** in the rows of `ariane tasks --json` and `ariane show --json`, in `/api/nodes` with the effective personas of each node, and on the story page of the interface, by name.

## Consequences

- Stories of one persona can be listed and filtered by id; a persona's label changes in one place.
- A new requirement level, `should_not`, appears in `ariane schema --json` for the kinds a key is not meant for; consumers that switch over requirement values must know it.
- The interface does not edit `personas` yet; the story's header is changed by hand or by the agent.
- A persona id is not validated beyond being declared: `doctor` does not enforce the `<group>:<role>` form, which stays a convention.

## Alternatives considered

### Personas as a custom field

`[fields.personas]` of type `enum` in each node. Rejected: an enum lists bare values with no label, so the readable name would live nowhere; and a node adding one persona would redeclare the whole list, since a redeclared field replaces the inherited declaration instead of extending it.

### Reuse `[people]`

A persona is a role, not a person: it has no tracker id, nobody waits for it, and `ariane people` would list it among the people to chase. Its ids carry a colon, which a handle does not allow (`^[a-z][a-z0-9-]*$`).

### Forbid `personas` outside stories

An error on a task carrying it. Rejected: a task promoted from a story, or a ressource describing a persona's journey, may legitimately keep it while the owner decides; a warning flags it without blocking `doctor`.

## Revisit if

- Personas need more than a label — goals, a description — at which point they are better as ressources of the node, referenced by id.
- Tasks routinely need personas too, which would make the key general rather than meant for stories.

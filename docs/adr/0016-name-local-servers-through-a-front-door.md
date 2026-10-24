# ADR-0016 — Name local servers through a front door in `ariane`

- **Status:** Accepted
- **Date:** 2026-10-23
- **Concerns:** SPEC §1.5, §1.6; [ADR-0004](0004-write-or-judge.md), [ADR-0012](0012-serve-the-interface-from-the-binary.md), [ADR-0013](0013-define-the-configuration-schema-in-code.md)

## Context

The owner wants to type `ariane.local` instead of `http://127.0.0.1:4317/…`, and the same for the development server of any project of the tree: `dreamcatchr.local` rather than `localhost:3000`. The `cmc` stack already does this for its own sites, with an nginx container in front, and that configuration belongs to `cmc`.

What stands in the way, verified on the owner's machine:

- **`/etc/hosts` maps a name to an address, never to a port or a path.** `ariane.local` in it reaches port 80 of that address, nothing else.
- **Port 80 is taken, on every interface, by `cmc`'s reverse proxy** — `lsof -nP -iTCP -sTCP:LISTEN` shows `com.docker.backend *:80` and `*:443`, the published ports of the container `choosemycompany_nginx_reverse_proxy` (`docker ps`).
- **Docker is not root.** `lsof` without `sudo` lists only the user's processes, and it lists Docker's. Since macOS 10.14, an unprivileged process may bind a port below 1024 **only on the wildcard address**: `python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 80))'` fails with `[Errno 13] Permission denied`, with or without `SO_REUSEADDR`. Ariane cannot share port 80 the way Docker holds it, and ADR-0012 forbids it to listen beyond loopback anyway.
- **The stock `/etc/pf.conf` loads `rdr-anchor "com.apple/*"`** (`grep anchor /etc/pf.conf`), so a redirect can be loaded into an anchor of its own, `com.apple/ariane`, without editing a system file.
- **4317, the default port of ADR-0012, is OpenTelemetry's OTLP/gRPC port** (4318 is OTLP/HTTP): any local collector or agent with OTLP enabled collides with it.

## Decision

> **`ariane` routes names to local ports through a front door it serves itself; the one root step — an alias, a redirect and the names — is a command the owner runs, which lasts until the next reboot.**

**The chain, from the browser inward.**

1. **The names.** `/etc/hosts` sends each routed name to **`127.0.0.2`**, a loopback alias only Ariane uses, so its port 80 can be redirected without touching `127.0.0.1` or Docker's wildcard binding. Ariane's lines sit between two marker lines; nothing else in the file is touched.
2. **The redirect.** A `pf` rule in the anchor `com.apple/ariane`: `rdr pass on lo0 inet proto tcp from any to 127.0.0.2 port 80 -> 127.0.0.1 port <proxy.port>`.
3. **The front door.** An HTTP server on `127.0.0.1:<proxy.port>`, default **27380**, that reads each request's `Host` and forwards it to the route's port on `127.0.0.1`. Like nginx's `proxy_pass`, the upstream receives `Host: localhost:<port>` — which development servers accept without configuration — and the typed name in `X-Forwarded-Host`; a `Location` pointing at the upstream's address is rewritten back to the name. A `101 Switching Protocols` joins both connections, so a dev server's reload WebSocket works.

**The routes.**

- **The interface's own:** `server.host` of `config.toml`, default `ariane.local`, to `server.port`.
- **One per node** whose `node.toml` has a `[proxy]` table: `host` (a lowercase host name) and `port`. It is not inherited. A name already taken keeps its first owner; the other is left out and reported, and `doctor` reports invalid and duplicated `[proxy]` tables (ADR-0004: the command that judges stays `doctor`).

**The processes.**

- **`ariane serve` opens the front door** next to the interface, and its routes follow the tree as the watcher reloads it. When the port is taken, it says so and keeps serving the interface.
- **`ariane proxy [node…]` opens it alone,** for when the server does not run: every route, or only those of the nodes named by path or last segment. It refuses to start while `ariane serve` holds the port, since the server already forwards every route.

**The root step.**

- **`ariane proxy setup`** runs, through `sudo`, each printed before it runs: the alias, the anchor, `pfctl -E`, the copy of the new `/etc/hosts` (staged next to `config.toml`), and the flush of the resolver's cache. Nothing persists across a reboot, by choice for now.
- **`ariane proxy teardown`** undoes it; **`ariane proxy status`** checks each link without root — the alias, a probe of the front door and of `127.0.0.2:80` (the front door marks its own answers with `x-ariane-door`, which is how a probe tells it from another server), each name in `/etc/hosts`, each route's port — and exits with `1` on a broken one.

**The default port moves** from 4317 to **27317**: below the ephemeral range of macOS (49152–65535), and the default of no tool the owner runs.

> **Amended 2026-10-24: the interface shows the chain and edits the routes.** The checks of `ariane proxy status` are computed once, as data — each a verdict, what it checks and the remedy — which the command prints and `GET /api/proxy` serves, with each route (host, port, owning node or `null` for the interface, whether its name is in `/etc/hosts` and its port listens), the ports of the front door and of the interface, and the `[proxy]` tables left out. The probes are the same blocking connections, run on the server's blocking pool. A **Proxy** page shows them, checked on load and on *Re-check*, and the sidebar's footer entry carries a dot, green when every link holds, refreshed every 30 s. The page writes a node's route: `PUT /api/proxy/routes` with `{ node, host, port }` sets the `[proxy]` table of its `node.toml` (edited in place, comments and other tables kept; created holding only `[proxy]` when the node has none), `DELETE` with `{ node }` removes it (and the file, when nothing else is left). A host must be a host name no other route has, a port must be 1–65535 and neither `proxy.port` nor `server.port`; a refusal is a `422` with its reason. The write leaves the content repository dirty, for the owner to commit like any other change, and reloads the tree, which notifies the open interfaces. The interface's own route stays `server.host` of `config.toml`, read-only there. **The root step stays a command:** the page shows `ariane proxy setup` and `teardown` to copy, never runs them, and reminds that a new name needs `setup` again before `/etc/hosts` knows it (ADR-0012 keeps the server from holding root).

> **Amended 2026-10-24: an IPv6 half of the chain, and a front-door check that trusts the redirect.** Measured after `setup`: `curl -4 http://ariane.local/` resolves in 12 ms, but `curl http://ariane.local/` with IPv6 allowed spends **5.01 s** in name lookup before the same 200. `.local` is the mDNS domain, and with no IPv6 line in `/etc/hosts` the resolver waits for an AAAA answer that never comes. The chain now carries both families:
>
> - **A second loopback alias, `fd61:7269:616e:6500::2`**, the constant `ALIAS_V6`. It is a unique local address (`fd00::/8`, RFC 4193), never routed beyond the machine and claimed with `prefixlen 128`, so no other address of that range is touched. `::1` is out, since Docker holds `[::1]:80`; a link-local `fe80::` address needs a zone index (`%lo0`) that `/etc/hosts` and browsers handle poorly; `::2` sits in the deprecated IPv4-compatible range. Its 40-bit global ID is not pseudo-random as RFC 4193 asks: the bytes after `fd` spell `ariane` in ASCII and the interface ID `::2` echoes `127.0.0.2`, a mnemonic that costs nothing on an address that never leaves `lo0`; a network handing out the same `/48` is a 2⁻⁴⁰ chance.
> - **`setup`** also runs `sudo ifconfig lo0 inet6 fd61:7269:616e:6500::2 prefixlen 128 alias`, the anchor holds a second rule, `rdr pass on lo0 inet6 proto tcp from any to fd61:7269:616e:6500::2 port 80 -> ::1 port <proxy.port>`, and the `/etc/hosts` block gives each name a `fd61:7269:616e:6500::2` line next to its `127.0.0.2` one. **`teardown`** also runs `sudo ifconfig lo0 inet6 fd61:7269:616e:6500::2 -alias`.
> - **The front door listens on `[::1]:<proxy.port>` too**, since a `pf` `rdr` cannot translate between address families. Both binds are required; `[::1]:27380` is free, Docker holding only port 80.
> - **`status`** gains the IPv6 alias on `lo0`, the probe of `[fd61:7269:616e:6500::2]:80`, and each name's IPv6 line in `/etc/hosts`; a route counts as `declared` only with both lines.
>
> **The front-door check.** Once `pf` is enabled with the anchor loaded, a direct connection to `127.0.0.1:<proxy.port>` times out although the process listens there, while `127.0.0.2:80`, redirected by `pf`, reaches the same process and gets its answer. Before `setup`, the direct probe succeeded. Measured again on the owner's machine: `127.0.0.1:27317`, the interface on the same address, answers in 2 ms; `127.0.0.1:27380` times out, also when the connection is bound to `127.0.0.2` as its source; `127.0.0.2:80` answers in 5 ms. Only the redirect's target port is affected. **The cause is observed, not explained:** `pf.conf(5)` does not describe it, and inspecting the state table (`pfctl -ss`) needs root, which was not done; a collision with the states the `rdr pass` rule creates on `lo0` is a suspicion only. The check named `front door on 127.0.0.1:<port> or [::1]:<port>` therefore holds when either redirect answers with the front door's mark, or else when a direct probe of either loopback address does; the hint *is `ariane serve` or `ariane proxy` running?* appears only when none answers. The two redirects keep their own lines.

## Consequences

- Any project of the tree gets a name with two lines of `node.toml` and one `ariane proxy setup`. The routes live with the projects, versioned with the content.
- The redirect and the alias vanish at reboot: until a LaunchDaemon makes them persistent, `ariane proxy setup` is run again after each one, and `status` is what tells.
- `/etc/hosts` is rewritten by `sudo cp` of a staged copy: an edit made to it between the read and the copy is lost. It is a window of seconds, on a file edited by hand.
- **`.local` is the mDNS domain.** A name declared only in IPv4 makes the resolver wait for an IPv6 answer; the `cmc` entries add a `::1` line for that reason, which Ariane cannot do — `::1` port 80 is Docker's. Measured at 5 s, it is why every name also has an IPv6 alias and a second rule (amendment of 2026-10-24).
- **Only plain HTTP.** Chrome tries HTTPS first; `127.0.0.2:443` reaches Docker's wildcard `*:443`, whose certificate does not match, before the browser falls back. Typing `http://` once avoids it.
- A dev server that writes absolute URLs from its `Host` writes `localhost:<port>` ones, which work but leave the name. Passing the typed `Host` through instead is the other half of the trade-off below.
- `setup` and `teardown` exist only for macOS. On Linux the front door runs, but the redirect is the owner's to make.
- The owner's `config.toml` keeps `server.port = 4317` until changed: `ariane config set server.port 27317`, then `ariane service install`.

## Alternatives considered

### A route in `cmc`'s nginx

It holds port 80 already. It would make `ariane.local` depend on the `cmc` Docker stack being up — the interface, kept up by launchd, would vanish whenever Docker stops — and it would put personal routes into a configuration the team versions.

### Ariane listening on port 80 itself

The bind is refused on any precise address without root (the `Errno 13` above), the wildcard is Docker's, and binding the wildcard would expose a server without authentication to the network, which ADR-0012 rules out.

### `*.localhost` names on a high port, no root at all

Chrome and Firefox resolve `*.localhost` to loopback by themselves, so nothing would touch the system. The port stays in every URL, which is what the owner wanted gone, and whether Safari follows is unverified. It remains the fallback should the redirect fail on a future macOS.

### An existing tool: Caddy, puma-dev, Laravel Valet

Each does the job — puma-dev with the same `pf` redirect and a DNS resolver, Valet with nginx and dnsmasq. The routes would live in one more tool's configuration instead of next to the projects in the tree, and none knows the nodes `ariane proxy <node>` names.

### Passing the typed `Host` through to the upstream

Truer to the browser, and what puma-dev does. Vite, since 6.0.9 and 5.4.12, answers "Blocked request. This host is not allowed" to a `Host` that is not `localhost` or an IP, until each project lists its name in `server.allowedHosts`: every project would need a change of its own to be reached.

### A random or ephemeral front-door port

No collision, but the `pf` rule names the port: it would have to be reloaded, with `sudo`, at every start of the server.

## Revisit if

- The redirect stops working on a macOS update, or Docker starts binding `127.0.0.2` explicitly — `ariane proxy status` shows which link broke.
- Reaching a project through its name needs HTTPS (secure cookies, service workers, OAuth callbacks), which would bring certificates and port 443 into the front door.
- `.local` lookups stay slow despite the IPv6 half of the chain, which would argue for another suffix.
- The direct probe of the front door's port gets explained, or starts failing through the redirects too.
- Rebooting and re-running `setup` becomes a chore, which is the moment for the LaunchDaemon left out here.

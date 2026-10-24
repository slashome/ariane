//! What `ariane proxy setup`, `status` and `teardown` do to the machine
//! (ADR-0016): the loopback alias, the `pf` redirect and the names in
//! `/etc/hosts`, all three until the next reboot.
//!
//! Every command that needs root runs through `sudo`, one at a time, printed
//! before it runs. The `pf` rule lives in its own anchor, `com.apple/ariane`,
//! which the stock `/etc/pf.conf` already loads (`rdr-anchor "com.apple/*"`),
//! so no system file but `/etc/hosts` is edited, and only between two marker
//! lines. The files handed to `sudo` are written next to `config.toml` first.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use super::door::MARK;
use super::{ALIAS, Table};
use crate::config::Config;
use crate::service::{Runner, System};
use crate::tree::NodePath;

/// The `pf` anchor holding the redirect.
const ANCHOR: &str = "com.apple/ariane";

/// The line that opens Ariane's block in `/etc/hosts`.
const BEGIN: &str = "# >>> ariane proxy: written by `ariane proxy setup` >>>";

/// The line that closes it.
const END: &str = "# <<< ariane proxy <<<";

/// How long a probe waits for a connection or an answer.
const PROBE: Duration = Duration::from_secs(1);

/// The machine the proxy is set up on.
pub struct Machine {
    /// Whether the machine runs macOS, the only system whose redirect Ariane knows.
    pub macos: bool,
    /// The hosts file, `/etc/hosts`.
    pub hosts: PathBuf,
    /// Where the `pf` rules and the next hosts file are written before `sudo` reads them.
    pub staging: PathBuf,
    /// The address whose port the redirect sends to the front door: port 80 of [`ALIAS`].
    pub redirected: SocketAddr,
    /// How the commands run.
    pub runner: Arc<dyn Runner + Send + Sync>,
}

impl Machine {
    /// This machine.
    pub fn current() -> Machine {
        let config = crate::config::file();
        Machine {
            macos: cfg!(target_os = "macos"),
            hosts: PathBuf::from("/etc/hosts"),
            staging: config.parent().map(Path::to_path_buf).unwrap_or_default(),
            redirected: SocketAddr::from((ALIAS, 80)),
            runner: Arc::new(System),
        }
    }
}

/// Aliases [`ALIAS`], redirects its port 80 to the front door on `port`, and
/// declares every route's name in `/etc/hosts`.
///
/// # Errors
///
/// Fails on another system than macOS, when a staged file cannot be written,
/// or when a command fails.
pub fn setup(machine: &Machine, table: &Table, port: u16) -> Result<ExitCode> {
    if !machine.macos {
        bail!(
            "`ariane proxy setup` only knows macOS's pf for now; the front door itself runs anywhere"
        );
    }
    std::fs::create_dir_all(&machine.staging)
        .with_context(|| format!("creating {}", machine.staging.display()))?;
    let rules = machine.staging.join("pf.conf");
    std::fs::write(&rules, rule(port)).with_context(|| format!("writing {}", rules.display()))?;
    let names: Vec<&str> = table
        .routes
        .iter()
        .map(|route| route.host.as_str())
        .collect();
    let mut steps = vec![
        sudo(&["ifconfig", "lo0", "alias", &ALIAS.to_string(), "up"]),
        sudo(&["pfctl", "-a", ANCHOR, "-f", &rules.display().to_string()]),
        sudo(&["pfctl", "-E"]),
    ];
    steps.extend(rewrite_hosts(machine, &names)?);
    steps.push(sudo(&["dscacheutil", "-flushcache"]));
    steps.push(sudo(&["killall", "-HUP", "mDNSResponder"]));
    for step in &steps {
        execute(machine, step, true)?;
    }
    println!("until the next reboot, these names reach the front door on 127.0.0.1:{port}:");
    for route in &table.routes {
        println!(
            "  http://{} → 127.0.0.1:{} ({})",
            route.host,
            route.port,
            route.owner()
        );
    }
    println!("check every link with `ariane proxy status`");
    Ok(ExitCode::SUCCESS)
}

/// Undoes [`setup`]: empties the anchor, removes the alias and Ariane's block
/// of `/etc/hosts`. A step that fails is reported and the others still run.
///
/// # Errors
///
/// Fails on another system than macOS, or when the hosts file cannot be read.
pub fn teardown(machine: &Machine) -> Result<ExitCode> {
    if !machine.macos {
        bail!("`ariane proxy teardown` only knows macOS's pf for now");
    }
    let mut steps = vec![
        sudo(&["pfctl", "-a", ANCHOR, "-F", "all"]),
        sudo(&["ifconfig", "lo0", "-alias", &ALIAS.to_string()]),
    ];
    steps.extend(rewrite_hosts(machine, &[])?);
    steps.push(sudo(&["dscacheutil", "-flushcache"]));
    steps.push(sudo(&["killall", "-HUP", "mDNSResponder"]));
    let mut failed = false;
    for step in &steps {
        failed |= !execute(machine, step, false)?;
    }
    Ok(if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// One link of the chain from the browser to a route, checked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Check {
    /// Whether the link holds.
    pub ok: bool,
    /// What the check is about, such as `alias 127.0.0.2 on lo0`.
    pub what: String,
    /// What to do when the link does not hold; empty when there is nothing to say.
    pub hint: String,
}

impl Check {
    /// A check of `what`, with `hint` shown only when the link is broken.
    fn new(ok: bool, what: String, hint: impl Into<String>) -> Check {
        Check {
            ok,
            what,
            hint: hint.into(),
        }
    }
}

impl std::fmt::Display for Check {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.ok {
            write!(f, "✓ {}", self.what)
        } else {
            write!(f, "✗ {}: {}", self.what, self.hint)
        }
    }
}

/// One route with the two links that are its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteState {
    /// The host name typed in the browser.
    pub host: String,
    /// The port of `127.0.0.1` the requests are forwarded to.
    pub port: u16,
    /// The node that declares the route; `None` for the interface itself.
    pub node: Option<NodePath>,
    /// Whether the hosts file sends the name to [`ALIAS`].
    pub declared: bool,
    /// Whether something accepts connections on the port.
    pub listening: bool,
}

/// Every check of the chain, and the routes they concern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// Whether every link holds.
    pub ok: bool,
    /// The port of the front door, `proxy.port`.
    pub proxy_port: u16,
    /// The port of the interface, `server.port`.
    pub server_port: u16,
    /// The checks, in the order `ariane proxy status` prints them.
    pub checks: Vec<Check>,
    /// The routes, the interface's first.
    pub routes: Vec<RouteState>,
    /// Why a declared route was left out of the table.
    pub problems: Vec<String>,
}

/// Checks every link from the browser to each route: the alias, the front
/// door, the redirect, then each route's name in the hosts file and its port.
///
/// The probes are blocking connections, each bounded by [`PROBE`].
pub fn check(machine: &Machine, config: &Config, table: &Table) -> Report {
    let mut checks = Vec::new();
    let aliased = machine
        .runner
        .run("ifconfig", &["lo0".to_owned()])
        .is_ok_and(|outcome| outcome.stdout.contains(&format!("inet {ALIAS} ")));
    checks.push(Check::new(
        aliased,
        format!("alias {ALIAS} on lo0"),
        "missing: run `ariane proxy setup`",
    ));
    let door = SocketAddr::from((Ipv4Addr::LOCALHOST, config.proxy.port));
    checks.push(probe_check(
        probe(door),
        format!("front door on {door}"),
        "is `ariane serve` or `ariane proxy` running?",
    ));
    let redirected = machine.redirected;
    checks.push(probe_check(
        probe(redirected),
        format!("{redirected} redirected to the front door"),
        "run `ariane proxy setup`",
    ));
    let hosts = std::fs::read_to_string(&machine.hosts).unwrap_or_default();
    let mut routes = Vec::new();
    for route in &table.routes {
        let declared = resolves_to(&hosts, &route.host) == Some(IpAddr::V4(ALIAS));
        checks.push(Check::new(
            declared,
            format!("{} → {ALIAS} in {}", route.host, machine.hosts.display()),
            "run `ariane proxy setup` to declare it",
        ));
        let upstream = SocketAddr::from((Ipv4Addr::LOCALHOST, route.port));
        let listening = TcpStream::connect_timeout(&upstream, PROBE).is_ok();
        checks.push(Check::new(
            listening,
            format!("{} → {upstream}, {}", route.host, route.owner()),
            "nothing listens: start its server",
        ));
        routes.push(RouteState {
            host: route.host.clone(),
            port: route.port,
            node: route.node.clone(),
            declared,
            listening,
        });
    }
    Report {
        ok: checks.iter().all(|check| check.ok),
        proxy_port: config.proxy.port,
        server_port: config.server.port,
        checks,
        routes,
        problems: table.problems.clone(),
    }
}

/// Prints every check of [`check`], one line each, and exits with `1` when a
/// link is broken.
pub fn status(machine: &Machine, config: &Config, table: &Table) -> ExitCode {
    let report = check(machine, config, table);
    for check in &report.checks {
        println!("{check}");
    }
    if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// What answered a probe of the front door.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// The front door, recognised by [`MARK`].
    Door,
    /// Some other HTTP server.
    Other,
    /// Nothing, with the reason.
    Nothing(String),
}

/// The check of a probe of `what`; `hint` when nothing answers.
fn probe_check(answer: Answer, what: String, hint: &str) -> Check {
    match answer {
        Answer::Door => Check::new(true, what, ""),
        Answer::Other => Check::new(
            false,
            what,
            "another server answers there instead of the front door",
        ),
        Answer::Nothing(reason) => Check::new(false, what, format!("{reason}; {hint}")),
    }
}

/// Asks `address` for a name no route has, and tells who answered.
fn probe(address: SocketAddr) -> Answer {
    let exchange = || -> std::io::Result<String> {
        let mut stream = TcpStream::connect_timeout(&address, PROBE)?;
        stream.set_read_timeout(Some(PROBE))?;
        stream.write_all(
            b"GET / HTTP/1.1\r\nHost: probe.ariane.invalid\r\nConnection: close\r\n\r\n",
        )?;
        let mut answer = Vec::new();
        let _ = stream.take(4096).read_to_end(&mut answer);
        Ok(String::from_utf8_lossy(&answer).to_ascii_lowercase())
    };
    match exchange() {
        Ok(answer) if answer.contains(&format!("\r\n{MARK}:")) => Answer::Door,
        Ok(answer) if answer.starts_with("http/") => Answer::Other,
        Ok(_) => Answer::Nothing("no HTTP answer".to_owned()),
        Err(error) => Answer::Nothing(error.to_string()),
    }
}

/// The `pf` rule sending port 80 of [`ALIAS`] to the front door on `port`.
fn rule(port: u16) -> String {
    format!("rdr pass on lo0 inet proto tcp from any to {ALIAS} port 80 -> 127.0.0.1 port {port}\n")
}

/// The steps that give `/etc/hosts` Ariane's block for `names` (none removes
/// it); no step when the file already has it.
///
/// # Errors
///
/// Fails when the hosts file cannot be read or the staged copy written.
fn rewrite_hosts(machine: &Machine, names: &[&str]) -> Result<Vec<Vec<String>>> {
    let current = std::fs::read_to_string(&machine.hosts)
        .with_context(|| format!("reading {}", machine.hosts.display()))?;
    let next = with_block(&current, names);
    if next == current {
        return Ok(Vec::new());
    }
    std::fs::create_dir_all(&machine.staging)
        .with_context(|| format!("creating {}", machine.staging.display()))?;
    let staged = machine.staging.join("hosts");
    std::fs::write(&staged, next).with_context(|| format!("writing {}", staged.display()))?;
    Ok(vec![sudo(&[
        "cp",
        &staged.display().to_string(),
        &machine.hosts.display().to_string(),
    ])])
}

/// `hosts` with Ariane's block replaced by one naming `names`, at the end;
/// without any block when `names` is empty. Every other line is kept as is.
fn with_block(hosts: &str, names: &[&str]) -> String {
    let mut kept = Vec::new();
    let mut inside = false;
    for line in hosts.lines() {
        match line.trim() {
            BEGIN => inside = true,
            END => inside = false,
            _ if !inside => kept.push(line),
            _ => {}
        }
    }
    while kept.last().is_some_and(|line| line.trim().is_empty()) {
        kept.pop();
    }
    let mut text = kept.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    if !names.is_empty() {
        text.push('\n');
        text.push_str(BEGIN);
        text.push('\n');
        for name in names {
            text.push_str(&format!("{ALIAS}\t{name}\n"));
        }
        text.push_str(END);
        text.push('\n');
    }
    text
}

/// The address `/etc/hosts` gives `name`, its first line naming it.
fn resolves_to(hosts: &str, name: &str) -> Option<IpAddr> {
    hosts.lines().find_map(|line| {
        let line = line.split('#').next().unwrap_or_default();
        let mut words = line.split_whitespace();
        let address = words.next()?;
        words
            .any(|word| word.eq_ignore_ascii_case(name))
            .then(|| address.parse().ok())
            .flatten()
    })
}

/// `sudo` followed by `args`, as a command line.
fn sudo(args: &[&str]) -> Vec<String> {
    std::iter::once("sudo")
        .chain(args.iter().copied())
        .map(str::to_owned)
        .collect()
}

/// Prints `step`, runs it and reports a failure. Returns whether it succeeded;
/// with `strict`, a failure is an error instead.
///
/// # Errors
///
/// Fails when the command cannot start, or fails while `strict`.
fn execute(machine: &Machine, step: &[String], strict: bool) -> Result<bool> {
    println!("$ {}", step.join(" "));
    let outcome = machine.runner.run(&step[0], &step[1..])?;
    if outcome.success {
        return Ok(true);
    }
    let reason = outcome.stderr.trim();
    if strict {
        bail!("`{}` failed: {reason}", step.join(" "));
    }
    eprintln!("ariane proxy: `{}` failed: {reason}", step.join(" "));
    Ok(false)
}

/// Unit tests of the setup and its files, over a fake runner.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::Route;
    use crate::service::Outcome;
    use std::sync::Mutex;

    /// A runner that records every command and succeeds, printing `stdout`.
    #[derive(Default)]
    struct Fake {
        commands: Mutex<Vec<String>>,
        stdout: String,
    }

    impl Fake {
        /// The commands run so far.
        fn commands(&self) -> Vec<String> {
            self.commands.lock().unwrap().clone()
        }
    }

    impl Runner for Fake {
        fn run(&self, program: &str, args: &[String]) -> Result<Outcome> {
            self.commands
                .lock()
                .unwrap()
                .push(format!("{program} {}", args.join(" ")));
            Ok(Outcome {
                success: true,
                stdout: self.stdout.clone(),
                ..Outcome::default()
            })
        }
    }

    /// A port of `127.0.0.1` that nothing listens on.
    fn closed_port() -> u16 {
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.local_addr().unwrap().port()
    }

    /// A macOS machine whose hosts file holds `hosts`, in a temporary directory,
    /// whose redirect leads nowhere.
    fn machine(dir: &Path, hosts: &str, runner: Arc<Fake>) -> Machine {
        std::fs::write(dir.join("etc-hosts"), hosts).unwrap();
        Machine {
            macos: true,
            hosts: dir.join("etc-hosts"),
            staging: dir.join("staging"),
            redirected: SocketAddr::from((Ipv4Addr::LOCALHOST, closed_port())),
            runner,
        }
    }

    /// A table of the interface and one project.
    fn table() -> Table {
        Table {
            routes: vec![
                Route {
                    host: "ariane.local".to_owned(),
                    port: 27317,
                    node: None,
                },
                Route {
                    host: "dreamcatchr.local".to_owned(),
                    port: 3000,
                    node: Some(crate::tree::NodePath::new("slashome/dreamcatchr")),
                },
            ],
            problems: Vec::new(),
        }
    }

    #[test]
    fn the_block_is_added_replaced_and_removed_keeping_every_other_line() {
        let hosts = "127.0.0.1\tlocalhost\n127.0.0.1 h.local # mine\n";
        let added = with_block(hosts, &["ariane.local", "d.local"]);
        assert_eq!(
            added,
            format!("{hosts}\n{BEGIN}\n127.0.0.2\tariane.local\n127.0.0.2\td.local\n{END}\n")
        );
        let replaced = with_block(&added, &["ariane.local"]);
        assert_eq!(
            replaced,
            format!("{hosts}\n{BEGIN}\n127.0.0.2\tariane.local\n{END}\n")
        );
        assert_eq!(with_block(&replaced, &[]), hosts);
        assert_eq!(with_block(&added, &["ariane.local", "d.local"]), added);
    }

    #[test]
    fn a_name_resolves_through_its_first_line_ignoring_comments() {
        let hosts = "# 10.0.0.1 a.local\n127.0.0.2\tA.local b.local\n127.0.0.1 a.local\n";
        assert_eq!(resolves_to(hosts, "a.local"), Some(IpAddr::V4(ALIAS)));
        assert_eq!(resolves_to(hosts, "b.local"), Some(IpAddr::V4(ALIAS)));
        assert_eq!(resolves_to(hosts, "c.local"), None);
    }

    #[test]
    fn setup_aliases_redirects_declares_and_flushes_in_that_order() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(Fake::default());
        let machine = machine(dir.path(), "127.0.0.1 localhost\n", fake.clone());
        setup(&machine, &table(), 27380).unwrap();
        let staging = dir.path().join("staging");
        assert_eq!(
            fake.commands(),
            [
                "sudo ifconfig lo0 alias 127.0.0.2 up".to_owned(),
                format!(
                    "sudo pfctl -a com.apple/ariane -f {}",
                    staging.join("pf.conf").display()
                ),
                "sudo pfctl -E".to_owned(),
                format!(
                    "sudo cp {} {}",
                    staging.join("hosts").display(),
                    dir.path().join("etc-hosts").display()
                ),
                "sudo dscacheutil -flushcache".to_owned(),
                "sudo killall -HUP mDNSResponder".to_owned(),
            ]
        );
        assert_eq!(
            std::fs::read_to_string(staging.join("pf.conf")).unwrap(),
            "rdr pass on lo0 inet proto tcp from any to 127.0.0.2 port 80 -> 127.0.0.1 port 27380\n"
        );
        let staged = std::fs::read_to_string(staging.join("hosts")).unwrap();
        assert!(staged.contains("127.0.0.2\tdreamcatchr.local\n"));
    }

    #[test]
    fn hosts_already_declared_are_not_copied_again() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(Fake::default());
        let hosts = with_block(
            "127.0.0.1 localhost\n",
            &["ariane.local", "dreamcatchr.local"],
        );
        let machine = machine(dir.path(), &hosts, fake.clone());
        setup(&machine, &table(), 27380).unwrap();
        assert!(
            !fake
                .commands()
                .iter()
                .any(|command| command.contains(" cp "))
        );
    }

    #[test]
    fn teardown_empties_the_anchor_removes_the_alias_and_the_block() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(Fake::default());
        let hosts = with_block("127.0.0.1 localhost\n", &["ariane.local"]);
        let machine = machine(dir.path(), &hosts, fake.clone());
        teardown(&machine).unwrap();
        let commands = fake.commands();
        assert_eq!(commands[0], "sudo pfctl -a com.apple/ariane -F all");
        assert_eq!(commands[1], "sudo ifconfig lo0 -alias 127.0.0.2");
        assert!(commands[2].starts_with("sudo cp "));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("staging/hosts")).unwrap(),
            "127.0.0.1 localhost\n"
        );
    }

    #[test]
    fn setup_refuses_another_system() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(Fake::default());
        let mut machine = machine(dir.path(), "", fake.clone());
        machine.macos = false;
        assert!(setup(&machine, &table(), 27380).is_err());
        assert!(fake.commands().is_empty());
    }

    #[test]
    fn a_probe_tells_the_front_door_from_another_server_and_from_nothing() {
        let closed = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = closed.local_addr().unwrap();
        drop(closed);
        assert!(matches!(probe(address), Answer::Nothing(_)));
        let other = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = other.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut socket, _) = other.accept().unwrap();
            let mut buffer = [0; 512];
            let _ = socket.read(&mut buffer);
            socket
                .write_all(b"HTTP/1.1 404 Not Found\r\nServer: nginx\r\n\r\n")
                .unwrap();
        });
        assert_eq!(probe(address), Answer::Other);
    }

    /// Answers every connection on a free port of `127.0.0.1` as the front
    /// door does, marked with [`MARK`], and returns the port.
    fn front_door() -> u16 {
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for socket in listener.incoming() {
                let Ok(mut socket) = socket else { continue };
                let mut buffer = [0; 512];
                let _ = socket.read(&mut buffer);
                let _ = socket.write_all(
                    format!("HTTP/1.1 404 Not Found\r\n{MARK}: 1\r\ncontent-length: 0\r\n\r\n")
                        .as_bytes(),
                );
            }
        });
        port
    }

    #[test]
    fn the_report_checks_each_link_in_the_order_status_prints_them() {
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(Fake {
            stdout: "lo0: flags\n\tinet 127.0.0.1 netmask 0xff000000\n\tinet 127.0.0.2 netmask 0xff000000\n".to_owned(),
            ..Fake::default()
        });
        let hosts = with_block("127.0.0.1 localhost\n", &["ariane.local"]);
        let mut machine = machine(dir.path(), &hosts, fake.clone());
        let door = front_door();
        machine.redirected = SocketAddr::from((Ipv4Addr::LOCALHOST, door));
        let upstream = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let mut config = Config::default();
        config.proxy.port = door;
        let mut table = table();
        table.routes[0].port = upstream.local_addr().unwrap().port();
        table.routes[1].port = closed_port();
        table
            .problems
            .push("node q: `Q` is not a host name".to_owned());

        let report = check(&machine, &config, &table);

        assert_eq!(fake.commands(), ["ifconfig lo0"]);
        let lines: Vec<String> = report.checks.iter().map(ToString::to_string).collect();
        assert_eq!(lines.len(), 7);
        assert_eq!(lines[0], "✓ alias 127.0.0.2 on lo0");
        assert_eq!(lines[1], format!("✓ front door on 127.0.0.1:{door}"));
        assert_eq!(
            lines[2],
            format!("✓ 127.0.0.1:{door} redirected to the front door")
        );
        assert!(lines[3].starts_with("✓ ariane.local → 127.0.0.2 in "));
        assert!(lines[4].ends_with(", the interface (`server.host`)"));
        assert!(lines[4].starts_with("✓ "));
        assert!(lines[5].starts_with("✗ dreamcatchr.local → 127.0.0.2 in "));
        assert!(lines[5].ends_with(": run `ariane proxy setup` to declare it"));
        assert!(lines[6].ends_with("node slashome/dreamcatchr: nothing listens: start its server"));
        assert!(!report.ok);
        assert_eq!(report.proxy_port, door);
        assert_eq!(report.server_port, config.server.port);
        assert_eq!(
            report.routes[0],
            RouteState {
                host: "ariane.local".to_owned(),
                port: table.routes[0].port,
                node: None,
                declared: true,
                listening: true,
            }
        );
        assert!(!report.routes[1].declared && !report.routes[1].listening);
        assert_eq!(report.problems, table.problems);
    }

    #[test]
    fn a_missing_alias_and_a_silent_door_are_broken_with_their_remedy() {
        let dir = tempfile::tempdir().unwrap();
        let machine = machine(dir.path(), "", Arc::new(Fake::default()));
        let mut config = Config::default();
        config.proxy.port = closed_port();
        let table = Table::default();
        let report = check(&machine, &config, &table);
        assert!(!report.ok);
        assert_eq!(report.checks.len(), 3);
        assert_eq!(
            report.checks[0].to_string(),
            "✗ alias 127.0.0.2 on lo0: missing: run `ariane proxy setup`"
        );
        assert!(
            report.checks[1]
                .hint
                .ends_with("; is `ariane serve` or `ariane proxy` running?")
        );
        assert!(
            report.checks[2]
                .hint
                .ends_with("; run `ariane proxy setup`")
        );
    }
}

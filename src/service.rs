//! `ariane service`: keep `ariane serve` running in the background (ADR-0012).
//!
//! The binary never daemonizes itself. It hands the job to the operating
//! system's service manager: a **launchd agent** on macOS, a **systemd user
//! unit** on Linux. Both restart the server when it stops and start it at
//! login; both write its output to a log the `logs` subcommand reads.
//!
//! Everything that touches the machine goes through a [`Machine`](crate::service::Machine): the home
//! directory, the path of the executable, and a [`Runner`](crate::service::Runner) for `launchctl`,
//! `systemctl` and `id`. Tests build one over a temporary directory and a fake
//! runner, so they check the generated files and the exact commands without
//! loading anything.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

use crate::cli::{ServiceArgs, ServiceCommand};

/// The launchd label, also the name of the agent's plist file.
pub const LABEL: &str = "com.slashome.ariane";

/// The systemd unit name.
pub const UNIT: &str = "ariane.service";

/// The `PATH` the service runs with, so that `git` is found whether it comes
/// from Homebrew (Apple silicon or Intel) or the system.
const SERVICE_PATH: &str = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin";

/// What a command run by a [`Runner`] produced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outcome {
    /// Whether the command exited with status 0.
    pub success: bool,
    /// Its standard output, as text.
    pub stdout: String,
    /// Its standard error, as text.
    pub stderr: String,
}

/// Runs an external command and captures its result.
pub trait Runner {
    /// Runs `program` with `args` and waits for it.
    ///
    /// # Errors
    ///
    /// Fails when the program cannot be started at all.
    fn run(&self, program: &str, args: &[String]) -> Result<Outcome>;
}

/// The real runner: spawns the process.
pub struct System;

impl Runner for System {
    fn run(&self, program: &str, args: &[String]) -> Result<Outcome> {
        let output = std::process::Command::new(program)
            .args(args)
            .output()
            .with_context(|| format!("running {program}"))?;
        Ok(Outcome {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// The service manager of the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Manager {
    /// macOS: a launchd agent in `~/Library/LaunchAgents`.
    Launchd,
    /// Linux: a systemd unit in `~/.config/systemd/user`.
    Systemd,
}

impl Manager {
    /// The manager of the platform this binary was compiled for.
    pub fn current() -> Manager {
        if cfg!(target_os = "macos") {
            Manager::Launchd
        } else {
            Manager::Systemd
        }
    }
}

/// The machine a service is installed on: where things live, and how commands run.
pub struct Machine<'a> {
    /// The service manager to talk to.
    pub manager: Manager,
    /// The home directory under which the service files and logs live.
    pub home: PathBuf,
    /// The absolute path of the `ariane` executable the service runs.
    pub exe: PathBuf,
    /// The port the server listens on, for the URL `status` prints.
    pub port: u16,
    /// How external commands run.
    pub runner: &'a dyn Runner,
}

/// Runs `ariane service <subcommand>` on this machine.
///
/// `install` refuses to proceed while `repo` is not configured: the service
/// starts in the home directory, where only the configuration can tell it
/// which content repository to serve, and a server that cannot start would be
/// restarted in a loop.
///
/// # Errors
///
/// Fails when the home directory or the executable cannot be found, the
/// configuration is invalid, or the service manager refuses a command.
pub fn run(args: &ServiceArgs) -> Result<ExitCode> {
    let config = crate::config::load(&crate::config::file())?.config;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")?;
    let exe = std::env::current_exe()
        .and_then(|exe| exe.canonicalize())
        .context("locating the ariane executable")?;
    let machine = Machine {
        manager: Manager::current(),
        home,
        exe,
        port: config.server.port,
        runner: &System,
    };
    match &args.command {
        ServiceCommand::Install => {
            let Some(repo) = config.repo_path() else {
                bail!(
                    "no content repository is configured; set it first with `ariane config set repo <path>`"
                );
            };
            if !crate::repo::is_content_repository(&repo) {
                bail!("{} is not an Ariane content repository", repo.display());
            }
            install(&machine)?;
            println!(
                "ariane serve is installed and running on http://127.0.0.1:{}",
                machine.port
            );
        }
        ServiceCommand::Uninstall => {
            uninstall(&machine)?;
            println!("ariane serve is no longer installed as a service");
        }
        ServiceCommand::Status => print!("{}", status(&machine)?),
        ServiceCommand::Logs { follow } => logs(&machine, *follow)?,
    }
    Ok(ExitCode::SUCCESS)
}

/// Path of the launchd agent file.
pub fn plist_path(home: &Path) -> PathBuf {
    home.join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

/// Path of the server's log under launchd.
pub fn log_path(home: &Path) -> PathBuf {
    home.join("Library/Logs/ariane/serve.log")
}

/// Path of the systemd user unit file.
pub fn unit_path(home: &Path) -> PathBuf {
    home.join(".config/systemd/user").join(UNIT)
}

/// The launchd agent: runs `<exe> serve` at login, restarts it whenever it
/// stops, from the home directory, logging to [`log_path`].
pub fn plist(exe: &Path, home: &Path) -> String {
    let log = escape(&log_path(home).to_string_lossy());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{LABEL}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{exe}</string>
		<string>serve</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>KeepAlive</key>
	<true/>
	<key>WorkingDirectory</key>
	<string>{home}</string>
	<key>EnvironmentVariables</key>
	<dict>
		<key>PATH</key>
		<string>{SERVICE_PATH}</string>
	</dict>
	<key>StandardOutPath</key>
	<string>{log}</string>
	<key>StandardErrorPath</key>
	<string>{log}</string>
</dict>
</plist>
"#,
        exe = escape(&exe.to_string_lossy()),
        home = escape(&home.to_string_lossy()),
    )
}

/// The systemd user unit: runs `<exe> serve` from the home directory and
/// restarts it whenever it stops; its output goes to the user journal.
pub fn unit(exe: &Path, home: &Path) -> String {
    format!(
        "[Unit]\n\
         Description=Ariane interface (ariane serve)\n\
         \n\
         [Service]\n\
         ExecStart=\"{exe}\" serve\n\
         WorkingDirectory={home}\n\
         Environment=PATH={SERVICE_PATH}\n\
         Restart=always\n\
         RestartSec=2\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exe = exe.display(),
        home = home.display(),
    )
}

/// Writes the service file and starts the service, replacing a previous one.
///
/// # Errors
///
/// Fails when a file cannot be written or the service manager refuses.
pub fn install(machine: &Machine) -> Result<()> {
    match machine.manager {
        Manager::Launchd => {
            let plist_file = plist_path(&machine.home);
            write(&plist_file, &plist(&machine.exe, &machine.home))?;
            let log = log_path(&machine.home);
            if let Some(directory) = log.parent() {
                std::fs::create_dir_all(directory)
                    .with_context(|| format!("creating {}", directory.display()))?;
            }
            let domain = gui_domain(machine)?;
            let target = format!("{domain}/{LABEL}");
            if launchctl(machine, &["print", &target])?.success {
                launchctl(machine, &["bootout", &target])?;
            }
            let file = plist_file.to_string_lossy().into_owned();
            expect(
                launchctl(machine, &["bootstrap", &domain, &file])?,
                "launchctl bootstrap",
            )
        }
        Manager::Systemd => {
            write(
                &unit_path(&machine.home),
                &unit(&machine.exe, &machine.home),
            )?;
            expect(
                systemctl(machine, &["daemon-reload"])?,
                "systemctl --user daemon-reload",
            )?;
            expect(
                systemctl(machine, &["enable", "--now", UNIT])?,
                "systemctl --user enable --now",
            )
        }
    }
}

/// Stops the service and removes its file. Removing a service that is not
/// installed is not an error.
///
/// # Errors
///
/// Fails when the service file cannot be removed or the manager fails to run.
pub fn uninstall(machine: &Machine) -> Result<()> {
    match machine.manager {
        Manager::Launchd => {
            let target = format!("{}/{LABEL}", gui_domain(machine)?);
            if launchctl(machine, &["print", &target])?.success {
                launchctl(machine, &["bootout", &target])?;
            }
            remove(&plist_path(&machine.home))
        }
        Manager::Systemd => {
            systemctl(machine, &["disable", "--now", UNIT])?;
            remove(&unit_path(&machine.home))?;
            systemctl(machine, &["daemon-reload"])?;
            Ok(())
        }
    }
}

/// A short report: whether the service is installed, loaded and running, and
/// the URL it serves.
///
/// # Errors
///
/// Fails when the service manager cannot be run.
pub fn status(machine: &Machine) -> Result<String> {
    let url = format!("http://127.0.0.1:{}", machine.port);
    let (file, loaded, pid) = match machine.manager {
        Manager::Launchd => {
            let target = format!("{}/{LABEL}", gui_domain(machine)?);
            let printed = launchctl(machine, &["print", &target])?;
            let pid = printed
                .success
                .then(|| launchd_pid(&printed.stdout))
                .flatten();
            (plist_path(&machine.home), printed.success, pid)
        }
        Manager::Systemd => {
            let enabled = systemctl(machine, &["is-enabled", UNIT])?.success;
            let shown = systemctl(machine, &["show", UNIT, "--property=MainPID", "--value"])?;
            let pid = shown
                .stdout
                .trim()
                .parse::<u32>()
                .ok()
                .filter(|pid| *pid != 0);
            (unit_path(&machine.home), enabled, pid)
        }
    };
    if !file.exists() && !loaded {
        return Ok(format!(
            "not installed (no {})\ninstall it with `ariane service install`\n",
            file.display()
        ));
    }
    let running = match pid {
        Some(pid) => format!("running, pid {pid}, on {url}"),
        None => "not running".to_owned(),
    };
    Ok(format!(
        "installed: {}\nloaded: {}\n{running}\n",
        file.display(),
        if loaded { "yes" } else { "no" }
    ))
}

/// Prints the last 100 lines of the server's log, then keeps following it with
/// `follow`.
///
/// # Errors
///
/// Fails when the log cannot be read or the follower cannot be started.
pub fn logs(machine: &Machine, follow: bool) -> Result<()> {
    match machine.manager {
        Manager::Launchd => {
            let log = log_path(&machine.home);
            if follow {
                let status = std::process::Command::new("tail")
                    .args(["-n", "100", "-F"])
                    .arg(&log)
                    .status()
                    .context("running tail")?;
                if !status.success() {
                    bail!("tail exited with {status}");
                }
                return Ok(());
            }
            let text = std::fs::read_to_string(&log)
                .with_context(|| format!("reading {}", log.display()))?;
            print!("{}", last_lines(&text, 100));
            Ok(())
        }
        Manager::Systemd => {
            let mut args = vec!["--user", "-u", UNIT, "-n", "100"];
            if follow {
                args.push("-f");
            }
            let status = std::process::Command::new("journalctl")
                .args(&args)
                .status()
                .context("running journalctl")?;
            if !status.success() {
                bail!("journalctl exited with {status}");
            }
            Ok(())
        }
    }
}

/// The launchd domain of the user's graphical session, `gui/<uid>`.
///
/// # Errors
///
/// Fails when `id -u` cannot run or prints no number.
fn gui_domain(machine: &Machine) -> Result<String> {
    let id = machine.runner.run("id", &["-u".to_owned()])?;
    let uid = id.stdout.trim();
    if !id.success || uid.is_empty() || !uid.chars().all(|c| c.is_ascii_digit()) {
        bail!("could not read the user id: {}", id.stderr.trim());
    }
    Ok(format!("gui/{uid}"))
}

/// Runs `launchctl` with `args`.
///
/// # Errors
///
/// Fails when `launchctl` cannot be started.
fn launchctl(machine: &Machine, args: &[&str]) -> Result<Outcome> {
    let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    machine.runner.run("launchctl", &args)
}

/// Runs `systemctl --user` with `args`.
///
/// # Errors
///
/// Fails when `systemctl` cannot be started.
fn systemctl(machine: &Machine, args: &[&str]) -> Result<Outcome> {
    let args: Vec<String> = std::iter::once("--user")
        .chain(args.iter().copied())
        .map(str::to_owned)
        .collect();
    machine.runner.run("systemctl", &args)
}

/// Turns a failed [`Outcome`] into an error carrying its standard error.
///
/// # Errors
///
/// Fails when the outcome is a failure.
fn expect(outcome: Outcome, what: &str) -> Result<()> {
    if outcome.success {
        Ok(())
    } else {
        bail!("{what} failed: {}", outcome.stderr.trim())
    }
}

/// The `pid = N` line of `launchctl print`, if the agent is running.
fn launchd_pid(printed: &str) -> Option<u32> {
    printed
        .lines()
        .find_map(|line| line.trim().strip_prefix("pid = "))
        .and_then(|pid| pid.trim().parse().ok())
}

/// The last `count` lines of `text`, each with its line ending.
fn last_lines(text: &str, count: usize) -> String {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    lines[lines.len().saturating_sub(count)..].concat()
}

/// Escapes the characters XML does not allow in text.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Writes `text` to `file`, creating its directory.
///
/// # Errors
///
/// Fails when the directory or the file cannot be written.
fn write(file: &Path, text: &str) -> Result<()> {
    if let Some(directory) = file.parent() {
        std::fs::create_dir_all(directory)
            .with_context(|| format!("creating {}", directory.display()))?;
    }
    std::fs::write(file, text).with_context(|| format!("writing {}", file.display()))
}

/// Removes `file`; a missing file is fine.
///
/// # Errors
///
/// Fails when the file exists but cannot be removed.
fn remove(file: &Path) -> Result<()> {
    match std::fs::remove_file(file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("removing {}", file.display())),
    }
}

/// Unit tests of the service files and of the commands sent to the managers,
/// over a temporary home and a recording runner.
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A runner that records every command and answers from a script.
    struct Fake {
        calls: RefCell<Vec<String>>,
        loaded: bool,
    }

    impl Fake {
        fn new(loaded: bool) -> Fake {
            Fake {
                calls: RefCell::new(Vec::new()),
                loaded,
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl Runner for Fake {
        fn run(&self, program: &str, args: &[String]) -> Result<Outcome> {
            let line = format!("{program} {}", args.join(" "));
            self.calls.borrow_mut().push(line.clone());
            Ok(match program {
                "id" => Outcome {
                    success: true,
                    stdout: "501\n".to_owned(),
                    ..Outcome::default()
                },
                "launchctl" if args[0] == "print" => Outcome {
                    success: self.loaded,
                    stdout: "\tstate = running\n\tpid = 4242\n".to_owned(),
                    ..Outcome::default()
                },
                "systemctl" if args.get(1).map(String::as_str) == Some("show") => Outcome {
                    success: true,
                    stdout: "4243\n".to_owned(),
                    ..Outcome::default()
                },
                _ => Outcome {
                    success: true,
                    ..Outcome::default()
                },
            })
        }
    }

    /// A machine over `home` with `runner`.
    fn machine<'a>(manager: Manager, home: &Path, runner: &'a Fake) -> Machine<'a> {
        Machine {
            manager,
            home: home.to_path_buf(),
            exe: PathBuf::from("/opt/ari & ne/bin/ariane"),
            port: 4317,
            runner,
        }
    }

    #[test]
    fn the_plist_runs_serve_from_home_and_logs_to_the_library() {
        let text = plist(Path::new("/x/ariane"), Path::new("/Users/me"));
        assert!(text.contains("<string>com.slashome.ariane</string>"));
        assert!(text.contains("<string>/x/ariane</string>\n\t\t<string>serve</string>"));
        assert!(text.contains("<key>RunAtLoad</key>\n\t<true/>"));
        assert!(text.contains("<key>KeepAlive</key>\n\t<true/>"));
        assert!(text.contains("<string>/Users/me</string>"));
        assert!(text.contains("/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin"));
        assert!(text.contains("<string>/Users/me/Library/Logs/ariane/serve.log</string>"));
    }

    #[test]
    fn the_plist_escapes_paths() {
        let text = plist(Path::new("/a & b/<ariane>"), Path::new("/h"));
        assert!(text.contains("<string>/a &amp; b/&lt;ariane&gt;</string>"));
    }

    #[test]
    fn the_unit_runs_serve_and_restarts_it() {
        let text = unit(Path::new("/x/ariane"), Path::new("/home/me"));
        assert!(text.contains("ExecStart=\"/x/ariane\" serve\n"));
        assert!(text.contains("WorkingDirectory=/home/me\n"));
        assert!(text.contains("Restart=always\n"));
        assert!(text.contains("WantedBy=default.target\n"));
    }

    #[test]
    fn installing_with_launchd_writes_the_agent_and_bootstraps_it() {
        let home = tempfile::tempdir().unwrap();
        let runner = Fake::new(false);
        install(&machine(Manager::Launchd, home.path(), &runner)).unwrap();
        let plist_file = plist_path(home.path());
        assert!(
            std::fs::read_to_string(&plist_file)
                .unwrap()
                .contains("serve")
        );
        assert!(log_path(home.path()).parent().unwrap().is_dir());
        assert_eq!(
            runner.calls(),
            [
                "id -u".to_owned(),
                "launchctl print gui/501/com.slashome.ariane".to_owned(),
                format!("launchctl bootstrap gui/501 {}", plist_file.display()),
            ]
        );
    }

    #[test]
    fn reinstalling_with_launchd_boots_the_old_agent_out_first() {
        let home = tempfile::tempdir().unwrap();
        let runner = Fake::new(true);
        install(&machine(Manager::Launchd, home.path(), &runner)).unwrap();
        assert!(
            runner
                .calls()
                .contains(&"launchctl bootout gui/501/com.slashome.ariane".to_owned())
        );
    }

    #[test]
    fn uninstalling_with_launchd_boots_out_and_removes_the_agent() {
        let home = tempfile::tempdir().unwrap();
        let runner = Fake::new(true);
        let machine = machine(Manager::Launchd, home.path(), &runner);
        install(&machine).unwrap();
        uninstall(&machine).unwrap();
        assert!(!plist_path(home.path()).exists());
        assert_eq!(
            runner.calls().last().unwrap(),
            "launchctl bootout gui/501/com.slashome.ariane"
        );
    }

    #[test]
    fn installing_with_systemd_enables_the_unit() {
        let home = tempfile::tempdir().unwrap();
        let runner = Fake::new(false);
        install(&machine(Manager::Systemd, home.path(), &runner)).unwrap();
        assert!(unit_path(home.path()).exists());
        assert_eq!(
            runner.calls(),
            [
                "systemctl --user daemon-reload",
                "systemctl --user enable --now ariane.service",
            ]
        );
    }

    #[test]
    fn status_reports_not_installed_running_and_the_pid() {
        let home = tempfile::tempdir().unwrap();
        let absent = Fake::new(false);
        let report = status(&machine(Manager::Launchd, home.path(), &absent)).unwrap();
        assert!(report.starts_with("not installed"));
        let present = Fake::new(true);
        let machine = machine(Manager::Launchd, home.path(), &present);
        install(&machine).unwrap();
        let report = status(&machine).unwrap();
        assert!(report.contains("loaded: yes"));
        assert!(report.contains("running, pid 4242, on http://127.0.0.1:4317"));
    }

    #[test]
    fn the_last_lines_keep_their_endings() {
        assert_eq!(last_lines("a\nb\nc\n", 2), "b\nc\n");
        assert_eq!(last_lines("a\n", 5), "a\n");
    }
}

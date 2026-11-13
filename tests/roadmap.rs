//! End-to-end tests of `ariane roadmap`: the binary run on a content
//! repository written to a temporary directory, with a configuration of its
//! own so that the machine's personal calendar is never read.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A content repository and an empty configuration directory.
struct Fixture {
    /// The repository: `.git/`, `tree/`, and `calendar.toml` when written.
    repo: tempfile::TempDir,
    /// The directory `XDG_CONFIG_HOME` points to.
    config: tempfile::TempDir,
}

impl Fixture {
    /// Node `p` with releases `1.0` and `2.0`, an epic and a story, a task of
    /// `p` and one of its child `p/q`, and its sibling `p/r` with a task.
    fn new() -> Fixture {
        let fixture = Fixture {
            repo: tempfile::tempdir().unwrap(),
            config: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(fixture.repo.path().join(".git")).unwrap();
        fixture
            .write(
                "tree/p/node.toml",
                "[[release]]\nname = \"1.0\"\n\n[[release]]\nname = \"2.0\"\n\n[estimates]\nXS = 1\nS = 2\nM = 4\nL = 8\nXL = 16\ntest_ratio = 0\n",
            )
            .write("tree/p/q/node.toml", "")
            .write("tree/p/r/node.toml", "")
            .write("tree/p/epics/E1-a.md", &item("p", "E1", "release: '1.0'\n"))
            .write("tree/p/stories/S1-a.md", &item("p", "S1", "parent: E1\n"))
            .write(
                "tree/p/tasks/T1-a.md",
                &item("p", "T1", "parent: S1\nsize: S\n"),
            )
            .write(
                "tree/p/q/tasks/T1-a.md",
                &item("p/q", "T1", "parent: p#S1\nsize: M\n"),
            )
            .write(
                "tree/p/r/tasks/T1-a.md",
                &item("p/r", "T1", "release: '2.0'\nsize: XS\n"),
            );
        fixture
    }

    /// Writes `text` at `relative` in the repository.
    fn write(&self, relative: &str, text: &str) -> &Fixture {
        let path = self.repo.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        self
    }

    /// Runs `ariane` with `arguments` from `directory`, relative to the
    /// repository.
    fn run_in(&self, directory: &str, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ariane"))
            .args(arguments)
            .current_dir(self.repo.path().join(directory))
            .env("XDG_CONFIG_HOME", self.config.path())
            .env_remove("ARIANE_REPO")
            .output()
            .unwrap()
    }

    /// Runs `ariane` with `arguments` at the root of the repository.
    fn run(&self, arguments: &[&str]) -> Output {
        self.run_in("", arguments)
    }
}

/// An item of `node` with `id` and extra header lines, to do.
fn item(node: &str, id: &str, extra: &str) -> String {
    format!(
        "---\nariane: 2\nnode: {node}\nid: {id}\ntitle: Title of {id}\nstatus: todo\n{extra}---\n# {id}\n"
    )
}

/// The standard output of a run that succeeded.
fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).unwrap()
}

/// The JSON a run printed.
fn json(output: &Output) -> serde_json::Value {
    serde_json::from_str(&stdout(output)).unwrap()
}

/// The path of the calendar of the repository.
fn calendar(root: &Path) -> PathBuf {
    root.join("calendar.toml")
}

#[test]
fn without_calendar_the_roadmap_reads_hours_from_now() {
    let fixture = Fixture::new();
    let text = stdout(&fixture.run(&["roadmap", "p"]));
    assert!(text.starts_with("Roadmap of p\n"), "{text}");
    assert!(
        text.contains("calendar  none on p or its ancestors"),
        "{text}"
    );
    assert!(text.contains("left      7 h of work"), "{text}");
    assert!(text.contains("      T1  Title of T1"), "{text}");
    assert!(text.contains("      q#T1  Title of T1"), "{text}");
    assert!(text.contains("+0 h → +2 h"), "{text}");
    assert!(text.contains("\n2.0 "), "{text}");
    let body = json(&fixture.run(&["roadmap", "p", "--json"]));
    assert_eq!(body["node"], "p");
    assert_eq!(body["dated"], false);
    assert_eq!(body["hours"], 7.0);
    let sequence: Vec<&str> = body["releases"][0]["epics"][0]["stories"][0]["work"]
        .as_array()
        .unwrap()
        .iter()
        .map(|work| work["reference"].as_str().unwrap())
        .collect();
    assert_eq!(sequence, ["p#T1", "p/q#T1"]);
}

#[test]
fn a_child_node_reads_its_part_of_the_carriers_dates() {
    let fixture = Fixture::new();
    fixture.write(
        "calendar.toml",
        "version = 1\ntimezone = \"Europe/Paris\"\n\n[node.\"p\"]\nblocks = [{ days = \"mon-sun\", from = \"00:00\", to = \"24:00\" }]\n",
    );
    assert!(calendar(fixture.repo.path()).exists());
    let text = stdout(&fixture.run(&["roadmap", "p/q"]));
    assert!(text.starts_with("Roadmap of p/q\n"), "{text}");
    assert!(
        text.contains(
            "calendar  the blocks of p, Europe/Paris, shared with p, p/r in one sequence"
        ),
        "{text}"
    );
    assert!(text.contains("\nfrom      "), "{text}");
    assert!(!text.contains("r#T1"), "{text}");
    let body = json(&fixture.run(&["roadmap", "p/q", "--json"]));
    assert_eq!(
        (body["node"].as_str(), body["carrier"].as_str()),
        (Some("p/q"), Some("p"))
    );
    assert_eq!(body["dated"], true);
    assert_eq!(body["hours"], 4.0);
    let work = &body["releases"][0]["epics"][0]["stories"][0]["work"];
    assert_eq!(work.as_array().unwrap().len(), 1);
    assert_eq!(work[0]["reference"], "p/q#T1");
    assert_eq!(work[0]["start"]["hours"], 2.0);
    assert!(work[0]["start"]["at"].is_string());
}

#[test]
fn the_node_defaults_to_the_current_directorys() {
    let fixture = Fixture::new();
    let text = stdout(&fixture.run_in("tree/p/r", &["roadmap"]));
    assert!(text.starts_with("Roadmap of p/r\n"), "{text}");
    let refused = fixture.run(&["roadmap"]);
    assert_eq!(refused.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("name a node"));
}

#[test]
fn an_unknown_node_or_release_is_refused() {
    let fixture = Fixture::new();
    let unknown = fixture.run(&["roadmap", "nope"]);
    assert_eq!(unknown.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("not a node"));
    let release = fixture.run(&["roadmap", "p", "--release", "9.9"]);
    assert_eq!(release.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&release.stderr).contains("1.0, 2.0"));
    let text = stdout(&fixture.run(&["roadmap", "p", "--release", "2.0"]));
    assert!(!text.contains("\n1.0 "), "{text}");
    assert!(text.contains("\n2.0 "), "{text}");
}

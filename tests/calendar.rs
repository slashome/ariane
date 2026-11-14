//! End-to-end tests of `ariane calendar set|unset` and `ariane estimates`: the
//! binary run on a content repository written to a temporary directory.

use std::process::{Command, Output};

/// A content repository with the nodes `p`, `p/c` and `q`; `p` prices `M` at
/// 10 hours. The configuration directory is empty, so the repository is the
/// personal one.
struct Fixture {
    repo: tempfile::TempDir,
    config: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Fixture {
        let fixture = Fixture {
            repo: tempfile::tempdir().unwrap(),
            config: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(fixture.repo.path().join(".git")).unwrap();
        fixture
            .write("tree/p/node.toml", "# p\n[estimates]\nM = 10\n")
            .write("tree/p/c/node.toml", "")
            .write("tree/q/node.toml", "");
        fixture
    }

    fn write(&self, relative: &str, text: &str) -> &Fixture {
        let path = self.repo.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        self
    }

    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.repo.path().join(relative)).unwrap()
    }

    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ariane"))
            .args(arguments)
            .current_dir(self.repo.path())
            .env("XDG_CONFIG_HOME", self.config.path())
            .env_remove("ARIANE_REPO")
            .output()
            .unwrap()
    }
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

/// The standard error of a run that failed.
fn refused(output: &Output) -> String {
    assert!(!output.status.success(), "{output:?}");
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn a_node_s_blocks_focus_and_days_off_are_set_then_its_table_removed() {
    let fixture = Fixture::new();
    let text = stdout(&fixture.run(&[
        "calendar",
        "set",
        "p",
        "--blocks",
        "mon-fri 09:00-12:00; sat 10:00-13:00",
        "--focus",
        "0.5",
        "--days-off",
        "2027-05-01",
    ]));
    assert!(text.contains("calendar.toml created"), "{text}");
    assert!(
        text.contains("18 h of blocks × focus 0.5 = 9 h of focused work"),
        "{text}"
    );
    let written = fixture.read("calendar.toml");
    assert!(
        written.ends_with(
            "[node.\"p\"]\nblocks = [\n  { days = \"mon-fri\", from = \"09:00\", to = \"12:00\" },\n  { days = \"sat\", from = \"10:00\", to = \"13:00\" },\n]\nfocus = 0.5\ndays_off = [\"2027-05-01\"]\n"
        ),
        "{written}"
    );
    stdout(&fixture.run(&["calendar", "set", "p", "--no-focus"]));
    assert!(!fixture.read("calendar.toml").contains("focus"));
    stdout(&fixture.run(&["calendar", "unset", "p"]));
    assert!(!fixture.read("calendar.toml").contains("[node."));
    assert!(fixture.run(&["doctor", "--all"]).status.success());
}

#[test]
fn an_overlap_is_refused_with_its_blocks_and_nothing_is_written() {
    let fixture = Fixture::new();
    stdout(&fixture.run(&["calendar", "set", "p", "--blocks", "mon-fri 09:00-12:00"]));
    let before = fixture.read("calendar.toml");
    let error = refused(&fixture.run(&["calendar", "set", "q", "--blocks", "wed 11:00-13:00"]));
    assert!(
        error.contains("error: the blocks `mon-fri 09:00–12:00` of `p`"),
        "{error}"
    );
    assert!(error.contains("not written"), "{error}");
    assert_eq!(fixture.read("calendar.toml"), before);
}

#[test]
fn globals_are_set_without_a_node_and_a_warning_does_not_stop_a_write() {
    let fixture = Fixture::new();
    let text = stdout(&fixture.run(&[
        "calendar",
        "set",
        "--timezone",
        "America/New_York",
        "--days-off",
        "2026-12-24..2027-01-02, 2027-05-01",
    ]));
    assert!(text.contains("timezone    America/New_York"), "{text}");
    let written = fixture.read("calendar.toml");
    assert!(
        written.contains("days_off = [\"2026-12-24..2027-01-02\", \"2027-05-01\"]"),
        "{written}"
    );
    let output = fixture.run(&["calendar", "set", "q", "--focus", "0.8"]);
    stdout(&output);
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("warning: [node.\"q\"] has no blocks")
    );
    assert!(
        refused(&fixture.run(&["calendar", "set", "--timezone", "Mars/Olympus"])).contains("IANA")
    );
    assert!(
        refused(&fixture.run(&["calendar", "set", "nowhere", "--focus", "0.5"]))
            .contains("not a node")
    );
    assert!(!refused(&fixture.run(&["calendar", "set", "p", "--timezone", "UTC"])).is_empty());
    assert!(!refused(&fixture.run(&["calendar", "set", "--focus", "0.5"])).is_empty());
    assert!(!refused(&fixture.run(&["calendar", "set", "p"])).is_empty());
}

#[test]
fn an_estimate_key_is_set_on_the_child_and_the_others_still_come_from_above() {
    let fixture = Fixture::new();
    let text = stdout(&fixture.run(&["estimates", "set", "p/c", "test_ratio", "0.1"]));
    assert!(text.contains("test_ratio  0.1      own"), "{text}");
    assert!(text.contains("M           10 h     from p"), "{text}");
    assert!(text.contains("XS          2 h      default"), "{text}");
    assert_eq!(
        fixture.read("tree/p/c/node.toml"),
        "[estimates]\ntest_ratio = 0.1\n"
    );
    let json: serde_json::Value =
        serde_json::from_str(&stdout(&fixture.run(&["estimates", "p/c", "--json"]))).unwrap();
    assert_eq!(json["estimates_from"]["M"], "p");
    assert_eq!(json["estimates_from"]["test_ratio"], "p/c");
    assert!(refused(&fixture.run(&["estimates", "set", "p", "M", "-2"])).contains("above zero"));
    assert!(refused(&fixture.run(&["estimates", "set", "p", "XXL", "3"])).contains("not a key"));
    stdout(&fixture.run(&["estimates", "unset", "p", "M"]));
    assert_eq!(fixture.read("tree/p/node.toml"), "");
    assert!(fixture.run(&["doctor", "--all"]).status.success());
}

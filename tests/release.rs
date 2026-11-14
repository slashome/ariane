//! End-to-end tests of `ariane release`: the binary run on a content
//! repository written to a temporary directory.

use std::process::{Command, Output};

/// A content repository: node `p` declaring `1.0` and `plus-tard`, a task of
/// `p` in `1.0`, and its child `p/c`, which inherits the list.
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
            .write(
                "tree/p/node.toml",
                "# p plans here\n[[release]]\nname = \"1.0\"\n\n[[release]]\nname = \"plus-tard\"\nparking = true\n",
            )
            .write("tree/p/tasks/T1-a.md", &item("p", "T1", "release: '1.0'\n"))
            .write("tree/p/tasks/T2-b.md", &item("p", "T2", ""))
            .write("tree/p/c/tasks/T1-c.md", &item("p/c", "T1", ""));
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

    fn run(&self, arguments: &[&str]) -> Output {
        self.run_in("", arguments)
    }
}

/// An item of `node` with `id` and extra header lines, to do.
fn item(node: &str, id: &str, extra: &str) -> String {
    format!(
        "---\nariane: 2\nnode: {node}\nid: {id}\ntitle: {id}\nstatus: todo\n{extra}---\n# {id}\n"
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

/// The standard error of a run that failed.
fn refused(output: &Output) -> String {
    assert!(!output.status.success());
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn the_list_is_written_in_place_and_printed_after_each_change() {
    let fixture = Fixture::new();
    let text = stdout(&fixture.run(&[
        "release",
        "add",
        "p",
        "0.9",
        "--top",
        "--target",
        "2027-01-15",
    ]));
    assert_eq!(
        text,
        "p: 0.9 (target 2027-01-15), 1.0, plus-tard (parking)\n"
    );
    let text = stdout(&fixture.run(&["release", "move", "p", "1.0", "--before", "0.9"]));
    assert_eq!(
        text,
        "p: 1.0, 0.9 (target 2027-01-15), plus-tard (parking)\n"
    );
    let text = stdout(&fixture.run(&["release", "rename", "p", "1.0", "mango"]));
    assert_eq!(
        text,
        "rewrote p#T1\np: mango, 0.9 (target 2027-01-15), plus-tard (parking)\n"
    );
    assert!(
        fixture
            .read("tree/p/tasks/T1-a.md")
            .contains("release: mango\n")
    );
    assert!(refused(&fixture.run(&["release", "remove", "p", "mango"])).contains("p#T1"),);
    let text = stdout(&fixture.run(&["release", "remove", "p", "0.9"]));
    assert_eq!(text, "p: mango, plus-tard (parking)\n");
    assert_eq!(
        fixture.read("tree/p/node.toml"),
        "# p plans here\n[[release]]\nname = \"mango\"\n\n[[release]]\nname = \"plus-tard\"\nparking = true\n"
    );
    assert!(fixture.run(&["doctor", "--all"]).status.success());
}

#[test]
fn a_release_is_marked_and_given_a_target_after_the_fact() {
    let fixture = Fixture::new();
    let text = stdout(&fixture.run(&[
        "release",
        "set",
        "p",
        "1.0",
        "--parking",
        "--target",
        "2027-02-01",
    ]));
    assert_eq!(
        text,
        "p: 1.0 (parking) (target 2027-02-01), plus-tard (parking)\n"
    );
    let text = stdout(&fixture.run(&["release", "set", "p", "1.0", "--no-parking", "--no-target"]));
    assert_eq!(text, "p: 1.0, plus-tard (parking)\n");
    assert_eq!(
        fixture.read("tree/p/node.toml"),
        "# p plans here\n[[release]]\nname = \"1.0\"\n\n[[release]]\nname = \"plus-tard\"\nparking = true\n"
    );
    assert!(
        !fixture
            .run(&["release", "set", "p", "1.0"])
            .status
            .success()
    );
    assert!(
        !fixture
            .run(&["release", "set", "p", "1.0", "--parking", "--no-parking"])
            .status
            .success()
    );
    assert!(
        refused(&fixture.run(&["release", "set", "p", "1.0", "--target", "soon"])).contains("soon")
    );
    assert!(
        refused(&fixture.run(&["release", "set", "p/c", "1.0", "--parking"]))
            .contains("`p/c` inherits its releases from `p`")
    );
    assert!(fixture.run(&["doctor", "--all"]).status.success());
}

#[test]
fn an_item_is_assigned_from_its_node_and_an_inherited_list_is_refused() {
    let fixture = Fixture::new();
    let text = stdout(&fixture.run_in("tree/p", &["release", "assign", "T2", "plus-tard"]));
    assert_eq!(text, "p#T2  release: plus-tard\n");
    let text = stdout(&fixture.run(&["release", "assign", "p/c#T1", "1.0"]));
    assert_eq!(text, "p/c#T1  release: 1.0\n");
    let text = stdout(&fixture.run(&["release", "assign", "p#T2", "--none"]));
    assert_eq!(text, "p#T2  release: none\n");
    assert!(
        refused(&fixture.run(&["release", "assign", "p#T2", "9.9"])).contains("1.0, plus-tard")
    );
    assert!(!fixture.run(&["release", "assign", "p#T2"]).status.success());
    let error = refused(&fixture.run(&["release", "add", "p/c", "2.0"]));
    assert!(
        error.contains("`p/c` inherits its releases from `p`"),
        "{error}"
    );
    assert!(fixture.run(&["doctor", "--all"]).status.success());
}

#[test]
fn a_release_is_closed_after_a_dry_run_and_its_changelog_printed_again() {
    let fixture = Fixture::new();
    fixture
        .write(
            "tree/p/node.toml",
            "[changelog]\nlanguage = \"fr\"\n\n[[release]]\nname = \"1.0\"\n\n[[release]]\nname = \"2.0\"\n\n[[release]]\nname = \"plus-tard\"\nparking = true\n",
        )
        .write(
            "tree/p/tasks/T3-c.md",
            &item("p", "T3", "release: '1.0'\ntype: bug\n").replace("status: todo", "status: done"),
        );
    let dry = fixture.run(&["release", "close", "p", "1.0", "--dry-run"]);
    assert_eq!(dry.status.code(), Some(2));
    let text = String::from_utf8(dry.stdout).unwrap();
    assert!(
        text.starts_with("p 1.0: cannot close yet (dry run, nothing written)\n"),
        "{text}"
    );
    assert!(
        text.contains("open work, refused; --carry-over moves it to 2.0:\n  T1  T1  (todo)\n"),
        "{text}"
    );
    assert!(
        text.contains(
            "would archive 1 item:\n  T3  tree/p/tasks/T3-c.md -> tree/p/archives/tasks/T3-c.md\n"
        ),
        "{text}"
    );
    assert!(
        text.ends_with("## 1.0\n\n### Corrections\n\n- **T3**. (T3)\n"),
        "{text}"
    );
    assert!(
        refused(&fixture.run(&["release", "close", "p", "1.0"]))
            .contains("still has open work: T1")
    );
    assert!(refused(&fixture.run(&["release", "close", "p", "plus-tard"])).contains("parking"));
    let text = stdout(&fixture.run(&["release", "close", "p", "1.0", "--carry-over"]));
    assert!(text.contains("open work, carried over to 2.0:"), "{text}");
    assert!(
        fixture
            .read("tree/p/tasks/T1-a.md")
            .contains("release: '2.0'\n")
    );
    assert!(
        fixture
            .read("tree/p/node.toml")
            .contains("name = \"1.0\"\nclosed = ")
    );
    let text = stdout(&fixture.run(&["release", "changelog", "p", "1.0"]));
    assert_eq!(text, "## 1.0\n\n### Corrections\n\n- **T3**. (T3)\n");
    let json: serde_json::Value = serde_json::from_str(&stdout(&fixture.run(&[
        "release",
        "changelog",
        "p",
        "1.0",
        "--json",
    ])))
    .unwrap();
    assert_eq!(json["language"], "fr");
    assert_eq!(json["sections"][0]["kind"], "fixed");
    assert!(refused(&fixture.run(&["release", "close", "p", "1.0"])).contains("already closed"));
    assert!(fixture.run(&["doctor", "--all"]).status.success());
}

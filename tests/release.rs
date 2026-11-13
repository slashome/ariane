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

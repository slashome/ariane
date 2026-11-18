//! End-to-end tests of the releases a release waits for (ADR-0030): the
//! binary run on a `cmc` tree whose repository nodes each declare their own
//! releases, sharing the owner's hours.

use std::process::{Command, Output};

/// A content repository and an empty configuration directory.
struct Fixture {
    /// The repository: `.git/` and `tree/`.
    repo: tempfile::TempDir,
    /// The directory `XDG_CONFIG_HOME` points to.
    config: tempfile::TempDir,
}

/// The estimates of `cmc`: sizes priced 1 to 16 hours, no testing.
const ESTIMATES: &str = "[estimates]\nXS = 1\nS = 2\nM = 4\nL = 8\nXL = 16\ntest_ratio = 0\n";

impl Fixture {
    /// `cmc` with `cmc-lib` declaring `1.0.0` and `1.1.0`, and
    /// `cmc-frontend/commercial` declaring `0.10.0` and `0.11.0`, the second
    /// declared with `commercial_next` as its extra lines; one task of size S
    /// in each release.
    fn new(commercial_next: &str) -> Fixture {
        let fixture = Fixture {
            repo: tempfile::tempdir().unwrap(),
            config: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(fixture.repo.path().join(".git")).unwrap();
        fixture
            .write("tree/cmc/node.toml", ESTIMATES)
            .write(
                "tree/cmc/cmc-lib/node.toml",
                "[[release]]\nname = \"1.0.0\"\n\n[[release]]\nname = \"1.1.0\"\n",
            )
            .write(
                "tree/cmc/cmc-frontend/commercial/node.toml",
                &format!(
                    "[[release]]\nname = \"0.10.0\"\n\n[[release]]\nname = \"0.11.0\"\n{commercial_next}"
                ),
            )
            .write(
                "tree/cmc/cmc-lib/tasks/T1-a.md",
                &task("cmc/cmc-lib", "T1", "1.0.0"),
            )
            .write(
                "tree/cmc/cmc-lib/tasks/T2-b.md",
                &task("cmc/cmc-lib", "T2", "1.1.0"),
            )
            .write(
                "tree/cmc/cmc-frontend/commercial/tasks/T1-a.md",
                &task("cmc/cmc-frontend/commercial", "T1", "0.10.0"),
            )
            .write(
                "tree/cmc/cmc-frontend/commercial/tasks/T2-b.md",
                &task("cmc/cmc-frontend/commercial", "T2", "0.11.0"),
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

    /// Runs `ariane` with `arguments` at the root of the repository.
    fn run(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ariane"))
            .args(arguments)
            .current_dir(self.repo.path())
            .env("XDG_CONFIG_HOME", self.config.path())
            .env_remove("ARIANE_REPO")
            .output()
            .unwrap()
    }

    /// The `node.toml` of `node`.
    fn node_toml(&self, node: &str) -> String {
        std::fs::read_to_string(self.repo.path().join("tree").join(node).join("node.toml")).unwrap()
    }
}

/// A task of `node` with `id`, to do, of size S, in `release`.
fn task(node: &str, id: &str, release: &str) -> String {
    format!(
        "---\nariane: 2\nnode: {node}\nid: {id}\ntitle: Title of {id}\nstatus: todo\nrelease: '{release}'\nsize: S\n---\n# {id}\n"
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

/// `(declared_by, name, depends_on)` of each release of a roadmap.
fn releases(roadmap: &serde_json::Value) -> Vec<(String, String, Vec<String>)> {
    roadmap["releases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|release| {
            (
                release["declared_by"].as_str().unwrap().to_owned(),
                release["name"].as_str().unwrap().to_owned(),
                release["depends_on"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|reference| reference.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect()
}

/// The `(start, end)` hours of the release `name` of a roadmap.
fn span(roadmap: &serde_json::Value, name: &str) -> (f64, f64) {
    let release = roadmap["releases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|release| release["name"] == name)
        .unwrap();
    (
        release["span"]["start"]["hours"].as_f64().unwrap(),
        release["span"]["end"]["hours"].as_f64().unwrap(),
    )
}

/// The checks of the errors `ariane doctor --json` finds, with its exit code.
fn doctor(fixture: &Fixture) -> (Option<i32>, Vec<(String, String)>) {
    let output = fixture.run(&["doctor", "--json"]);
    let findings: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let errors = findings
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["severity"] == "error")
        .map(|finding| {
            (
                finding["check"].as_str().unwrap().to_owned(),
                finding["message"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    (output.status.code(), errors)
}

#[test]
fn without_dependencies_releases_interleave_by_place_then_node_path() {
    let fixture = Fixture::new("");
    let roadmap = json(&fixture.run(&["roadmap", "cmc", "--json"]));
    let order: Vec<(String, String)> = releases(&roadmap)
        .into_iter()
        .map(|(node, name, _)| (node, name))
        .collect();
    assert_eq!(
        order,
        [
            (
                "cmc/cmc-frontend/commercial".to_owned(),
                "0.10.0".to_owned()
            ),
            ("cmc/cmc-lib".to_owned(), "1.0.0".to_owned()),
            (
                "cmc/cmc-frontend/commercial".to_owned(),
                "0.11.0".to_owned()
            ),
            ("cmc/cmc-lib".to_owned(), "1.1.0".to_owned()),
        ]
    );
    assert_eq!(span(&roadmap, "0.11.0"), (4.0, 6.0));
}

#[test]
fn a_release_that_depends_on_another_node_s_release_is_scheduled_after_it() {
    let fixture = Fixture::new("depends_on = [\"cmc/cmc-lib@1.1.0\"]\n");
    let (code, errors) = doctor(&fixture);
    assert_eq!((code, errors), (Some(0), Vec::new()));
    let roadmap = json(&fixture.run(&["roadmap", "cmc", "--json"]));
    assert_eq!(
        releases(&roadmap),
        [
            (
                "cmc/cmc-frontend/commercial".to_owned(),
                "0.10.0".to_owned(),
                Vec::new()
            ),
            ("cmc/cmc-lib".to_owned(), "1.0.0".to_owned(), Vec::new()),
            ("cmc/cmc-lib".to_owned(), "1.1.0".to_owned(), Vec::new()),
            (
                "cmc/cmc-frontend/commercial".to_owned(),
                "0.11.0".to_owned(),
                vec!["cmc/cmc-lib@1.1.0".to_owned()]
            ),
        ]
    );
    let (_, library_end) = span(&roadmap, "1.1.0");
    let (commercial_start, _) = span(&roadmap, "0.11.0");
    assert_eq!((library_end, commercial_start), (6.0, 6.0));
    let text = stdout(&fixture.run(&["roadmap", "cmc"]));
    let library = text.find("\n1.1.0 ").unwrap();
    let commercial = text.find("\n0.11.0  (after cmc/cmc-lib@1.1.0)").unwrap();
    assert!(library < commercial, "{text}");
}

#[test]
fn a_dependency_on_an_unknown_node_or_release_is_an_error_for_a_human() {
    let fixture = Fixture::new("depends_on = [\"cmc/cmc-api@1.0.0\", \"cmc/cmc-lib@9.9.9\"]\n");
    let (code, errors) = doctor(&fixture);
    assert_eq!(code, Some(2));
    let checks: Vec<&str> = errors.iter().map(|(check, _)| check.as_str()).collect();
    assert_eq!(
        checks,
        ["unknown_release_dependency", "unknown_release_dependency"]
    );
    assert!(
        errors[0].1.contains("`cmc/cmc-api` is not a node"),
        "{errors:?}"
    );
    assert!(
        errors[1].1.contains("declares no release `9.9.9`"),
        "{errors:?}"
    );
}

#[test]
fn a_cycle_of_releases_is_an_error_naming_its_path() {
    let fixture = Fixture::new("depends_on = [\"cmc/cmc-lib@1.1.0\"]\n");
    fixture.write(
        "tree/cmc/cmc-lib/node.toml",
        "[[release]]\nname = \"1.0.0\"\n\n[[release]]\nname = \"1.1.0\"\ndepends_on = [\"cmc/cmc-frontend/commercial@0.11.0\"]\n",
    );
    let (code, errors) = doctor(&fixture);
    assert_eq!(code, Some(2));
    assert_eq!(
        errors,
        [(
            "release_dependency_cycle".to_owned(),
            "releases wait for each other, so none can be delivered: cmc/cmc-frontend/commercial@0.11.0 -> cmc/cmc-lib@1.1.0 -> cmc/cmc-frontend/commercial@0.11.0".to_owned()
        )]
    );
    let roadmap = json(&fixture.run(&["roadmap", "cmc", "--json"]));
    assert_eq!(roadmap["releases"].as_array().unwrap().len(), 4);
    let cycle = roadmap["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|warning| warning["kind"] == "dependency_cycle")
        .unwrap();
    assert_eq!(cycle["items"][0]["reference"], "release 0.11.0");
}

#[test]
fn release_set_writes_and_clears_dependencies_and_refuses_a_cycle() {
    let fixture = Fixture::new("");
    let commercial = "cmc/cmc-frontend/commercial";
    stdout(&fixture.run(&[
        "release",
        "set",
        commercial,
        "0.11.0",
        "--depends-on",
        "cmc/cmc-lib@1.1.0",
        "--depends-on",
        "@0.10.0",
    ]));
    assert_eq!(
        fixture.node_toml(commercial),
        "[[release]]\nname = \"0.10.0\"\n\n[[release]]\nname = \"0.11.0\"\ndepends_on = [\"cmc/cmc-lib@1.1.0\", \"@0.10.0\"]\n"
    );
    let refused = fixture.run(&[
        "release",
        "set",
        "cmc/cmc-lib",
        "1.1.0",
        "--depends-on",
        "cmc/cmc-frontend/commercial@0.11.0",
    ]);
    assert!(!refused.status.success());
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("would wait for each other"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    stdout(&fixture.run(&["release", "set", commercial, "0.11.0", "--no-depends-on"]));
    assert_eq!(
        fixture.node_toml(commercial),
        "[[release]]\nname = \"0.10.0\"\n\n[[release]]\nname = \"0.11.0\"\n"
    );
}

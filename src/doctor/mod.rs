//! `ariane doctor`: checks every item and node against SPEC.md §1.6.
//!
//! Doctor only observes (ADR-0004): there is no write path anywhere in this
//! module or in `check`. It prints its findings grouped by node, or as JSON for
//! agents and scripts, and its exit code says how much human involvement the
//! repairs need:
//!
//! - `0` no error (warnings alone do not fail, unless `--strict`);
//! - `1` every error is repaired by a command, `ariane migrate`;
//! - `2` at least one error needs a human decision;
//! - `3` the diagnosis could not run at all (returned by `main`).

/// The checks themselves.
mod check;
/// The types of a finding and of its remedy.
mod finding;

use std::collections::BTreeMap;
use std::process::ExitCode;

use anyhow::Result;

use crate::cli::DoctorArgs;
use crate::tree::{NodePath, Tree};
use finding::{Finding, Remedy, Severity};

/// Runs `ariane doctor`: diagnoses the scope, prints the report and returns the
/// exit code.
///
/// # Errors
///
/// Fails only if the findings cannot be serialized to JSON.
pub fn run(tree: &Tree, args: &DoctorArgs, scope: Option<&NodePath>) -> Result<ExitCode> {
    let findings = diagnose(tree, scope, args.strict);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&findings)?);
    } else {
        print_report(&findings);
    }
    Ok(ExitCode::from(exit_code(&findings)))
}

/// Collects the findings of `scope`, promoting every warning to an error under `--strict`.
pub(crate) fn diagnose(tree: &Tree, scope: Option<&NodePath>, strict: bool) -> Vec<Finding> {
    let mut findings = check::findings(tree, scope);
    if strict {
        for finding in &mut findings {
            finding.severity = Severity::Error;
        }
    }
    findings
}

/// The exit code for these findings (see the module documentation).
///
/// `peekable()` lets the function look at the first error without consuming it,
/// so the same iterator can then be checked with `all`.
fn exit_code(findings: &[Finding]) -> u8 {
    let mut errors = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .peekable();
    if errors.peek().is_none() {
        0
    } else if errors.all(|finding| finding.remedy.is_command()) {
        1
    } else {
        2
    }
}

/// Prints the findings grouped by node. Files that only need `ariane migrate`
/// are folded into one line per node, so that a tree still on the legacy format
/// does not bury the other findings.
fn print_report(findings: &[Finding]) {
    let mut by_node: BTreeMap<String, Vec<&Finding>> = BTreeMap::new();
    for finding in findings {
        let node = finding
            .node
            .as_ref()
            .map_or_else(|| "(tree)".to_owned(), ToString::to_string);
        by_node.entry(node).or_default().push(finding);
    }
    for (node, findings) in &by_node {
        println!("{node}");
        let (migrate, others): (Vec<&&Finding>, Vec<&&Finding>) =
            findings.iter().partition(|finding| finding.needs_migrate());
        if let Some(first) = migrate.first() {
            println!(
                "  {:7}  {} item{} without a current versioned header",
                first.severity.label(),
                migrate.len(),
                if migrate.len() == 1 { "" } else { "s" },
            );
            print_remedy(&first.remedy);
        }
        for finding in others {
            match &finding.path {
                Some(path) => println!(
                    "  {:7}  {path}: {}",
                    finding.severity.label(),
                    finding.message
                ),
                None => println!("  {:7}  {}", finding.severity.label(), finding.message),
            }
            print_remedy(&finding.remedy);
        }
    }
    println!("{}", summary(findings));
}

/// Prints the remedy line under a finding.
fn print_remedy(remedy: &Remedy) {
    match remedy {
        Remedy::Command { command } => println!("           → {command}"),
        Remedy::Human { resolutions } => {
            println!("           → either: {}", resolutions.join(" | "))
        }
    }
}

/// The closing line: errors, warnings, nodes concerned and files needing migration.
fn summary(findings: &[Finding]) -> String {
    if findings.is_empty() {
        return "doctor: healthy".to_owned();
    }
    let errors = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .count();
    let warnings = findings.len() - errors;
    let migrate = findings
        .iter()
        .filter(|finding| finding.needs_migrate())
        .count();
    let nodes = findings
        .iter()
        .map(|finding| finding.node.as_ref())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    format!(
        "doctor: {errors} error{}, {warnings} warning{} across {nodes} node{}; {migrate} item{} need `ariane migrate`",
        plural(errors),
        plural(warnings),
        plural(nodes),
        plural(migrate),
    )
}

/// `"s"` unless `count` is one.
fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// Unit tests of every SPEC.md §1.6 row, of scoping, `--strict` and exit codes.
#[cfg(test)]
mod tests {
    use super::finding::Check;
    use super::*;
    use crate::tree::fixture::Repo;

    /// A `node.toml` declaring custom fields and a pharmakon-like workflow.
    const WORKFLOW: &str = r#"
[fields.mod]
type = "string"
required = true

[fields.tests]
type = "list<string>"

[fields.type]
type = "enum"
values = ["code", "test"]

[[workflow]]
id = "a-faire"
label = "À faire"
category = "todo"

[[workflow]]
id = "valide"
label = "Validé"
category = "done"
"#;

    /// The text of a version-1 task of `node` with id `id`, plus `extra` header lines.
    fn task(node: &str, id: &str, extra: &str) -> String {
        format!(
            "---\nariane: 1\nnode: {node}\nid: {id}\ntitle: Title {id}\nstatus: a-faire\nmod: M\n{extra}---\n# {id}\n"
        )
    }

    /// A repository with one node, its `node.toml` and its `INDEX.md`.
    fn base() -> Repo {
        let repo = Repo::new();
        repo.write("p/INDEX.md", "# p\n")
            .write("p/node.toml", WORKFLOW);
        repo
    }

    /// The checks that fire on the whole repository, sorted.
    fn checks(repo: &Repo) -> Vec<Check> {
        let tree = Tree::load(repo.root()).unwrap();
        diagnose(&tree, None, false)
            .into_iter()
            .map(|finding| finding.check)
            .collect()
    }

    /// Asserts that exactly one check fires, and that it is `expected`.
    fn only(repo: &Repo, expected: Check) {
        assert_eq!(checks(repo), vec![expected]);
    }

    #[test]
    fn a_conforming_tree_is_healthy() {
        let repo = base();
        repo.write("p/tasks/T1-a.md", &task("p", "T1", "tests: [a]\ntype: code\nparent: T2\n"))
            .write("p/tasks/T2-b.md", &task("p", "T2", "depends_on: [p#T1]\nsize: XS\nimpact: high\npriority: low\ncreated: 2026-10-20\nrelease: \"1.0\"\nexternal: [monday:12]\nrefs: [ADR-0008]\n"))
            .write("p/reflexions/R1-r.md", "---\nariane: 1\nnode: p\nid: R1\ntitle: R\nstatus: open\nmod: M\n---\n")
            .write("p/ressources/S1-s.md", "---\nariane: 1\nnode: p\nid: S1\ntitle: S\nmod: M\n---\n")
            .write("p/archives/tasks/T3-c.md", &task("p", "T3", "").replace("a-faire", "valide"));
        assert_eq!(checks(&repo), vec![]);
        let tree = Tree::load(repo.root()).unwrap();
        assert_eq!(exit_code(&diagnose(&tree, None, false)), 0);
    }

    #[test]
    fn an_item_of_the_home_node_carries_an_empty_node() {
        let repo = Repo::new();
        repo.write(
            "reflexions/R1-r.md",
            "---\nariane: 1\nnode: ''\nid: R1\ntitle: R\nstatus: open\n---\n",
        );
        assert_eq!(checks(&repo), vec![]);
    }

    #[test]
    fn no_front_matter_needs_migrate_and_nothing_else() {
        let repo = base();
        repo.write("p/tasks/x.md", "# x\n\n- **État** : ouvert\n");
        only(&repo, Check::NoHeader);
    }

    #[test]
    fn a_front_matter_without_ariane_needs_migrate() {
        let repo = base();
        repo.write("p/tasks/x.md", "---\nnode: p\ntitre: x\n---\n");
        only(&repo, Check::NoHeader);
    }

    #[test]
    fn an_older_schema_needs_migrate() {
        let repo = base();
        repo.write("p/tasks/x.md", "---\nariane: 0\n---\n");
        only(&repo, Check::OutdatedSchema);
    }

    #[test]
    fn a_newer_schema_means_the_cli_is_outdated() {
        let repo = base();
        repo.write("p/tasks/x.md", "---\nariane: 9\n---\n");
        only(&repo, Check::FutureSchema);
    }

    #[test]
    fn a_malformed_front_matter_is_reported() {
        let repo = base();
        repo.write("p/tasks/x.md", "---\n: [\n---\n");
        only(&repo, Check::MalformedHeader);
    }

    #[test]
    fn a_missing_required_core_key_is_reported() {
        let repo = base();
        repo.write(
            "p/tasks/T1-a.md",
            "---\nariane: 1\nnode: p\nid: T1\nstatus: a-faire\nmod: M\n---\n",
        );
        only(&repo, Check::MissingKey);
    }

    #[test]
    fn a_node_that_does_not_match_the_location_is_reported() {
        let repo = base();
        repo.write("p/tasks/T1-a.md", &task("q", "T1", ""));
        only(&repo, Check::NodeMismatch);
    }

    #[test]
    fn an_invalid_id_is_reported() {
        let repo = base();
        repo.write("p/tasks/x.md", &task("p", "1-bad", ""));
        only(&repo, Check::InvalidId);
    }

    #[test]
    fn a_duplicated_id_is_reported_on_each_file() {
        let repo = base();
        repo.write("p/tasks/T1-a.md", &task("p", "T1", ""))
            .write("p/tasks/T1-b.md", &task("p", "T1", ""));
        assert_eq!(checks(&repo), vec![Check::DuplicateId, Check::DuplicateId]);
    }

    #[test]
    fn a_status_outside_the_workflow_is_reported() {
        let repo = base();
        repo.write(
            "p/tasks/T1-a.md",
            &task("p", "T1", "").replace("a-faire", "todo"),
        );
        only(&repo, Check::UnknownStatus);
    }

    #[test]
    fn a_reflexion_only_accepts_open_or_settled() {
        let repo = base();
        repo.write(
            "p/reflexions/R1-r.md",
            "---\nariane: 1\nnode: p\nid: R1\ntitle: R\nstatus: a-faire\nmod: M\n---\n",
        );
        only(&repo, Check::UnknownStatus);
    }

    #[test]
    fn a_file_name_must_begin_with_the_id_and_a_slug() {
        let repo = base();
        repo.write("p/tasks/T1-a.md", &task("p", "T1", ""));
        assert_eq!(checks(&repo), vec![]);
        repo.write("p/tasks/alleger-la-doc.md", &task("p", "T2", ""));
        only(&repo, Check::FileName);
        let tree = Tree::load(repo.root()).unwrap();
        let finding = diagnose(&tree, None, false).remove(0);
        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(
            finding.remedy,
            Remedy::Command {
                command: "git mv tree/p/tasks/alleger-la-doc.md tree/p/tasks/T2-alleger-la-doc.md"
                    .to_owned()
            }
        );
    }

    #[test]
    fn a_file_named_by_its_bare_id_or_another_id_is_reported() {
        let repo = base();
        repo.write("p/tasks/T1.md", &task("p", "T1", ""));
        only(&repo, Check::FileName);
        let tree = Tree::load(repo.root()).unwrap();
        assert!(!diagnose(&tree, None, false)[0].remedy.is_command());
        let repo = base();
        repo.write("p/tasks/T10-a.md", &task("p", "T1", ""));
        only(&repo, Check::FileName);
    }

    #[test]
    fn a_reflexion_on_trial_is_live_but_not_closed() {
        let repo = base();
        let reflexion = "---\nariane: 1\nnode: p\nid: R1\ntitle: R\nstatus: trial\nmod: M\n---\n";
        repo.write("p/reflexions/R1-r.md", reflexion);
        assert_eq!(checks(&repo), vec![]);
        repo.write(
            "p/archives/reflexions/R2-r.md",
            &reflexion.replace("R1", "R2"),
        );
        only(&repo, Check::OpenInArchives);
    }

    #[test]
    fn core_keys_of_the_wrong_type_are_reported() {
        let repo = base();
        repo.write(
            "p/tasks/T1-a.md",
            &task(
                "p",
                "T1",
                "release: 1.0\nrefs: ADR-0008\nexternal: [Monday 12]\n",
            ),
        );
        assert_eq!(checks(&repo), vec![Check::WrongType; 3]);
    }

    #[test]
    fn a_status_on_a_ressource_is_reported() {
        let repo = base();
        repo.write(
            "p/ressources/S1-s.md",
            "---\nariane: 1\nnode: p\nid: S1\ntitle: S\nstatus: open\nmod: M\n---\n",
        );
        only(&repo, Check::ForbiddenKey);
    }

    #[test]
    fn an_open_item_in_archives_is_reported() {
        let repo = base();
        repo.write("p/archives/tasks/T1-a.md", &task("p", "T1", ""))
            .write(
                "p/archives/reflexions/R1-r.md",
                "---\nariane: 1\nnode: p\nid: R1\ntitle: R\nstatus: open\nmod: M\n---\n",
            );
        assert_eq!(
            checks(&repo),
            vec![Check::OpenInArchives, Check::OpenInArchives]
        );
    }

    #[test]
    fn a_dangling_reference_is_reported() {
        let repo = base();
        repo.write(
            "p/tasks/T1-a.md",
            &task("p", "T1", "parent: T9\ndepends_on: [q#T1]\n"),
        );
        assert_eq!(
            checks(&repo),
            vec![Check::DanglingReference, Check::DanglingReference]
        );
    }

    #[test]
    fn values_outside_their_vocabulary_or_format_are_reported() {
        let repo = base();
        repo.write(
            "p/tasks/T1-a.md",
            &task(
                "p",
                "T1",
                "size: XXL\nimpact: huge\npriority: 🔴\ncreated: 17/08/2026\n",
            ),
        );
        assert_eq!(checks(&repo), vec![Check::OutOfVocabulary; 4]);
    }

    #[test]
    fn a_custom_key_of_the_wrong_type_is_reported() {
        let repo = base();
        repo.write(
            "p/tasks/T1-a.md",
            &task("p", "T1", "tests: one\ntype: design\n"),
        );
        assert_eq!(checks(&repo), vec![Check::WrongType, Check::WrongType]);
    }

    #[test]
    fn a_required_custom_key_missing_is_reported() {
        let repo = base();
        repo.write(
            "p/tasks/T1-a.md",
            &task("p", "T1", "").replace("mod: M\n", ""),
        );
        only(&repo, Check::MissingKey);
    }

    #[test]
    fn an_undeclared_custom_key_is_a_warning() {
        let repo = base();
        repo.write("p/tasks/T1-a.md", &task("p", "T1", "owner: me\n"));
        let tree = Tree::load(repo.root()).unwrap();
        let findings = diagnose(&tree, None, false);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].check, Check::UndeclaredKey);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(exit_code(&findings), 0);
        assert_eq!(exit_code(&diagnose(&tree, None, true)), 2);
    }

    #[test]
    fn an_invalid_node_toml_and_a_node_without_index_are_reported() {
        let repo = Repo::new();
        repo.write("q/node.toml", "[[workflow]]\nid = 3\n").write(
            "q/tasks/T1-a.md",
            &task("q", "T1", "").replace("a-faire", "todo"),
        );
        let found = checks(&repo);
        assert!(found.contains(&Check::InvalidNodeToml));
        assert!(found.contains(&Check::MissingIndex));
    }

    #[test]
    fn exit_code_is_one_when_every_error_is_repaired_by_migrate() {
        let repo = base();
        repo.write("p/tasks/a.md", "# a\n")
            .write("p/tasks/b.md", "---\nariane: 0\n---\n");
        let tree = Tree::load(repo.root()).unwrap();
        assert_eq!(exit_code(&diagnose(&tree, None, false)), 1);
    }

    #[test]
    fn exit_code_is_two_when_a_human_decision_is_needed() {
        let repo = base();
        repo.write("p/tasks/a.md", "# a\n")
            .write("p/tasks/T1-a.md", &task("q", "T1", ""));
        let tree = Tree::load(repo.root()).unwrap();
        assert_eq!(exit_code(&diagnose(&tree, None, false)), 2);
    }

    #[test]
    fn the_scope_limits_the_diagnosis_to_a_node_and_its_descendants() {
        let repo = base();
        repo.write("p/tasks/a.md", "# a\n")
            .write("p/child/INDEX.md", "# c\n")
            .write("p/child/tasks/b.md", "# b\n")
            .write("r/INDEX.md", "# r\n")
            .write("r/tasks/c.md", "# c\n");
        let tree = Tree::load(repo.root()).unwrap();
        let scoped = diagnose(&tree, Some(&NodePath::new("p")), false);
        let nodes: Vec<_> = scoped
            .iter()
            .map(|finding| finding.node.as_ref().unwrap().as_str().to_owned())
            .collect();
        assert_eq!(nodes, vec!["p/child", "p"]);
    }

    #[test]
    fn every_human_remedy_enumerates_at_least_one_resolution() {
        let repo = base();
        repo.write(
            "p/tasks/T1-a.md",
            &task("q", "1x", "size: XXL\nowner: me\nparent: T9\n"),
        );
        let tree = Tree::load(repo.root()).unwrap();
        for finding in diagnose(&tree, None, false) {
            if let Remedy::Human { resolutions } = &finding.remedy {
                assert!(!resolutions.is_empty(), "{finding:?}");
            }
        }
    }
}

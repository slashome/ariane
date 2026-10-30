//! Entry point of the `ariane` command-line tool.
//!
//! `ariane` manages an Ariane content repository: a git repository whose `tree/`
//! directory mirrors the user's projects as nodes, each holding items (tasks,
//! stories, reflexions, ressources) as Markdown files with a YAML
//! front-matter (SPEC.md §1, ADR-0008).
//!
//! This file only wires things together: it parses the command line ([`cli`]),
//! finds the content repository ([`repo`]), loads it into memory ([`tree`]) and
//! hands it to the module that implements the requested command:
//!
//! - `ariane tasks` → [`tasks`], lists items;
//! - `ariane search` → [`search`], finds items by header and body;
//! - `ariane show` → [`show`], one item with its body and relations ([`relations`]);
//! - `ariane doctor` → [`doctor`], checks items without writing (ADR-0004);
//! - `ariane migrate` → [`migrate`], upgrades items to the current schema (ADR-0009);
//! - `ariane schema` → [`schema`], prints the header schema; it needs no repository;
//! - `ariane serve` → [`serve`], serves the web interface (ADR-0012), and pulls the content once;
//! - `ariane pull` → [`serve::git`], fast-forwards the content to its upstream;
//! - `ariane proxy` → [`proxy`], local names for the interface and the projects (ADR-0016).
//!
//! Exit code `3` means the command could not run at all (ADR-0004); the other
//! codes are chosen by each command.

/// Command-line arguments, declared with `clap`.
mod cli;
/// `config.toml` and `ariane config`.
mod config;
/// `ariane doctor`: validation of items and nodes.
mod doctor;
/// `ariane fix`: the deterministic repairs of `doctor` findings.
mod fix;
/// `ariane focus`: the item an agent session works on.
mod focus;
/// When each item file last changed, according to git.
mod history;
/// `ariane hook`: what Ariane does inside an agent's harness.
mod hook;
/// One item file: its front-matter and its body.
mod item;
/// `ariane links`: pull requests found on GitHub, recorded in their items.
mod links;
/// The logo printed above the help.
mod logo;
/// `ariane migrate`: schema migrations of item files.
mod migrate;
/// `ariane sync monday`: the monday connector.
mod monday;
/// `node.toml`: custom fields and status workflows of a node.
mod node;
/// `ariane people`: whom items may wait for.
mod people;
/// The personas a story is written for, as the interface edits them.
mod personas;
/// `ariane proxy`: local names for the interface and the projects' servers.
mod proxy;
/// Parents, children, dependencies and dependents of items.
mod relations;
/// Discovery of the content repository on disk.
mod repo;
/// The core header schema, declared once as data.
mod schema;
/// `ariane search`: items found by header and body.
mod search;
/// `secrets.toml`: the trackers' tokens.
mod secrets;
mod serve;
/// `ariane service`: the server as a background service.
mod service;
/// `settings.toml`: the preferences versioned with the content.
mod settings;
/// `ariane show`: one item with its header, body and relations.
mod show;
/// `ariane tasks`: the list of items.
mod tasks;
/// The whole content repository loaded in memory.
mod tree;
/// The branches and pull requests of items.
mod work;

use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches};

/// Runs the command and turns its outcome into the process exit code.
///
/// Any error that escapes a command is printed on stderr with its full chain of
/// causes (`{error:#}` asks `anyhow` for every context layer) and exits with `3`.
fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ariane: {error:#}");
            ExitCode::from(3)
        }
    }
}

/// Parses the arguments and dispatches to the requested command.
///
/// `ariane schema`, `ariane config` and `ariane service` are answered before
/// looking for a repository: the schema is built into the binary, and the other
/// two work on the configuration and the machine, not on content. `ariane pull`
/// needs the repository but not the tree, which it is about to change. Every
/// other command first discovers the content repository and loads the whole tree once.
///
/// # Errors
///
/// Fails when no content repository can be found or read, or when the command
/// itself fails.
fn run() -> Result<ExitCode> {
    let cli = parse();
    match &cli.command {
        cli::Command::Schema(args) => return schema_command(args),
        cli::Command::Config(args) => return config::command::run(args, cli.repo.as_deref()),
        cli::Command::Service(args) => return service::run(args),
        cli::Command::Pull => return pull_command(&repo::discover(cli.repo.as_deref())?.root),
        cli::Command::Proxy(cli::ProxyArgs {
            command: Some(cli::ProxyCommand::Teardown),
            ..
        }) => return proxy::teardown(),
        cli::Command::Hook(args) => {
            return Ok(match args.harness {
                cli::HookHarness::ClaudeCode => hook::run(),
            });
        }
        cli::Command::People(cli::PeopleArgs {
            command: Some(cli::PeopleCommand::Find { account, name }),
            json,
            ..
        }) => return monday::find_users(account, name, *json),
        cli::Command::Connect(args) => {
            return match args.connector {
                cli::Connector::Monday => {
                    let known = repo::discover(cli.repo.as_deref())
                        .and_then(|location| tree::Tree::load(&location.root))
                        .map(|tree| monday::accounts(&tree))
                        .unwrap_or_default();
                    monday::connect(&args.account, &known)
                }
            };
        }
        _ => {}
    }
    let location = repo::discover(cli.repo.as_deref())?;
    let tree = tree::Tree::load(&location.root)?;
    let here = location.here.as_ref();
    match cli.command {
        cli::Command::Tasks(args) => {
            let scope = scope(args.node.as_deref(), args.all, here);
            tasks::run(&tree, &args, scope.as_ref())
        }
        cli::Command::Search(args) => {
            let scope = scope(args.node.as_deref(), args.all, here);
            search::run(&tree, &args, scope.as_ref())
        }
        cli::Command::Show(args) => show::run(&tree, &args, here.filter(|node| !node.is_home())),
        cli::Command::Settings(args) => settings::run(&location.root, &args),
        cli::Command::Focus(args) => focus::run(&tree, &args, here.filter(|node| !node.is_home())),
        cli::Command::Links(args) => {
            let scope = scope(args.node.as_deref(), args.all, here);
            links::run(&tree, &args, scope.as_ref())
        }
        cli::Command::Hook(_) | cli::Command::Connect(_) | cli::Command::Pull => {
            unreachable!("answered before loading the tree")
        }
        cli::Command::People(args) => {
            let scope = args.node.as_deref().map(tree::NodePath::new);
            people::run(&tree, &args, scope.as_ref())
        }
        cli::Command::Sync(args) => {
            let scope = scope(args.node.as_deref(), args.all, here);
            match args.connector {
                cli::Connector::Monday => monday::run(&tree, &args, scope.as_ref()),
            }
        }
        cli::Command::Doctor(args) => {
            let scope = scope(args.node.as_deref(), args.all, here);
            doctor::run(&tree, &args, scope.as_ref())
        }
        cli::Command::Migrate(args) => {
            let scope = scope(args.node.as_deref(), args.all, here);
            migrate::run(&tree, &args, scope.as_ref())
        }
        cli::Command::Fix(args) => {
            let scope = scope(args.node.as_deref(), args.all, here);
            fix::run(&tree, &args, scope.as_ref())
        }
        cli::Command::Schema(args) => schema_command(&args),
        cli::Command::Serve(args) => serve::run(location.root, tree, &args),
        cli::Command::Proxy(args) => proxy::run(&tree, &args),
        cli::Command::Config(args) => config::command::run(&args, cli.repo.as_deref()),
        cli::Command::Service(args) => service::run(&args),
    }
}

/// Parses the command line. When it asks for the help, or names no command,
/// the help opens on the logo in the content's theme color; the repository
/// is looked for only then.
fn parse() -> cli::Cli {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let helps = arguments.is_empty()
        || arguments.first().is_some_and(|first| first == "help")
        || arguments
            .iter()
            .any(|argument| argument == "-h" || argument == "--help");
    let mut command = cli::Cli::command();
    if helps {
        // The banner carries the tagline: `about` would repeat it.
        command = command
            .before_help(logo::banner(&logo::theme_color()))
            .help_template("{before-help}{usage-heading} {usage}\n\n{all-args}");
    }
    cli::Cli::from_arg_matches(&command.get_matches()).unwrap_or_else(|error| error.exit())
}

/// Prints the core header schema, as a table or as JSON with `--json`.
///
/// The JSON form is what the future interface reads, so that no vocabulary is
/// ever redeclared outside [`schema`].
///
/// # Errors
///
/// Fails only if the schema cannot be serialized to JSON.
fn schema_command(args: &cli::SchemaArgs) -> Result<ExitCode> {
    if args.json {
        println!("{}", serde_json::to_string_pretty(&schema::export())?);
        return Ok(ExitCode::SUCCESS);
    }
    for key in schema::CORE_KEYS {
        let values = match key.value {
            schema::ValueType::Enum(values) => format!(" ({})", values.join(", ")),
            _ => String::new(),
        };
        println!("{:<11} {}{values}", key.name, key.value.id());
    }
    Ok(ExitCode::SUCCESS)
}

/// Fast-forwards the content repository at `root` to its upstream, and says
/// how many commits came in.
///
/// # Errors
///
/// Fails with git's message when there is no upstream, the remote cannot be
/// reached, or the branch diverged.
fn pull_command(root: &std::path::Path) -> Result<ExitCode> {
    match serve::git::pull(root)? {
        serve::git::Pulled::UpToDate => println!("already up to date"),
        serve::git::Pulled::Forward(count) => {
            println!(
                "pulled {count} {}",
                if count == 1 { "commit" } else { "commits" }
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Chooses the node a command covers (PLAN §10).
///
/// An explicit `--node` wins; `--all` covers the whole tree (`None`); otherwise
/// the command focuses on the node of the current directory, and covers the
/// whole tree when the current directory stands in no node. The home node also
/// means the whole tree, since every node descends from it.
fn scope(node: Option<&str>, all: bool, here: Option<&tree::NodePath>) -> Option<tree::NodePath> {
    match (node, all) {
        (Some(node), _) => Some(tree::NodePath::new(node)),
        (None, true) => None,
        (None, false) => here.filter(|node| !node.is_home()).cloned(),
    }
}

/// Unit tests of the scope chosen for a command.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::NodePath;
    use clap::Parser;

    #[test]
    fn the_command_line_definition_is_consistent() {
        cli::Cli::command().debug_assert();
    }

    #[test]
    fn config_init_takes_the_repository_from_the_global_option() {
        let cli =
            cli::Cli::try_parse_from(["ariane", "config", "init", "--repo", "/content"]).unwrap();
        assert_eq!(cli.repo.as_deref(), Some(std::path::Path::new("/content")));
        assert!(matches!(
            cli.command,
            cli::Command::Config(cli::ConfigArgs {
                command: cli::ConfigCommand::Init { force: false }
            })
        ));
    }

    #[test]
    fn an_explicit_node_wins_over_the_current_directory() {
        let here = NodePath::new("cmc");
        assert_eq!(
            scope(Some("slashome/me"), false, Some(&here)),
            Some(NodePath::new("slashome/me"))
        );
    }

    #[test]
    fn all_covers_the_whole_tree_even_inside_a_node() {
        assert_eq!(scope(None, true, Some(&NodePath::new("cmc"))), None);
    }

    #[test]
    fn without_flags_the_command_focuses_on_the_current_node() {
        let here = NodePath::new("cmc/cmc-lib");
        assert_eq!(scope(None, false, Some(&here)), Some(here.clone()));
        assert_eq!(scope(None, false, Some(&NodePath::new(""))), None);
        assert_eq!(scope(None, false, None), None);
    }
}

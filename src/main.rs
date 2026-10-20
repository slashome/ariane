//! Entry point of the `ariane` command-line tool.
//!
//! `ariane` manages an Ariane content repository: a git repository whose `tree/`
//! directory mirrors the user's projects as nodes, each holding items (tasks,
//! stories, backlog items, reflexions, ressources) as Markdown files with a YAML
//! front-matter (SPEC.md §1, ADR-0008).
//!
//! This file only wires things together: it parses the command line ([`cli`]),
//! finds the content repository ([`repo`]), loads it into memory ([`tree`]) and
//! hands it to the module that implements the requested command:
//!
//! - `ariane tasks` → [`tasks`], lists items;
//! - `ariane doctor` → [`doctor`], checks items without writing (ADR-0004);
//! - `ariane migrate` → [`migrate`], upgrades items to the current schema (ADR-0009);
//! - `ariane schema` → [`schema`], prints the header schema; it needs no repository.
//!
//! Exit code `3` means the command could not run at all (ADR-0004); the other
//! codes are chosen by each command.

/// Command-line arguments, declared with `clap`.
mod cli;
/// `ariane doctor`: validation of items and nodes.
mod doctor;
/// One item file: its front-matter and its body.
mod item;
/// `ariane migrate`: schema migrations of item files.
mod migrate;
/// `node.toml`: custom fields and status workflows of a node.
mod node;
/// Discovery of the content repository on disk.
mod repo;
/// The core header schema, declared once as data.
mod schema;
/// `ariane tasks`: the list of items.
mod tasks;
/// The whole content repository loaded in memory.
mod tree;

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

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
/// `ariane schema` is answered before looking for a repository, because the
/// schema is built into the binary. Every other command first discovers the
/// content repository and loads the whole tree once.
///
/// # Errors
///
/// Fails when no content repository can be found or read, or when the command
/// itself fails.
fn run() -> Result<ExitCode> {
    let cli = cli::Cli::parse();
    if let cli::Command::Schema(args) = &cli.command {
        return schema_command(args);
    }
    let root = repo::discover(cli.repo.as_deref())?;
    let tree = tree::Tree::load(&root)?;
    match cli.command {
        cli::Command::Tasks(args) => tasks::run(&tree, &args),
        cli::Command::Doctor(args) => doctor::run(&tree, &args),
        cli::Command::Migrate(args) => migrate::run(&tree, &args),
        cli::Command::Schema(args) => schema_command(&args),
    }
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

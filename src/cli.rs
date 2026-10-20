//! Command-line interface of `ariane`, declared with `clap`'s derive macros.
//!
//! Each struct below becomes a parser: `#[derive(Parser)]` for the top-level
//! command, `#[derive(Subcommand)]` for the list of commands and `#[derive(Args)]`
//! for the arguments of one command. The `about` and `help` attributes are the
//! texts shown by `ariane --help`; they are set explicitly so that the rustdoc on
//! these items never leaks into the help output.
//!
//! Struct fields carry no rustdoc of their own, and each struct keeps a
//! one-paragraph doc: on a `clap` item, a field doc or a multi-paragraph doc
//! becomes help text. `repo` is the content repository given by `--repo` or
//! `ARIANE_REPO`; `global = true` makes it accepted after any command, and
//! `None` means "discover it from the current directory".

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// The whole command line: the global `repo` option and the chosen command.
#[derive(Parser)]
#[command(
    name = "ariane",
    version,
    about = "Ariadne's thread through your projects"
)]
pub struct Cli {
    #[arg(long, global = true, env = "ARIANE_REPO")]
    pub repo: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

/// The commands `ariane` understands.
#[derive(Subcommand)]
pub enum Command {
    #[command(about = "List the items of the tree")]
    Tasks(TasksArgs),
    #[command(
        about = "Check every item and node against the specification, without writing anything"
    )]
    Doctor(DoctorArgs),
    #[command(about = "Bring items to the current schema version")]
    Migrate(MigrateArgs),
    #[command(about = "Print the item header schema")]
    Schema(SchemaArgs),
    #[command(about = "Serve the web interface on 127.0.0.1")]
    Serve(ServeArgs),
}

/// Arguments of `ariane tasks`: an optional node scope, whether to include
/// archived items, and `json` to print JSON instead of a table.
#[derive(Args)]
pub struct TasksArgs {
    #[arg(
        long,
        help = "Only the items of this node and its descendants (default: the node of the current directory)"
    )]
    pub node: Option<String>,

    #[arg(
        long,
        conflicts_with = "node",
        help = "Cover the whole tree, wherever the command runs"
    )]
    pub all: bool,

    #[arg(long, help = "Include archived items")]
    pub archived: bool,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane doctor`: an optional node scope, `strict` to fail on
/// warnings, and `json` to print the findings as JSON.
#[derive(Args)]
pub struct DoctorArgs {
    #[arg(
        long,
        help = "Only check this node and its descendants (default: the node of the current directory)"
    )]
    pub node: Option<String>,

    #[arg(
        long,
        conflicts_with = "node",
        help = "Cover the whole tree, wherever the command runs"
    )]
    pub all: bool,

    #[arg(long, help = "Treat warnings as errors")]
    pub strict: bool,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane migrate`: an optional node scope, `apply` to write the
/// files (a dry run otherwise), and `json` to print the plan as JSON.
#[derive(Args)]
pub struct MigrateArgs {
    #[arg(
        long,
        help = "Only migrate this node and its descendants (default: the node of the current directory)"
    )]
    pub node: Option<String>,

    #[arg(
        long,
        conflicts_with = "node",
        help = "Cover the whole tree, wherever the command runs"
    )]
    pub all: bool,

    #[arg(
        long,
        help = "Write the migrated files. Without it, print the changes and write nothing"
    )]
    pub apply: bool,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane schema`: `json` to print the schema as JSON.
#[derive(Args)]
pub struct SchemaArgs {
    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane serve`: the port to listen on, on the loopback interface
/// only (ADR-0012), and whether to open the browser once listening.
#[derive(Args)]
pub struct ServeArgs {
    #[arg(long, default_value_t = 4317, help = "Port to listen on, on 127.0.0.1")]
    pub port: u16,

    #[arg(long, help = "Open the interface in the browser once listening")]
    pub open: bool,
}

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

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
}

#[derive(Args)]
pub struct TasksArgs {
    #[arg(long, help = "Only the items of this node and its descendants")]
    pub node: Option<String>,

    #[arg(long, help = "Include archived items")]
    pub archived: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct DoctorArgs {
    #[arg(long, help = "Only check this node and its descendants")]
    pub node: Option<String>,

    #[arg(long, help = "Treat warnings as errors")]
    pub strict: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct MigrateArgs {
    #[arg(long, help = "Only migrate this node and its descendants")]
    pub node: Option<String>,

    #[arg(
        long,
        help = "Write the migrated files. Without it, print the changes and write nothing"
    )]
    pub apply: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct SchemaArgs {
    #[arg(long)]
    pub json: bool,
}

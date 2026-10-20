mod cli;
mod doctor;
mod item;
mod migrate;
mod node;
mod repo;
mod schema;
mod tasks;
mod tree;

use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ariane: {error:#}");
            ExitCode::from(3)
        }
    }
}

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

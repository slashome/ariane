mod cli;
mod doctor;
mod item;
mod migrate;
mod node;
mod repo;
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
    let root = repo::discover(cli.repo.as_deref())?;
    let tree = tree::Tree::load(&root)?;
    match cli.command {
        cli::Command::Tasks(args) => tasks::run(&tree, &args),
        cli::Command::Doctor(args) => doctor::run(&tree, &args),
        cli::Command::Migrate(args) => migrate::run(&tree, &args),
    }
}

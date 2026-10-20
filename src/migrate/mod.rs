mod v1;

use std::process::ExitCode;

use anyhow::{Result, bail};

use crate::cli::MigrateArgs;
use crate::tree::Tree;

pub fn run(_tree: &Tree, _args: &MigrateArgs) -> Result<ExitCode> {
    bail!("migrate is not implemented yet")
}

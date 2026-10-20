use std::process::ExitCode;

use anyhow::{Result, bail};

use crate::cli::DoctorArgs;
use crate::tree::Tree;

pub fn run(_tree: &Tree, _args: &DoctorArgs) -> Result<ExitCode> {
    bail!("doctor is not implemented yet")
}

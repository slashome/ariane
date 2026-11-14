//! `ariane release`: declares, orders, renames, marks and removes the releases
//! of a node, and plans an item in one (ADR-0023). Every subcommand writes at once
//! and never commits, like `ariane people`; it goes through [`super::declare`],
//! the code `ariane serve` runs for the same gestures.

use std::process::ExitCode;

use anyhow::{Result, bail};

use super::declare::{self, Declaration, Marks, Place, Refusal};
use crate::cli::{ReleaseArgs, ReleaseCommand};
use crate::tree::{NodePath, Tree};

/// The place the flags name, `default` when none is given.
fn place<'a>(
    before: Option<&'a str>,
    after: Option<&'a str>,
    top: bool,
    default: Place<'a>,
) -> Place<'a> {
    match (before, after, top) {
        (Some(other), _, _) => Place::Before(other),
        (_, Some(other), _) => Place::After(other),
        (_, _, true) => Place::Top,
        _ => default,
    }
}

/// Prints the release list of `node` once written, in delivery order.
fn print_list(root: &std::path::Path, node: &NodePath) -> Result<()> {
    let tree = Tree::load(root)?;
    let (_, releases) = declare::effective_list(&tree, node);
    let names: Vec<String> = releases
        .iter()
        .map(|release| {
            let mut shown = release.name.clone();
            if release.parking {
                shown.push_str(" (parking)");
            }
            if let Some(target) = &release.target {
                shown.push_str(&format!(" (target {target})"));
            }
            shown
        })
        .collect();
    println!("{node}: {}", names.join(", "));
    Ok(())
}

/// Runs `ariane release`.
///
/// # Errors
///
/// Fails when a file cannot be read or written, an item cannot be found, or a
/// change is refused.
pub fn run(tree: &Tree, args: &ReleaseArgs, here: Option<&NodePath>) -> Result<ExitCode> {
    let listed = match &args.command {
        ReleaseCommand::Add(add) => {
            let node = NodePath::new(&add.node);
            let declaration = Declaration {
                name: add.name.clone(),
                parking: add.parking,
                target: add.target.clone(),
            };
            let place = place(
                add.before.as_deref(),
                add.after.as_deref(),
                add.top,
                Place::Bottom,
            );
            settle(declare::add(tree, &node, &declaration, place)?)?;
            Some(node)
        }
        ReleaseCommand::Move(moved) => {
            let node = NodePath::new(&moved.node);
            let place = place(
                moved.before.as_deref(),
                moved.after.as_deref(),
                moved.top,
                Place::Bottom,
            );
            settle(declare::move_to(tree, &node, &moved.name, place)?)?;
            Some(node)
        }
        ReleaseCommand::Rename { node, name, to } => {
            let node = NodePath::new(node);
            let rewritten = settle(declare::rename(tree, &node, name, to)?)?;
            if !rewritten.is_empty() {
                println!("rewrote {}", rewritten.join(", "));
            }
            Some(node)
        }
        ReleaseCommand::Remove { node, name } => {
            let node = NodePath::new(node);
            settle(declare::remove(tree, &node, name)?)?;
            Some(node)
        }
        ReleaseCommand::Set(marked) => {
            let node = NodePath::new(&marked.node);
            let marks = Marks {
                parking: marked.parking(),
                target: marked.target(),
            };
            settle(declare::set(tree, &node, &marked.name, marks)?)?;
            Some(node)
        }
        ReleaseCommand::Assign(assign) => {
            let item = crate::show::find(tree, &assign.reference, here)?;
            let release = assign.name.as_deref().filter(|_| !assign.none);
            let changed = settle(declare::assign(tree, item, release)?)?;
            let reference = item.reference().unwrap_or_default();
            let shown = release.unwrap_or("none");
            match (changed, release) {
                (true, _) => println!("{reference}  release: {shown}"),
                (false, Some(_)) => println!("{reference} is already in {shown}"),
                (false, None) => println!("{reference} has no release of its own already"),
            }
            None
        }
    };
    if let Some(node) = listed {
        print_list(&tree.root, &node)?;
    }
    Ok(ExitCode::SUCCESS)
}

/// The value of a change that went through, or its refusal as an error.
fn settle<T>(outcome: Result<T, Refusal>) -> Result<T> {
    match outcome {
        Ok(value) => Ok(value),
        Err(refusal) => bail!("{refusal}"),
    }
}

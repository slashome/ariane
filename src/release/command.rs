//! `ariane release`: declares, orders, renames, marks and removes the releases
//! of a node, plans an item in one (ADR-0023), closes one and prints its
//! changelog entry (ADR-0029). Every subcommand writes at once and never
//! commits, like `ariane people`; it goes through [`super::declare`] and
//! [`super::close`], the code `ariane serve` runs for the same gestures.

use std::process::ExitCode;

use anyhow::{Result, bail};

use super::close::{self, Closing, Request};
use super::declare::{self, Declaration, Marks, Place, Refusal};
use crate::cli::{ReleaseArgs, ReleaseCloseArgs, ReleaseCommand};
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
        ReleaseCommand::Close(args) => return run_close(tree, args),
        ReleaseCommand::Changelog(args) => {
            let node = NodePath::new(&args.node);
            let (_, releases) = declare::effective_list(tree, &node);
            if let Err(refusal) = declare::check_declarer(tree, &node) {
                bail!("{refusal}");
            }
            if !releases.iter().any(|release| release.name == args.name) {
                bail!("no release `{}` declared in `{node}`", args.name);
            }
            let entry = super::changelog::entry(tree, &node, &args.name);
            if args.json {
                println!("{}", serde_json::to_string_pretty(&entry)?);
            } else {
                print!("{}", entry.markdown);
            }
            None
        }
    };
    if let Some(node) = listed {
        print_list(&tree.root, &node)?;
    }
    Ok(ExitCode::SUCCESS)
}

/// Runs `ariane release close`: prints the plan, or JSON; exits `2` on a dry
/// run the close would refuse for its open work.
fn run_close(tree: &Tree, args: &ReleaseCloseArgs) -> Result<ExitCode> {
    let node = NodePath::new(&args.node);
    let today = crate::closed::today();
    let request = Request {
        node: &node,
        name: &args.name,
        carry_over: args.carry_over,
        dry_run: args.dry_run,
        today: &today,
    };
    let closing = settle(close::close(tree, &request)?)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&closing)?);
    } else {
        print_closing(&closing);
    }
    Ok(if closing.blocked {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

/// Prints the plan of a close: the open work, what is archived and what
/// stays, then the changelog entry.
fn print_closing(closing: &Closing) {
    let would = |done: &str, planned: &str| {
        if closing.dry_run {
            planned.to_owned()
        } else {
            done.to_owned()
        }
    };
    let head = match (closing.dry_run, closing.blocked) {
        (true, true) => format!(
            "{} {}: cannot close yet (dry run, nothing written)",
            closing.node, closing.release
        ),
        (true, false) => format!(
            "{} {}: would close on {} (dry run, nothing written)",
            closing.node, closing.release, closing.closed
        ),
        (false, _) => format!(
            "{} {}: closed on {}",
            closing.node, closing.release, closing.closed
        ),
    };
    println!("{head}");
    if !closing.open.is_empty() {
        let destination = match (&closing.carry_to, closing.carry_over) {
            (Some(next), true) => would(
                &format!("carried over to {next}"),
                &format!("would be carried over to {next}"),
            ),
            (Some(next), false) => format!("refused; --carry-over moves it to {next}"),
            (None, _) => "refused; no later release to carry it over to".to_owned(),
        };
        println!("\nopen work, {destination}:");
        for work in &closing.open {
            println!(
                "  {}  {}  ({})",
                work.id,
                work.title,
                work.status.as_deref().unwrap_or("no status")
            );
        }
    }
    println!(
        "\n{} {}:",
        would("archived", "would archive"),
        plural(closing.archived.len(), "item")
    );
    for archived in &closing.archived {
        let mut line = format!(
            "  {}  {} -> {}",
            archived.work.id, archived.from, archived.to
        );
        if archived.references > 0 {
            line.push_str(&format!(
                " ({} in {})",
                plural(archived.references, "reference"),
                plural(archived.files, "file")
            ));
        }
        println!("{line}");
        for file in &archived.mentioned {
            println!("    still mentioned in {file}, to read by hand");
        }
    }
    if !closing.kept.is_empty() {
        println!("\nkept outside archives:");
        for kept in &closing.kept {
            println!("  {}  {}: {}", kept.work.id, kept.work.title, kept.reason);
        }
    }
    println!("\nchangelog entry:\n");
    print!("{}", closing.changelog.markdown);
}

/// `count` followed by `word`, with an `s` when it is not one.
fn plural(count: usize, word: &str) -> String {
    if count == 1 {
        format!("1 {word}")
    } else {
        format!("{count} {word}s")
    }
}

/// The value of a change that went through, or its refusal as an error.
fn settle<T>(outcome: Result<T, Refusal>) -> Result<T> {
    match outcome {
        Ok(value) => Ok(value),
        Err(refusal) => bail!("{refusal}"),
    }
}

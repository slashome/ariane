//! `ariane fix`: applies the repairs `ariane doctor` names as a command
//! (ADR-0004: doctor judges, fix writes; every `doctor` finding whose remedy is
//! `ariane fix` is part of fix's plan). Two repairs, both moves of a file:
//! give every item the file name SPEC.md §1.4 requires, `<id>-<slug>.md`, and
//! move the items of the former `backlog/` directory into `tasks/` (SPEC.md
//! §1.1); then rewrite the references to the old path.
//!
//! An id lives in the file name, so a new id — a task promoted to a story —
//! means a new name, and every link to the old one breaks. This command is the
//! one place that renames: it keeps the current name as the slug (dropping an
//! older id at its head, so `T003-login.md` with `id: S001` becomes
//! `S001-login.md`), then rewrites, in every Markdown file of the repository,
//! the link texts, link targets and code spans that point to the old file.
//!
//! A reference is rewritten when it resolves to the old file from the file
//! that holds it — it then becomes the path from where the holder will sit to
//! the new file — or when it is a tail of the old path (`tasks/a.md`, `a.md`)
//! that resolves to no other existing file — its tail is then swapped for the
//! new one. A mention in plain prose is never rewritten: it is reported, for a
//! human to read.
//!
//! Without `--apply` it prints the plan and writes nothing. It never commits
//! (ADR-0012), and it judges nothing: a file named by its bare id, or whose new
//! name is taken, is reported as needing a human.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::cli::FixArgs;
use crate::item::LEGACY_BACKLOG;
use crate::schema::is_valid_id;
use crate::tree::{Item, NodePath, Tree};

/// The prefixes of the ids `ariane migrate` allocates (SPEC.md §1.4).
const ID_PREFIXES: [char; 5] = ['T', 'E', 'S', 'R', 'D'];

/// Whether `stem` is `<id>-` followed by a non-empty slug.
pub fn conforms(id: &str, stem: &str) -> bool {
    stem.strip_prefix(id)
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|slug| !slug.is_empty())
}

/// The stem the file must take: `<id>-<slug>`, the slug being the current
/// stem without an older id at its head. `None` when no slug is left, which
/// only a human can choose.
pub fn target_stem(id: &str, stem: &str) -> Option<String> {
    let slug = strip_old_id(stem);
    (!slug.is_empty() && slug != id).then(|| format!("{id}-{slug}"))
}

/// Drops a leading `T012-`, `S3-`… from `stem`.
fn strip_old_id(stem: &str) -> &str {
    let mut chars = stem.char_indices();
    let Some((_, first)) = chars.next() else {
        return stem;
    };
    if !ID_PREFIXES.contains(&first) {
        return stem;
    }
    let digits_end = stem[1..]
        .find(|c: char| !c.is_ascii_digit())
        .map_or(stem.len(), |index| index + 1);
    if digits_end == 1 {
        return stem;
    }
    match &stem[digits_end..] {
        "" => "",
        rest => rest.strip_prefix('-').unwrap_or(stem),
    }
}

/// Whether `text` names `name` as a whole file name, not inside a longer one
/// (`a.md` is not mentioned by `T1-a.md`).
fn mentions(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(index, _)| {
        let before = text[..index].chars().next_back();
        let after = text[index + name.len()..].chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
            && !after.is_some_and(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
    })
}

/// The directory that replaces the former `backlog/`.
const TASKS: &str = "tasks";

/// What happens to one item.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Renamed or moved, or to be by `--apply`.
    Renamed,
    /// A human must choose the name.
    NeedsHuman,
}

/// One file to rename or move, with the references rewritten for it.
#[derive(Debug, Serialize)]
pub struct Entry {
    /// Current path, relative to the repository root.
    pub from: String,
    /// New path, relative to the repository root; `None` when a human must choose.
    pub to: Option<String>,
    pub outcome: Outcome,
    /// Why a human is needed.
    pub reason: Option<String>,
    /// Files whose references to this one are rewritten, with how many.
    pub rewritten: BTreeMap<String, usize>,
    /// Files that still mention the old name in prose, to be read by a human.
    pub mentioned: Vec<String>,
    #[serde(skip)]
    file: PathBuf,
    #[serde(skip)]
    target: Option<PathBuf>,
}

/// Runs `ariane fix`: plans the renames of the items in `scope`, applies
/// them with `--apply`, then prints the plan. Exits `2` when a file needs a human.
///
/// # Errors
///
/// Fails when a file cannot be read, written or renamed.
pub fn run(tree: &Tree, args: &FixArgs, scope: Option<&NodePath>) -> Result<ExitCode> {
    let markdown = markdown_files(tree)?;
    let (entries, texts) = plan(tree, scope, &markdown)?;
    if args.apply {
        for (file, text) in &texts {
            std::fs::write(file, text).with_context(|| format!("writing {}", file.display()))?;
        }
        for entry in &entries {
            if let Some(target) = &entry.target {
                if let Some(directory) = target.parent() {
                    std::fs::create_dir_all(directory)
                        .with_context(|| format!("creating {}", directory.display()))?;
                }
                std::fs::rename(&entry.file, target)
                    .with_context(|| format!("renaming {}", entry.file.display()))?;
            }
        }
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        report(&entries, args.apply);
    }
    let blocked = entries
        .iter()
        .any(|entry| entry.outcome == Outcome::NeedsHuman);
    Ok(if blocked {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

/// Every Markdown file of the repository with its text — items, the
/// `README.md` and `HANDOFF.md` files among them, and the files outside
/// `tree/` such as the root `README.md` — hidden directories (`.git`) excepted.
fn markdown_files(tree: &Tree) -> Result<BTreeMap<PathBuf, String>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![tree.root.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .with_context(|| format!("reading {}", directory.display()))?
        {
            let path = entry?.path();
            let hidden = path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'));
            if hidden {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                let text = std::fs::read_to_string(&path)
                    .with_context(|| format!("reading {}", path.display()))?;
                files.insert(path, text);
            }
        }
    }
    Ok(files)
}

/// Plans the renames and moves and the rewritten texts, writing nothing.
///
/// Every target is chosen first, so that a reference is rewritten from where
/// its holder will sit once moved too. The texts are then rewritten in memory
/// one move after the other, so a file that links to several moved items gets
/// all its references rewritten. They are keyed by the path the file has
/// before any move.
fn plan(
    tree: &Tree,
    scope: Option<&NodePath>,
    markdown: &BTreeMap<PathBuf, String>,
) -> Result<(Vec<Entry>, BTreeMap<PathBuf, String>)> {
    let mut entries = Vec::new();
    let mut taken: Vec<PathBuf> = Vec::new();
    for item in tree.items_within(scope) {
        let Some(destination) = destination(tree, item) else {
            continue;
        };
        let mut entry = Entry {
            from: relative(tree, &item.file),
            to: None,
            outcome: Outcome::NeedsHuman,
            reason: None,
            rewritten: BTreeMap::new(),
            mentioned: Vec::new(),
            file: item.file.clone(),
            target: None,
        };
        match destination {
            Err(reason) => entry.reason = Some(reason),
            Ok(target) if target.exists() || taken.contains(&target) => {
                entry.reason = Some(format!("{} already exists", relative(tree, &target)));
            }
            Ok(target) => {
                entry.to = Some(relative(tree, &target));
                entry.outcome = Outcome::Renamed;
                entry.target = Some(target.clone());
                taken.push(target);
            }
        }
        entries.push(entry);
    }
    let moves: BTreeMap<PathBuf, PathBuf> = entries
        .iter()
        .filter_map(|entry| Some((entry.file.clone(), entry.target.clone()?)))
        .collect();
    let mut texts: BTreeMap<PathBuf, String> = BTreeMap::new();
    for entry in &mut entries {
        let Some(target) = entry.target.clone() else {
            continue;
        };
        let mark = mark(&entry.file, &target);
        for (file, original) in markdown {
            let text = texts.get(file).unwrap_or(original);
            let after = moves.get(file).unwrap_or(file);
            let holder = Holder {
                before: file,
                after: after.parent().unwrap_or(after),
            };
            let (rewritten, count) = rewrite(text, &holder, &entry.file, &target);
            if count > 0 {
                entry.rewritten.insert(relative(tree, file), count);
                texts.insert(file.clone(), rewritten);
            }
            let after = texts.get(file).unwrap_or(original);
            if mentions(after, &mark) {
                entry.mentioned.push(relative(tree, file));
            }
        }
    }
    Ok((entries, texts))
}

/// Where `item` must go: `None` when it is where it belongs, the reason a
/// human is needed when its new name cannot be chosen.
///
/// An item of the former `backlog/` moves to `tasks/` (or `archives/tasks/`);
/// an item with a valid `id` its file name does not begin with is renamed
/// after it.
fn destination(tree: &Tree, item: &Item) -> Option<Result<PathBuf, String>> {
    let stem = item.file.file_stem()?.to_string_lossy();
    let id = item.document.text("id").filter(|id| is_valid_id(id));
    let rename = id.filter(|id| !conforms(id, &stem));
    if rename.is_none() && !item.legacy {
        return None;
    }
    let name = match rename {
        Some(id) => match target_stem(id, &stem) {
            Some(new_stem) => format!("{new_stem}.md"),
            None => {
                return Some(Err(format!(
                    "the name holds no slug: choose `{id}-<slug>.md`"
                )));
            }
        },
        None => item.file.file_name()?.to_string_lossy().into_owned(),
    };
    let mut directory = item.file.parent()?.to_path_buf();
    if item.legacy {
        let node = tree.node_dir(&item.node);
        let within = directory.strip_prefix(&node).ok()?;
        let mut moved = node.clone();
        let mut replaced = false;
        for component in within.components() {
            if !replaced && component.as_os_str() == LEGACY_BACKLOG {
                moved.push(TASKS);
                replaced = true;
            } else {
                moved.push(component);
            }
        }
        directory = moved;
    }
    Some(Ok(directory.join(name)))
}

/// What a prose mention of `old` looks like: its file name, or, when the name
/// is kept and only the directory changes, the directory and the name
/// (`backlog/T1-a.md`).
fn mark(old: &Path, new: &Path) -> String {
    let name = old.file_name().unwrap_or_default().to_string_lossy();
    if old.file_name() != new.file_name() {
        return name.into_owned();
    }
    let directory = old
        .parent()
        .and_then(Path::file_name)
        .unwrap_or_default()
        .to_string_lossy();
    format!("{directory}/{name}")
}

/// A Markdown file whose references are rewritten: where it is read from, and
/// the directory it will sit in once every move is applied.
pub struct Holder<'a> {
    pub before: &'a Path,
    pub after: &'a Path,
}

/// Rewrites, in `text` held by `holder`, the link texts, link targets and code
/// spans that point to `old` so that they point to `new`, and counts them.
pub fn rewrite(text: &str, holder: &Holder, old: &Path, new: &Path) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    let mut rest = text;
    while let Some((start, end, next)) = next_reference(rest) {
        out.push_str(&rest[..start]);
        let reference = &rest[start..end];
        match retarget(reference, holder, old, new) {
            Some(rewritten) => {
                out.push_str(&rewritten);
                count += 1;
            }
            None => out.push_str(reference),
        }
        out.push_str(&rest[end..next]);
        rest = &rest[next..];
    }
    out.push_str(rest);
    (out, count)
}

/// The next bracketed span of `text` — between `[` and `]`, `(` and `)`, or
/// two backticks, on one line — as `(start, end, next)`: the content is
/// `start..end` and scanning resumes at `next`, past the closing character, so
/// a closing backtick is never read as an opening one. An opener without its
/// closer on the line, or a `[` or `(` whose content opens another span (the
/// outer parenthesis of `([a](b.md))`), gives an empty span right after it, so
/// scanning goes on inside.
fn next_reference(text: &str) -> Option<(usize, usize, usize)> {
    let open = text.find(['[', '(', '`'])?;
    let closing = match &text[open..open + 1] {
        "[" => ']',
        "(" => ')',
        _ => '`',
    };
    let start = open + 1;
    match text[start..].find([closing, '\n']) {
        Some(length)
            if text[start + length..].starts_with(closing)
                && (closing == '`' || !text[start..start + length].contains(['[', '(', '`'])) =>
        {
            Some((start, start + length, start + length + 1))
        }
        _ => Some((start, start, start)),
    }
}

/// The reference pointing to `new` when it points to `old`.
fn retarget(reference: &str, holder: &Holder, old: &Path, new: &Path) -> Option<String> {
    let old_name = old.file_name()?.to_str()?;
    let (path, anchor) = match reference.split_once('#') {
        Some((path, anchor)) => (path, Some(anchor)),
        None => (reference, None),
    };
    let (lead, candidate) = match path.rfind(char::is_whitespace) {
        Some(index) => path.split_at(index + 1),
        None => ("", path),
    };
    let trimmed = candidate.trim_matches(['<', '>']);
    if !trimmed.ends_with(old_name) || trimmed.contains("://") {
        return None;
    }
    let before = &trimmed[..trimmed.len() - old_name.len()];
    if !(before.is_empty() || before.ends_with('/')) {
        return None;
    }
    let directory = holder.before.parent()?;
    let resolved = normalize(&directory.join(trimmed));
    let replacement = if resolved == *old {
        let path = path_between(holder.after, new);
        if trimmed.starts_with("./") && !path.starts_with("../") {
            format!("./{path}")
        } else {
            path
        }
    } else if !resolved.exists() && is_tail(trimmed, old) {
        swap_tail(trimmed, new)
    } else {
        return None;
    };
    let mut rewritten = format!("{lead}{}", candidate.replacen(trimmed, &replacement, 1));
    if let Some(anchor) = anchor {
        rewritten.push('#');
        rewritten.push_str(anchor);
    }
    Some(rewritten)
}

/// Whether the components of `reference`, leading `.` and `..` dropped, end `old`.
fn is_tail(reference: &str, old: &Path) -> bool {
    let tail: Vec<_> = Path::new(reference)
        .components()
        .skip_while(|component| matches!(component, Component::CurDir | Component::ParentDir))
        .collect();
    let whole: Vec<_> = old.components().collect();
    !tail.is_empty() && whole.ends_with(&tail)
}

/// `reference`, a tail of an old path after its leading `.` and `..`, with
/// that tail replaced by as many trailing components of `new`.
fn swap_tail(reference: &str, new: &Path) -> String {
    let segments: Vec<&str> = reference.split('/').collect();
    let lead = segments
        .iter()
        .take_while(|segment| matches!(**segment, "." | ".."))
        .count();
    let tail = segments.len() - lead;
    let components: Vec<String> = new
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    let start = components.len().saturating_sub(tail);
    segments[..lead]
        .iter()
        .map(|segment| (*segment).to_owned())
        .chain(components[start..].iter().cloned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The relative path, with `/` separators, from the directory `from` to the
/// file `to`: `..` for each component of `from` past their common prefix.
fn path_between(from: &Path, to: &Path) -> String {
    let (from, to) = (normalize(from), normalize(to));
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    std::iter::repeat_n("..".to_owned(), from.len() - common)
        .chain(
            to[common..]
                .iter()
                .map(|component| component.as_os_str().to_string_lossy().into_owned()),
        )
        .collect::<Vec<_>>()
        .join("/")
}

/// Resolves `.` and `..` without touching the file system.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// `path` relative to the repository root, for messages.
fn relative(tree: &Tree, path: &Path) -> String {
    path.strip_prefix(&tree.root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// Prints one line per file, then the files to read by hand.
fn report(entries: &[Entry], applied: bool) {
    let verb = if applied { "moved" } else { "would move" };
    for entry in entries {
        match (&entry.outcome, &entry.to) {
            (Outcome::Renamed, Some(to)) => {
                let references: usize = entry.rewritten.values().sum();
                println!(
                    "{verb} {} -> {to} ({references} references in {} files)",
                    entry.from,
                    entry.rewritten.len()
                );
                for file in &entry.mentioned {
                    println!("  still mentioned in {file}, to read by hand");
                }
            }
            _ => println!(
                "needs a human: {} — {}",
                entry.from,
                entry.reason.as_deref().unwrap_or_default()
            ),
        }
    }
    if entries.is_empty() {
        println!("every file name begins with its id, and no item is left in backlog/");
    } else if !applied {
        println!("\nnothing written; run with --apply to move");
    }
}

/// Unit tests of the naming rule, of the reference rewriting and of a run.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    /// Rewrites `text`, held by `holder` which does not move, for `old`
    /// renamed `T1-a.md` in its directory.
    fn to_t1(text: &str, holder: &Path, old: &Path) -> (String, usize) {
        let holder = Holder {
            before: holder,
            after: holder.parent().unwrap(),
        };
        rewrite(text, &holder, old, &old.with_file_name("T1-a.md"))
    }

    #[test]
    fn a_reference_follows_a_move_from_where_its_holder_will_sit() {
        let old = Path::new("/r/tree/p/backlog/T1-a.md");
        let new = Path::new("/r/tree/p/tasks/T1-a.md");
        let index = Holder {
            before: Path::new("/r/tree/p/README.md"),
            after: Path::new("/r/tree/p"),
        };
        let text = "[A](./backlog/T1-a.md), `backlog/T1-a.md`, [B](backlog/T2-b.md)";
        assert_eq!(
            rewrite(text, &index, old, new),
            (
                "[A](./tasks/T1-a.md), `tasks/T1-a.md`, [B](backlog/T2-b.md)".to_owned(),
                2
            )
        );
        let sibling = Holder {
            before: Path::new("/r/tree/p/backlog/T2-b.md"),
            after: Path::new("/r/tree/p/tasks"),
        };
        assert_eq!(
            rewrite("[A](./T1-a.md)", &sibling, old, new),
            ("[A](./T1-a.md)".to_owned(), 1)
        );
        let staying = Holder {
            before: Path::new("/r/tree/p/backlog/README.md"),
            after: Path::new("/r/tree/p/backlog"),
        };
        assert_eq!(
            rewrite("[A](./T1-a.md)", &staying, old, new),
            ("[A](../tasks/T1-a.md)".to_owned(), 1)
        );
        let elsewhere = Holder {
            before: Path::new("/r/tree/q/tasks/T9-z.md"),
            after: Path::new("/r/tree/q/tasks"),
        };
        assert_eq!(
            rewrite("`p/backlog/T1-a.md`", &elsewhere, old, new),
            ("`p/tasks/T1-a.md`".to_owned(), 1)
        );
    }

    #[test]
    fn a_conforming_stem_is_the_id_a_dash_and_a_slug() {
        assert!(conforms("T1", "T1-a"));
        assert!(!conforms("T1", "T1"));
        assert!(!conforms("T1", "T1-"));
        assert!(!conforms("T1", "T10-a"));
        assert!(!conforms("T1", "a"));
    }

    #[test]
    fn the_target_keeps_the_name_and_drops_an_older_id() {
        assert_eq!(
            target_stem("T2", "alleger-la-doc").as_deref(),
            Some("T2-alleger-la-doc")
        );
        assert_eq!(target_stem("S1", "T003-login").as_deref(), Some("S1-login"));
        assert_eq!(target_stem("T1", "T10-a").as_deref(), Some("T1-a"));
        assert_eq!(target_stem("E2", "E002-epic").as_deref(), Some("E2-epic"));
        assert_eq!(
            target_stem("T5", "3035690590-migrer").as_deref(),
            Some("T5-3035690590-migrer")
        );
        assert_eq!(target_stem("T1", "Tables").as_deref(), Some("T1-Tables"));
        assert_eq!(target_stem("T1", "T1"), None);
        assert_eq!(target_stem("T1", "T7"), None);
    }

    #[test]
    fn a_name_is_mentioned_only_as_a_whole() {
        assert!(mentions("see a.md.", "a.md"));
        assert!(mentions("(a.md)", "a.md"));
        assert!(!mentions("see T1-a.md", "a.md"));
        assert!(!mentions("see a.mdx", "a.md"));
    }

    #[test]
    fn an_unclosed_link_does_not_stop_the_rewriting() {
        let old = Path::new("/r/tree/p/a.md");
        let holder = Path::new("/r/tree/p/README.md");
        let (rewritten, count) = to_t1("[x](broken\n[a](a.md)", holder, old);
        assert_eq!((rewritten.as_str(), count), ("[x](broken\n[a](T1-a.md)", 1));
    }

    #[test]
    fn only_references_to_the_renamed_file_are_rewritten() {
        let old = Path::new("/r/tree/p/tasks/a.md");
        let holder = Path::new("/r/tree/p/README.md");
        let text = "See [tasks/a.md](tasks/a.md#plan), `tasks/a.md`, [b](tasks/b.md), \
                    `08-other/a.md`, [x](https://x.io/a.md) and a.md in prose.";
        let (rewritten, count) = to_t1(text, holder, old);
        assert_eq!(count, 3);
        assert_eq!(
            rewritten,
            "See [tasks/T1-a.md](tasks/T1-a.md#plan), `tasks/T1-a.md`, [b](tasks/b.md), \
             `08-other/a.md`, [x](https://x.io/a.md) and a.md in prose."
        );
    }

    #[test]
    fn a_code_span_in_the_link_text_does_not_hide_the_target() {
        let old = Path::new("/r/tree/p/tasks/a.md");
        let holder = Path::new("/r/tree/p/README.md");
        let text = "[norme `constants.ts`/`types.ts` : A](./tasks/a.md) (cas `x.ts`)";
        let (rewritten, count) = to_t1(text, holder, old);
        assert_eq!(count, 1);
        assert_eq!(
            rewritten,
            "[norme `constants.ts`/`types.ts` : A](./tasks/T1-a.md) (cas `x.ts`)"
        );
    }

    #[test]
    fn nested_and_prefixed_references_are_found() {
        let old = Path::new("/r/tree/p/tasks/a.md");
        let holder = Path::new("/r/tree/p/README.md");
        let text = "([A](./tasks/a.md)) - [T1 — tasks/a.md](tasks/a.md) (cf. `a.md`)";
        let (rewritten, count) = to_t1(text, holder, old);
        assert_eq!(
            rewritten,
            "([A](./tasks/T1-a.md)) - [T1 — tasks/T1-a.md](tasks/T1-a.md) (cf. `T1-a.md`)"
        );
        assert_eq!(count, 4);
    }

    #[test]
    fn a_relative_link_from_another_node_is_rewritten() {
        let old = Path::new("/r/tree/p/tasks/a.md");
        let holder = Path::new("/r/tree/q/tasks/b.md");
        let (rewritten, count) = to_t1("[a](../../p/tasks/a.md)", holder, old);
        assert_eq!(
            (rewritten.as_str(), count),
            ("[a](../../p/tasks/T1-a.md)", 1)
        );
    }

    /// A one-node repository whose task is named without its id, linked from
    /// the node's README and from another task.
    fn sample() -> Repo {
        let repo = Repo::new();
        repo.write("p/README.md", "# p\n\n- [A](tasks/a.md)\n")
            .write(
                "p/tasks/a.md",
                "---\nariane: 1\nnode: p\nid: T1\ntitle: A\nstatus: todo\n---\n# A\n",
            )
            .write(
                "p/tasks/T2-b.md",
                "---\nariane: 1\nnode: p\nid: T2\ntitle: B\nstatus: todo\n---\nAfter [A](a.md); see a.md.\n",
            );
        repo
    }

    fn args(apply: bool) -> FixArgs {
        FixArgs {
            node: None,
            all: true,
            apply,
            json: false,
        }
    }

    #[test]
    fn a_dry_run_writes_nothing() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        run(&tree, &args(false), None).unwrap();
        assert!(repo.root().join("tree/p/tasks/a.md").exists());
        assert!(!repo.root().join("tree/p/tasks/T1-a.md").exists());
    }

    #[test]
    fn applying_renames_rewrites_the_links_and_reports_prose() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let markdown = markdown_files(&tree).unwrap();
        let (entries, _) = plan(&tree, None, &markdown).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].mentioned, ["tree/p/tasks/T2-b.md"]);
        assert_eq!(run(&tree, &args(true), None).unwrap(), ExitCode::SUCCESS);
        assert!(!repo.root().join("tree/p/tasks/a.md").exists());
        let read = |path: &str| std::fs::read_to_string(repo.root().join(path)).unwrap();
        assert!(read("tree/p/tasks/T1-a.md").contains("id: T1"));
        assert_eq!(read("tree/p/README.md"), "# p\n\n- [A](tasks/T1-a.md)\n");
        assert!(read("tree/p/tasks/T2-b.md").ends_with("After [A](T1-a.md); see a.md.\n"));
        let tree = Tree::load(repo.root()).unwrap();
        assert!(crate::doctor::diagnose(&tree, None, false).is_empty());
    }

    #[test]
    fn the_items_of_the_former_backlog_move_to_tasks_with_their_links() {
        let repo = Repo::new();
        repo.write(
            "p/README.md",
            "# p\n\n- [A](backlog/T1-a.md)\n- [B](./backlog/b.md)\n",
        )
        .write(
            "p/backlog/T1-a.md",
            "---\nariane: 1\nnode: p\nid: T1\ntitle: A\nstatus: backlog\n---\nSee [B](./b.md) and [C](../tasks/T3-c.md).\n",
        )
        .write(
            "p/backlog/b.md",
            "---\nariane: 1\nnode: p\nid: T2\ntitle: B\nstatus: backlog\n---\n",
        )
        .write(
            "p/archives/backlog/T4-d.md",
            "---\nariane: 1\nnode: p\nid: T4\ntitle: D\nstatus: done\n---\n",
        )
        .write(
            "p/tasks/T3-c.md",
            "---\nariane: 1\nnode: p\nid: T3\ntitle: C\nstatus: todo\ndepends_on: [T1]\n---\nAfter [A](../backlog/T1-a.md).\n",
        );
        let tree = Tree::load(repo.root()).unwrap();
        assert_eq!(run(&tree, &args(true), None).unwrap(), ExitCode::SUCCESS);
        let read = |path: &str| std::fs::read_to_string(repo.root().join(path)).unwrap();
        assert_eq!(
            read("tree/p/README.md"),
            "# p\n\n- [A](tasks/T1-a.md)\n- [B](./tasks/T2-b.md)\n"
        );
        assert!(
            read("tree/p/tasks/T1-a.md")
                .ends_with("See [B](./T2-b.md) and [C](../tasks/T3-c.md).\n")
        );
        assert!(read("tree/p/tasks/T3-c.md").ends_with("After [A](T1-a.md).\n"));
        assert!(repo.root().join("tree/p/archives/tasks/T4-d.md").exists());
        assert!(
            !repo
                .root()
                .join("tree/p/backlog")
                .read_dir()
                .unwrap()
                .any(|_| true)
        );
        let tree = Tree::load(repo.root()).unwrap();
        assert!(crate::doctor::diagnose(&tree, None, false).is_empty());
    }

    #[test]
    fn a_bare_id_or_a_taken_name_needs_a_human() {
        let repo = Repo::new();
        repo.write("p/README.md", "# p\n")
            .write(
                "p/tasks/T1.md",
                "---\nariane: 1\nnode: p\nid: T1\ntitle: A\nstatus: todo\n---\n",
            )
            .write(
                "p/tasks/b.md",
                "---\nariane: 1\nnode: p\nid: T2\ntitle: B\nstatus: todo\n---\n",
            )
            .write(
                "p/tasks/T2-b.md",
                "---\nariane: 1\nnode: p\nid: T3\ntitle: C\nstatus: todo\n---\n",
            );
        let tree = Tree::load(repo.root()).unwrap();
        let markdown = markdown_files(&tree).unwrap();
        let (entries, _) = plan(&tree, None, &markdown).unwrap();
        let blocked: Vec<_> = entries
            .iter()
            .filter(|entry| entry.outcome == Outcome::NeedsHuman)
            .map(|entry| entry.from.as_str())
            .collect();
        assert_eq!(
            blocked,
            ["tree/p/tasks/T1.md", "tree/p/tasks/b.md"],
            "{entries:?}"
        );
        assert_eq!(run(&tree, &args(true), None).unwrap(), ExitCode::from(2));
        assert!(repo.root().join("tree/p/tasks/b.md").exists());
    }
}

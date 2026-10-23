//! `ariane search`: finds items by their header and their body.
//!
//! The text is cut into words and an item matches when every word appears in
//! its id, its reference, its title or its body, ignoring case and accents
//! ([`fold`]). Results come ranked: the item whose id or reference is the text
//! itself first, then the items whose header holds every word, then those found
//! only in their body; within a rank, tree order. Each result is the item's
//! `ariane tasks` row with the field that matched, an excerpt around the match
//! and the ranges of the excerpt to highlight, so the command line, the
//! interface (`GET /api/search`) and an agent all read the same answer.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use serde::Serialize;

use crate::cli::SearchArgs;
use crate::item::Kind;
use crate::tasks::{self, Row};
use crate::tree::{Item, NodePath, Tree};

/// How many characters of the body an excerpt keeps before the match.
const BEFORE: usize = 40;
/// How many characters of the body an excerpt keeps from the start of the match.
const AFTER: usize = 100;

/// Where the words were found, which is also the rank of a result: an exact id
/// or reference sorts before a header match, which sorts before a body match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    /// The text is the item's id or its reference.
    Reference,
    /// Every word is in the title, the id or the reference.
    Title,
    /// At least one word is only in the body.
    Body,
}

/// One item found, as printed by `ariane search --json`.
#[derive(Debug, Serialize)]
pub struct Hit<'a> {
    /// The item's `ariane tasks` row, its fields at the top level.
    #[serde(flatten)]
    pub row: Row<'a>,
    /// Where the words were found.
    pub field: Field,
    /// The title for a header match; for a body match, the body around the
    /// first word missing from the header, its whitespace collapsed.
    pub excerpt: String,
    /// The ranges of `excerpt` that match a word, as `[start, end)` counted in
    /// Unicode characters (code points), sorted and never overlapping.
    pub highlights: Vec<[usize; 2]>,
}

/// Which items a search covers.
#[derive(Debug, Default)]
pub struct Options<'a> {
    /// Include the items under `archives/`.
    pub archived: bool,
    /// Keep only these kinds; every kind when empty.
    pub kinds: &'a [Kind],
    /// The date of the latest commit of each file, as for `ariane tasks --dates`.
    pub dates: Option<&'a HashMap<PathBuf, String>>,
}

/// The items of `scope` that `text` finds, best first. The command and
/// `GET /api/search` both search through it; a text without words finds nothing.
pub fn search<'a>(
    tree: &'a Tree,
    scope: Option<&'a NodePath>,
    text: &str,
    options: &Options,
) -> Vec<Hit<'a>> {
    let words: Vec<Vec<char>> = text.split_whitespace().map(|word| fold(word).0).collect();
    if words.is_empty() {
        return Vec::new();
    }
    let whole = fold(text.trim()).0;
    let mut hits: Vec<Hit> = tree
        .items_within(scope)
        .filter(|item| options.archived || !item.archived)
        .filter(|item| options.kinds.is_empty() || options.kinds.contains(&item.kind))
        .filter_map(|item| {
            let mut hit = hit(tree, item, &words, &whole)?;
            if let Some(dates) = options.dates {
                hit.row.updated = dates.get(&item.relative).cloned();
            }
            Some(hit)
        })
        .collect();
    // A stable sort keeps tree order within a rank.
    hits.sort_by_key(|hit| hit.field);
    hits
}

/// Runs `ariane search`: prints the results as a table, each with its excerpt
/// when it comes from the body, or as JSON.
///
/// # Errors
///
/// Fails only if the results cannot be serialized to JSON.
pub fn run(tree: &Tree, args: &SearchArgs, scope: Option<&NodePath>) -> Result<ExitCode> {
    let options = Options {
        archived: args.archived,
        kinds: &args.kind,
        dates: None,
    };
    let hits = search(tree, scope, &args.text.join(" "), &options);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&hits)?);
        return Ok(ExitCode::SUCCESS);
    }
    let width = hits.iter().map(|hit| label(hit).len()).max().unwrap_or(0);
    for hit in &hits {
        println!(
            "{:width$}  {}",
            label(hit),
            hit.row.title.unwrap_or(&hit.row.path)
        );
        if hit.field == Field::Body {
            println!("{:width$}  … {} …", "", hit.excerpt);
        }
    }
    println!("\n{} items", hits.len());
    Ok(ExitCode::SUCCESS)
}

/// The reference of a hit, or its path for an item without an id.
fn label(hit: &Hit) -> String {
    hit.row
        .reference
        .clone()
        .unwrap_or_else(|| hit.row.path.clone())
}

/// Matches one item against the folded `words`, or `None` when a word is in
/// neither its header nor its body.
fn hit<'a>(tree: &Tree, item: &'a Item, words: &[Vec<char>], whole: &[char]) -> Option<Hit<'a>> {
    let row = tasks::row(tree, item);
    let title = row.title.unwrap_or_default();
    let title_folded = fold(title).0;
    let references: Vec<Vec<char>> = [row.id, row.reference.as_deref()]
        .into_iter()
        .flatten()
        .map(|text| fold(text).0)
        .collect();
    let in_header = |word: &Vec<char>| {
        contains(&title_folded, word) || references.iter().any(|field| contains(field, word))
    };
    let body = &item.document.body;
    let body_folded = fold(body).0;
    if !words
        .iter()
        .all(|word| in_header(word) || contains(&body_folded, word))
    {
        return None;
    }
    let field = if references.iter().any(|field| field.as_slice() == whole) {
        Field::Reference
    } else if words.iter().all(in_header) {
        Field::Title
    } else {
        Field::Body
    };
    let excerpt = match field {
        Field::Body => {
            let first = words
                .iter()
                .find(|word| !in_header(word))
                .unwrap_or(&words[0]);
            excerpt(body, first)
        }
        Field::Reference | Field::Title => title.to_owned(),
    };
    let highlights = highlights(&excerpt, words);
    Some(Hit {
        row,
        field,
        excerpt,
        highlights,
    })
}

/// Lowercases `text` and strips the accents of Latin letters, so that `Été`
/// and `ete` compare equal. Returns the folded characters with, for each, the
/// index of the original character it comes from: a ligature such as `œ`
/// folds to two characters that both point back to it.
pub fn fold(text: &str) -> (Vec<char>, Vec<usize>) {
    let mut folded = Vec::with_capacity(text.len());
    let mut origin = Vec::with_capacity(text.len());
    for (index, character) in text.chars().enumerate() {
        for lower in character.to_lowercase() {
            match unaccented(lower) {
                Some(plain) => {
                    for character in plain.chars() {
                        folded.push(character);
                        origin.push(index);
                    }
                }
                None => {
                    folded.push(lower);
                    origin.push(index);
                }
            }
        }
    }
    (folded, origin)
}

/// The unaccented form of a lowercase Latin letter, `None` for a character
/// that folds to itself. Combining marks (U+0300–U+036F) fold to nothing, so
/// a decomposed `é` matches like a precomposed one.
fn unaccented(character: char) -> Option<&'static str> {
    Some(match character {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ď' | 'đ' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'ĵ' => "j",
        'ķ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'œ' => "oe",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'ß' => "ss",
        'ţ' | 'ť' | 'ŧ' => "t",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        '\u{300}'..='\u{36f}' => "",
        _ => return None,
    })
}

/// Whether `needle` occurs in `haystack`.
fn contains(haystack: &[char], needle: &[char]) -> bool {
    find(haystack, needle, 0).is_some()
}

/// The index of the first occurrence of `needle` in `haystack` at or after
/// `from`; `None` for an empty needle.
fn find(haystack: &[char], needle: &[char], from: usize) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|&start| haystack[start..].starts_with(needle))
}

/// The part of `body` around the first occurrence of the folded `word`: up to
/// [`BEFORE`] characters before it and [`AFTER`] from it, cut at word
/// boundaries, with every run of whitespace collapsed into one space.
fn excerpt(body: &str, word: &[char]) -> String {
    let (folded, origin) = fold(body);
    let characters: Vec<char> = body.chars().collect();
    let start = find(&folded, word, 0).map_or(0, |index| origin[index]);
    let mut from = start.saturating_sub(BEFORE);
    let mut to = (start + AFTER).min(characters.len());
    if from > 0 {
        // Start after the first whitespace, so the excerpt opens on a whole word.
        from = (from..start)
            .find(|&index| characters[index].is_whitespace())
            .map_or(from, |index| index + 1);
    }
    if to < characters.len() {
        to = (start..to)
            .rev()
            .find(|&index| characters[index].is_whitespace())
            .unwrap_or(to);
    }
    characters[from..to]
        .iter()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The ranges of `text` where one of the folded `words` occurs, in characters
/// of `text`, sorted, with overlapping or touching ranges merged.
pub fn highlights(text: &str, words: &[Vec<char>]) -> Vec<[usize; 2]> {
    let (folded, origin) = fold(text);
    let mut ranges: Vec<[usize; 2]> = Vec::new();
    for word in words {
        let mut from = 0;
        while let Some(index) = find(&folded, word, from) {
            let last = index + word.len() - 1;
            ranges.push([origin[index], origin[last] + 1]);
            from = index + 1;
        }
    }
    ranges.sort_unstable();
    let mut merged: Vec<[usize; 2]> = Vec::with_capacity(ranges.len());
    for range in ranges {
        match merged.last_mut() {
            Some(previous) if range[0] <= previous[1] => previous[1] = previous[1].max(range[1]),
            _ => merged.push(range),
        }
    }
    merged
}

/// Unit tests of folding, matching, ranking and excerpts.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::fixture::Repo;

    fn folded(text: &str) -> String {
        fold(text).0.into_iter().collect()
    }

    fn words(text: &str) -> Vec<Vec<char>> {
        text.split_whitespace().map(|word| fold(word).0).collect()
    }

    #[test]
    fn folding_ignores_case_and_accents_and_points_back_to_the_original() {
        assert_eq!(folded("Été à Noël"), "ete a noel");
        assert_eq!(folded("Œuvre STRAßE"), "oeuvre strasse");
        assert_eq!(folded("e\u{301}te\u{301}"), "ete");
        assert_eq!(folded("日本 Ελλάδα"), "日本 ελλάδα");
        let (characters, origin) = fold("cœur");
        assert_eq!(characters.len(), 5);
        assert_eq!(origin, [0, 1, 1, 2, 3]);
    }

    #[test]
    fn highlights_count_characters_of_the_original_and_merge_overlaps() {
        assert_eq!(
            highlights("Été chaud, été", &words("ete")),
            [[0, 3], [11, 14]]
        );
        assert_eq!(highlights("cœur", &words("oe")), [[1, 2]]);
        assert_eq!(highlights("abcabc", &words("abc bca")), [[0, 6]]);
        assert!(highlights("rien", &words("x")).is_empty());
    }

    #[test]
    fn an_excerpt_opens_and_closes_on_whole_words_with_whitespace_collapsed() {
        let body = format!(
            "{}needle in\n\nthe   haystack {}",
            "word ".repeat(20),
            "tail ".repeat(40)
        );
        let excerpt = excerpt(&body, &fold("needle").0);
        assert!(excerpt.starts_with("word "), "{excerpt}");
        assert!(excerpt.contains("needle in the haystack"), "{excerpt}");
        assert!(excerpt.ends_with("tail"), "{excerpt}");
        assert!(excerpt.chars().count() <= BEFORE + AFTER);
    }

    /// A repository with a task found by reference, by title and by body.
    fn sample() -> Repo {
        let repo = Repo::new();
        repo.write(
            "p/tasks/T001-a.md",
            "---\nariane: 1\nnode: p\nid: T001\ntitle: Préparer l'été\nstatus: todo\n---\nRien.\n",
        )
        .write(
            "p/tasks/T002-b.md",
            "---\nariane: 1\nnode: p\nid: T002\ntitle: Autre chose\nstatus: todo\n---\nIl faut préparer la rentrée et l'ÉTÉ.\n",
        )
        .write(
            "p/stories/S001-s.md",
            "---\nariane: 1\nnode: p\nid: S001\ntitle: T002 en avance\nstatus: todo\n---\n",
        )
        .write(
            "p/archives/tasks/T003-c.md",
            "---\nariane: 1\nnode: p\nid: T003\ntitle: Été passé\nstatus: done\n---\n",
        )
        .write(
            "q/tasks/T001-x.md",
            "---\nariane: 1\nnode: q\nid: T001\ntitle: Ailleurs\nstatus: todo\n---\n",
        );
        repo
    }

    fn references(hits: &[Hit]) -> Vec<String> {
        hits.iter()
            .map(|hit| hit.row.reference.clone().unwrap())
            .collect()
    }

    #[test]
    fn results_rank_the_exact_reference_then_the_header_then_the_body() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let hits = search(&tree, None, "t002", &Options::default());
        assert_eq!(references(&hits), ["p#T002", "p#S001"]);
        assert_eq!(hits[0].field, Field::Reference);
        assert_eq!(hits[1].field, Field::Title);
        assert_eq!(hits[1].highlights, [[0, 4]]);

        let hits = search(&tree, None, "preparer ETE", &Options::default());
        assert_eq!(references(&hits), ["p#T001", "p#T002"]);
        assert_eq!(hits[0].field, Field::Title);
        assert_eq!(hits[0].excerpt, "Préparer l'été");
        assert_eq!(hits[0].highlights, [[0, 8], [11, 14]]);
        assert_eq!(hits[1].field, Field::Body);
        assert_eq!(hits[1].excerpt, "Il faut préparer la rentrée et l'ÉTÉ.");
    }

    #[test]
    fn a_search_keeps_its_scope_kinds_and_archives() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let hits = search(&tree, None, "p#T001", &Options::default());
        assert_eq!(references(&hits), ["p#T001"]);
        assert_eq!(hits[0].field, Field::Reference);
        let q = NodePath::new("q");
        assert_eq!(
            references(&search(&tree, Some(&q), "T001", &Options::default())),
            ["q#T001"]
        );
        assert_eq!(
            references(&search(&tree, None, "été", &Options::default())),
            ["p#T001", "p#T002"]
        );
        let archived = Options {
            archived: true,
            kinds: &[Kind::Task],
            ..Options::default()
        };
        assert_eq!(
            references(&search(&tree, None, "été", &archived)),
            ["p#T003", "p#T001", "p#T002"]
        );
        assert!(search(&tree, None, "  ", &Options::default()).is_empty());
        assert!(search(&tree, None, "été absent", &Options::default()).is_empty());
    }

    #[test]
    fn a_hit_is_a_tasks_row_with_its_field_excerpt_and_highlights() {
        let repo = sample();
        let tree = Tree::load(repo.root()).unwrap();
        let hits = search(&tree, None, "rentree", &Options::default());
        let json = serde_json::to_value(&hits[0]).unwrap();
        assert_eq!(json["reference"], "p#T002");
        assert_eq!(json["field"], "body");
        assert_eq!(json["highlights"], serde_json::json!([[20, 27]]));
    }
}

//! The pinned nodes of the dashboard, stored in `config.toml` (ADR-0013).
//!
//! Pins live under `[dashboard] pinned`, a list of node paths whose order is the
//! order of the cards. Reading tolerates a missing file. Writing goes through
//! `toml_edit`, which edits the parsed document in place: every other key,
//! comment and blank line of the owner's file is kept as it was.

use std::path::Path;

use anyhow::{Context, Result};
use toml_edit::{Array, DocumentMut, Item, Table, value};

/// The pinned node paths, in card order; empty when the file or the key is missing.
///
/// # Errors
///
/// Fails when the file exists but cannot be read or parsed.
pub fn read(file: &Path) -> Result<Vec<String>> {
    let Some(document) = load(file)? else {
        return Ok(Vec::new());
    };
    Ok(document
        .get("dashboard")
        .and_then(|dashboard| dashboard.get("pinned"))
        .and_then(Item::as_array)
        .map(|pinned| {
            pinned
                .iter()
                .filter_map(|entry| entry.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default())
}

/// Replaces the pinned node paths, creating the file and its directory if needed.
///
/// # Errors
///
/// Fails when the existing file cannot be parsed, or the file cannot be written.
pub fn write(file: &Path, pinned: &[String]) -> Result<()> {
    let mut document = load(file)?.unwrap_or_default();
    if !document.contains_table("dashboard") {
        document.insert("dashboard", Item::Table(Table::new()));
    }
    let list: Array = pinned.iter().map(String::as_str).collect();
    document["dashboard"]["pinned"] = value(list);
    if let Some(directory) = file.parent() {
        std::fs::create_dir_all(directory)
            .with_context(|| format!("creating {}", directory.display()))?;
    }
    std::fs::write(file, document.to_string())
        .with_context(|| format!("writing {}", file.display()))
}

/// Parses the configuration file, or `None` when it does not exist.
///
/// # Errors
///
/// Fails when the file exists but cannot be read or is not valid TOML.
fn load(file: &Path) -> Result<Option<DocumentMut>> {
    match std::fs::read_to_string(file) {
        Ok(text) => text
            .parse::<DocumentMut>()
            .map(Some)
            .with_context(|| format!("parsing {}", file.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", file.display())),
    }
}

/// Unit tests of pin storage, on temporary files.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_pins_nothing_and_writing_creates_it() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("ariane/config.toml");
        assert!(read(&file).unwrap().is_empty());
        write(&file, &["cmc".to_owned(), "slashome/me".to_owned()]).unwrap();
        assert_eq!(read(&file).unwrap(), ["cmc", "slashome/me"]);
    }

    #[test]
    fn writing_keeps_every_other_key_and_comment() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        let original = "# my config\nrepo = \"~/content\" # where it lives\n\n[dashboard]\n# card order\npinned = [\"a\"]\n";
        std::fs::write(&file, original).unwrap();
        write(&file, &["b".to_owned(), "a".to_owned()]).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("# my config\nrepo = \"~/content\" # where it lives\n"));
        assert!(text.contains("# card order\n"));
        assert_eq!(read(&file).unwrap(), ["b", "a"]);
    }
}

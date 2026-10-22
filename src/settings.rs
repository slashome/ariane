//! `settings.toml`: the preferences versioned with the content (ADR-0013,
//! amended 2026-10-23).
//!
//! `config.toml` only says how a machine finds the content repository and
//! serves it; it cannot hold anything that should follow the owner, since it
//! lives outside every repository. What follows the owner from machine to
//! machine — the theme of the interface, the dashboard pins and their order —
//! lives in `settings.toml` at the root of the content repository, versioned
//! and committed with the content.
//!
//! A missing file, or a missing key, takes its default. Writing goes through
//! `toml_edit`, so the owner's comments, order and blank lines survive.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use toml_edit::{Array, DocumentMut, Item, Table, value};

use crate::cli::{SettingsArgs, SettingsCommand};

/// Name of the file, at the root of the content repository.
pub const FILE: &str = "settings.toml";

/// The schema version this CLI writes and reads.
pub const VERSION: i64 = 1;

/// The primary color of the interface when none is chosen: its historical blue.
pub const DEFAULT_COLOR: &str = "#3760c9";

/// What a new file starts with, before its first key is written.
const HEADER: &str = "# Ariane settings, versioned with the content (ADR-0013).\n# Edit them in the interface, or with `ariane settings set`.\n\nversion = 1\n";

/// The keys `ariane settings set` changes.
pub const KEYS: [&str; 3] = ["theme.primary", "theme.secondary", "dashboard.pinned"];

/// The whole settings file, defaults filled in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: i64,
    pub theme: Theme,
    pub dashboard: Dashboard,
}

/// The `[theme]` table: the two colors the interface derives its palette from,
/// light and dark, as `#rrggbb`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    /// Buttons, links, focus rings, the logo and the favicon.
    pub primary: String,
    /// The accent: the active menu entry and the halo behind icon buttons.
    pub secondary: String,
}

/// The `[dashboard]` table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dashboard {
    /// Nodes pinned on the dashboard, in the order of their cards.
    pub pinned: Vec<String>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            version: VERSION,
            theme: Theme::default(),
            dashboard: Dashboard::default(),
        }
    }
}

impl Default for Theme {
    fn default() -> Theme {
        Theme {
            primary: DEFAULT_COLOR.to_owned(),
            secondary: DEFAULT_COLOR.to_owned(),
        }
    }
}

/// The settings file of the content repository at `root`.
pub fn path(root: &Path) -> PathBuf {
    root.join(FILE)
}

/// Whether `text` is a color as the theme writes it: `#` and six hex digits.
pub fn is_color(text: &str) -> bool {
    text.strip_prefix('#')
        .is_some_and(|hex| hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Reads the settings of the content repository at `root`; defaults when the
/// file is missing.
///
/// # Errors
///
/// Fails when the file cannot be read, is not valid TOML, or holds a theme
/// color that is not `#rrggbb`.
pub fn load(root: &Path) -> Result<Settings> {
    let file = path(root);
    let settings: Settings = match std::fs::read_to_string(&file) {
        Ok(text) => toml::from_str(&text).with_context(|| format!("parsing {FILE}"))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Settings::default(),
        Err(error) => return Err(error).with_context(|| format!("reading {FILE}")),
    };
    for (key, color) in [
        ("theme.primary", &settings.theme.primary),
        ("theme.secondary", &settings.theme.secondary),
    ] {
        if !is_color(color) {
            bail!("{FILE}: `{key} = \"{color}\"` is not a color written `#rrggbb`");
        }
    }
    Ok(settings)
}

/// Sets both theme colors.
///
/// # Errors
///
/// Fails when a color is not `#rrggbb`, or the file cannot be read or written.
pub fn set_theme(root: &Path, primary: &str, secondary: &str) -> Result<()> {
    for color in [primary, secondary] {
        if !is_color(color) {
            bail!("`{color}` is not a color written `#rrggbb`");
        }
    }
    edit(root, |document| {
        table(document, "theme");
        document["theme"]["primary"] = value(primary.to_ascii_lowercase());
        document["theme"]["secondary"] = value(secondary.to_ascii_lowercase());
    })
}

/// Replaces the dashboard pins, in card order.
///
/// # Errors
///
/// Fails when the file cannot be read or written.
pub fn set_pinned(root: &Path, pinned: &[String]) -> Result<()> {
    edit(root, |document| {
        table(document, "dashboard");
        let list: Array = pinned.iter().map(String::as_str).collect();
        document["dashboard"]["pinned"] = value(list);
    })
}

/// Makes sure `name` is a table of `document`.
fn table(document: &mut DocumentMut, name: &str) {
    if !document.contains_table(name) {
        document.insert(name, Item::Table(Table::new()));
    }
}

/// Applies `change` to the parsed file, or to a new one, and writes it back.
///
/// # Errors
///
/// Fails when the existing file cannot be parsed, or the file cannot be written.
fn edit(root: &Path, change: impl FnOnce(&mut DocumentMut)) -> Result<()> {
    let file = path(root);
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => HEADER.to_owned(),
        Err(error) => return Err(error).with_context(|| format!("reading {FILE}")),
    };
    let mut document = text
        .parse::<DocumentMut>()
        .with_context(|| format!("parsing {FILE}"))?;
    change(&mut document);
    std::fs::write(&file, document.to_string()).with_context(|| format!("writing {FILE}"))
}

/// Runs `ariane settings`: prints the settings, or sets one key.
///
/// # Errors
///
/// Fails when the file is invalid, the key unknown or the value refused.
pub fn run(root: &Path, args: &SettingsArgs) -> Result<ExitCode> {
    if let Some(SettingsCommand::Set { key, value }) = &args.command {
        set(root, key, value)?;
    }
    let settings = load(root)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&settings)?);
    } else {
        println!("theme.primary     {}", settings.theme.primary);
        println!("theme.secondary   {}", settings.theme.secondary);
        println!("dashboard.pinned  {}", settings.dashboard.pinned.join(", "));
        println!("\n{}", path(root).display());
    }
    Ok(ExitCode::SUCCESS)
}

/// Sets one key of [`KEYS`]; a list is written `a, b`.
///
/// # Errors
///
/// Fails when the key is unknown or the value refused.
fn set(root: &Path, key: &str, raw: &str) -> Result<()> {
    let current = load(root)?;
    match key {
        "theme.primary" => set_theme(root, raw, &current.theme.secondary),
        "theme.secondary" => set_theme(root, &current.theme.primary, raw),
        "dashboard.pinned" => {
            let pinned: Vec<String> = raw
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_owned)
                .collect();
            set_pinned(root, &pinned)
        }
        _ => bail!("unknown key `{key}`; known keys: {}", KEYS.join(", ")),
    }
}

/// Unit tests of reading, validating and writing the settings.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_gives_the_defaults_and_a_write_creates_it() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), Settings::default());
        set_pinned(dir.path(), &["cmc".to_owned(), "slashome/me".to_owned()]).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.dashboard.pinned, ["cmc", "slashome/me"]);
        assert_eq!(loaded.theme.primary, DEFAULT_COLOR);
        let text = std::fs::read_to_string(path(dir.path())).unwrap();
        assert!(text.starts_with("# Ariane settings"), "{text}");
    }

    #[test]
    fn writing_the_theme_keeps_comments_and_other_keys() {
        let dir = tempfile::tempdir().unwrap();
        let original = "# mine\nversion = 1\n\n[dashboard]\n# card order\npinned = [\"a\"]\n";
        std::fs::write(path(dir.path()), original).unwrap();
        set_theme(dir.path(), "#AA0000", "#00aa00").unwrap();
        let text = std::fs::read_to_string(path(dir.path())).unwrap();
        assert!(text.starts_with("# mine\nversion = 1\n"));
        assert!(text.contains("# card order\npinned = [\"a\"]"));
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.theme.primary, "#aa0000");
        assert_eq!(loaded.theme.secondary, "#00aa00");
    }

    #[test]
    fn a_color_must_be_six_hex_digits() {
        assert!(is_color("#3760c9"));
        assert!(!is_color("3760c9"));
        assert!(!is_color("#376"));
        assert!(!is_color("#3760cg"));
        let dir = tempfile::tempdir().unwrap();
        assert!(set_theme(dir.path(), "blue", "#000000").is_err());
        std::fs::write(path(dir.path()), "[theme]\nprimary = \"red\"\n").unwrap();
        assert!(
            load(dir.path())
                .unwrap_err()
                .to_string()
                .contains("theme.primary")
        );
    }

    #[test]
    fn set_changes_one_key_and_refuses_an_unknown_one() {
        let dir = tempfile::tempdir().unwrap();
        set(dir.path(), "theme.secondary", "#112233").unwrap();
        set(dir.path(), "dashboard.pinned", "a, b").unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.theme.primary, DEFAULT_COLOR);
        assert_eq!(loaded.theme.secondary, "#112233");
        assert_eq!(loaded.dashboard.pinned, ["a", "b"]);
        assert!(set(dir.path(), "theme.tertiary", "#000000").is_err());
    }
}

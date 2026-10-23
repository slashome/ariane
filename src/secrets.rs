//! `secrets.toml`: the tokens of the trackers, one per account (ADR-0011).
//!
//! Tokens never go in `config.toml` nor in any repository (ADR-0013). They
//! live in `secrets.toml`, next to `config.toml` but apart from it, as
//! `[<tracker>."<account>"] token = "…"`: a node's `[tracker]` names its
//! account, so two projects bound to two monday accounts use two tokens. The
//! file is created readable by its owner only, and refused when it is not.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use toml_edit::{DocumentMut, Item, Table, value};

/// The first line of the file, kept on top.
const HEADER: &str =
    "# Tokens of the trackers, one per account (ADR-0011). Never commit this file.\n";

/// `secrets.toml`, next to `config.toml`.
pub fn file() -> PathBuf {
    crate::config::file().with_file_name("secrets.toml")
}

/// The token of `account` on `tracker`, if one is stored.
///
/// # Errors
///
/// Fails when the file is readable by others than its owner, or cannot be
/// read or parsed.
pub fn token(file: &Path, tracker: &str, account: &str) -> Result<Option<String>> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("reading {}", file.display())),
    };
    check_private(file)?;
    let document: DocumentMut = text
        .parse()
        .with_context(|| format!("parsing {}", file.display()))?;
    Ok(document
        .get(tracker)
        .and_then(|table| table.get(account))
        .and_then(|entry| entry.get("token"))
        .and_then(Item::as_str)
        .map(str::to_owned))
}

/// Stores the token of `account` on `tracker`, keeping every other entry,
/// in a file readable by its owner only.
///
/// # Errors
///
/// Fails when the file cannot be read, parsed or written.
pub fn store(file: &Path, tracker: &str, account: &str, token: &str) -> Result<()> {
    let mut document: DocumentMut = match std::fs::read_to_string(file) {
        Ok(text) => text
            .parse()
            .with_context(|| format!("parsing {}", file.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DocumentMut::new(),
        Err(error) => return Err(error).with_context(|| format!("reading {}", file.display())),
    };
    if !document.contains_table(tracker) {
        let mut table = Table::new();
        table.set_implicit(true);
        document.insert(tracker, Item::Table(table));
    }
    document[tracker][account]["token"] = value(token);
    if let Some(directory) = file.parent() {
        std::fs::create_dir_all(directory)
            .with_context(|| format!("creating {}", directory.display()))?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut out = options
        .open(file)
        .with_context(|| format!("writing {}", file.display()))?;
    let mut text = document.to_string();
    if !text.starts_with(HEADER) {
        text.insert_str(0, HEADER);
    }
    out.write_all(text.as_bytes())
        .with_context(|| format!("writing {}", file.display()))?;
    restrict(file)
}

/// Refuses a secrets file that others than its owner may read.
fn check_private(file: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(file)?.permissions().mode();
        if mode & 0o077 != 0 {
            bail!(
                "{} is readable by others (mode {:o}): run `chmod 600 {}`",
                file.display(),
                mode & 0o777,
                file.display()
            );
        }
    }
    Ok(())
}

/// Makes an existing file readable by its owner only.
fn restrict(file: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("restricting {}", file.display()))?;
    }
    Ok(())
}

/// Unit tests of storing and reading tokens.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_per_account_in_a_private_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("secrets.toml");
        assert_eq!(token(&file, "monday", "acme").unwrap(), None);
        store(&file, "monday", "acme", "t1").unwrap();
        store(&file, "monday", "other", "t2").unwrap();
        assert_eq!(
            token(&file, "monday", "acme").unwrap().as_deref(),
            Some("t1")
        );
        assert_eq!(
            token(&file, "monday", "other").unwrap().as_deref(),
            Some("t2")
        );
        assert!(std::fs::read_to_string(&file).unwrap().starts_with(HEADER));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&file).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(
                token(&file, "monday", "acme")
                    .unwrap_err()
                    .to_string()
                    .contains("chmod 600")
            );
        }
    }
}

//! `ariane config`: create, read and change `config.toml` (ADR-0013).
//!
//! Every subcommand works on an explicit file path, which [`run`] takes from
//! [`super::file`] and tests take from a temporary directory: no test ever
//! touches the owner's real configuration. Changes go through `toml_edit`, so
//! comments, order and blank lines of the file survive; keys the schema does
//! not know are reported as warnings and left in place.

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use toml_edit::{Array, DocumentMut, Item, Table, Value, value};

use super::{KEYS, Kind, example, key, load, parse};
use crate::cli::{ConfigArgs, ConfigCommand};

/// Runs `ariane config <subcommand>` against the owner's configuration file.
///
/// # Errors
///
/// Fails when the file cannot be read, written or validated, or when the
/// subcommand's own checks refuse the change.
pub fn run(args: &ConfigArgs) -> Result<ExitCode> {
    let file = super::file();
    match &args.command {
        ConfigCommand::Path => println!("{}", file.display()),
        ConfigCommand::Example => print!("{}", example(None)),
        ConfigCommand::Init { force, repo } => {
            init(&file, *force, repo.as_deref())?;
            println!("created {}", file.display());
        }
        ConfigCommand::Get { key } => {
            warn_unknown(&file)?;
            println!("{}", get(&file, key)?);
        }
        ConfigCommand::Set { key, value } => {
            warn_unknown(&file)?;
            set(&file, key, value)?;
            println!("{key} = {}", get(&file, key)?);
        }
        ConfigCommand::Edit => return edit(&file),
    }
    Ok(ExitCode::SUCCESS)
}

/// Writes the default configuration to `file`, with `repo` set when given.
///
/// # Errors
///
/// Fails when the file already exists and `force` is not set, or when it
/// cannot be written.
pub fn init(file: &Path, force: bool, repo: Option<&str>) -> Result<()> {
    if file.exists() && !force {
        bail!(
            "{} already exists; use `ariane config set` to change a key, or --force to overwrite it",
            file.display()
        );
    }
    write(file, &example(repo))
}

/// The value of the dotted `name` in `file`, its default when absent, rendered
/// as TOML; an optional key without a value renders as an empty line.
///
/// # Errors
///
/// Fails when `name` is not a key of the schema or the file is invalid.
pub fn get(file: &Path, name: &str) -> Result<String> {
    let schema_key = known(name)?;
    let config = load(file)?.config;
    Ok(match schema_key.name {
        "version" => config.version.to_string(),
        "repo" => config.repo.unwrap_or_default(),
        "server.port" => config.server.port.to_string(),
        "dashboard.pinned" => {
            let list: Array = config.dashboard.pinned.iter().map(String::as_str).collect();
            list.to_string()
        }
        other => unreachable!("key {other} is in KEYS but not read here"),
    })
}

/// Sets the dotted `name` to `raw` in `file`, creating the file and its table
/// when needed.
///
/// `raw` is validated against the key's [`Kind`]: a port must be a number from
/// 1 to 65535, a list is either TOML (`["a", "b"]`) or comma-separated
/// (`a, b`), a path is taken as written. The whole file is parsed again before
/// it is written, so an invalid result never reaches the disk.
///
/// # Errors
///
/// Fails when `name` is unknown or not settable, `raw` is invalid for its kind,
/// or the file cannot be read or written.
pub fn set(file: &Path, name: &str, raw: &str) -> Result<()> {
    let schema_key = known(name)?;
    if !schema_key.settable {
        bail!("`{name}` is not set by hand; `ariane migrate` changes it");
    }
    let parsed = parse_value(schema_key.kind, raw).with_context(|| format!("invalid `{name}`"))?;
    let mut document = read_document(file)?;
    let (section, leaf) = name.rsplit_once('.').unwrap_or(("", name));
    let table = if section.is_empty() {
        document.as_table_mut()
    } else {
        if !document.contains_table(section) {
            document.insert(section, Item::Table(Table::new()));
        }
        document[section]
            .as_table_mut()
            .with_context(|| format!("`{section}` is not a table in {}", file.display()))?
    };
    match table.get_mut(leaf).and_then(Item::as_value_mut) {
        Some(existing) => {
            let decor = existing.decor().clone();
            *existing = parsed;
            *existing.decor_mut() = decor;
        }
        None => table[leaf] = value(parsed),
    }
    let text = document.to_string();
    parse(&text).context("the change would make the file invalid")?;
    write(file, &text)
}

/// Opens `file` in `$VISUAL` or `$EDITOR` (`vi` otherwise), creating it from
/// the defaults first, then validates it and reports what is wrong.
///
/// # Errors
///
/// Fails when the editor cannot be started or the file cannot be created.
fn edit(file: &Path) -> Result<ExitCode> {
    if !file.exists() {
        init(file, false, None)?;
    }
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_owned());
    let mut words = editor.split_whitespace();
    let program = words.next().unwrap_or("vi");
    let status = std::process::Command::new(program)
        .args(words)
        .arg(file)
        .status()
        .with_context(|| format!("starting the editor `{editor}`"))?;
    if !status.success() {
        bail!("the editor `{editor}` exited with {status}");
    }
    match load(file) {
        Ok(loaded) => {
            report_unknown(&loaded.unknown);
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            eprintln!("ariane config: {} is invalid: {error:#}", file.display());
            Ok(ExitCode::from(1))
        }
    }
}

/// The schema entry for `name`, or an error listing the known keys.
fn known(name: &str) -> Result<&'static super::Key> {
    key(name).with_context(|| {
        let names: Vec<_> = KEYS.iter().map(|key| key.name).collect();
        format!("unknown key `{name}`; known keys: {}", names.join(", "))
    })
}

/// Turns the text given on the command line into a TOML value of `kind`.
///
/// # Errors
///
/// Fails when `raw` does not fit `kind`.
fn parse_value(kind: Kind, raw: &str) -> Result<Value> {
    match kind {
        Kind::Version => bail!("the version is not set by hand"),
        Kind::Path => {
            if raw.trim().is_empty() {
                bail!("a path cannot be empty");
            }
            Ok(Value::from(raw))
        }
        Kind::Port => {
            let port: u16 = raw
                .trim()
                .parse()
                .with_context(|| format!("`{raw}` is not a port number"))?;
            if port == 0 {
                bail!("a port must be between 1 and 65535");
            }
            Ok(Value::from(i64::from(port)))
        }
        Kind::Strings => {
            let trimmed = raw.trim();
            if trimmed.starts_with('[') {
                let parsed: Value = trimmed
                    .parse()
                    .with_context(|| format!("`{raw}` is not a TOML list"))?;
                let all_strings = parsed
                    .as_array()
                    .is_some_and(|list| list.iter().all(|entry| entry.as_str().is_some()));
                if !all_strings {
                    bail!("`{raw}` must be a list of strings");
                }
                return Ok(parsed);
            }
            let list: Array = trimmed
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .collect();
            Ok(Value::Array(list))
        }
    }
}

/// Parses `file` as an editable document; a missing file is an empty one.
///
/// # Errors
///
/// Fails when the file exists but cannot be read or is not valid TOML.
fn read_document(file: &Path) -> Result<DocumentMut> {
    match std::fs::read_to_string(file) {
        Ok(text) => text
            .parse()
            .with_context(|| format!("parsing {}", file.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(DocumentMut::new()),
        Err(error) => Err(error).with_context(|| format!("reading {}", file.display())),
    }
}

/// Writes `text` to `file`, creating its directory.
///
/// # Errors
///
/// Fails when the directory or the file cannot be written.
fn write(file: &Path, text: &str) -> Result<()> {
    if let Some(directory) = file.parent() {
        std::fs::create_dir_all(directory)
            .with_context(|| format!("creating {}", directory.display()))?;
    }
    std::fs::write(file, text).with_context(|| format!("writing {}", file.display()))
}

/// Prints a warning for each key of `file` the schema does not know.
///
/// # Errors
///
/// Fails when the file is invalid.
fn warn_unknown(file: &Path) -> Result<()> {
    report_unknown(&load(file)?.unknown);
    Ok(())
}

/// Prints one warning per unknown key, on stderr.
fn report_unknown(unknown: &[String]) {
    for name in unknown {
        eprintln!("ariane config: warning: `{name}` is not a known key; it is kept as it is");
    }
}

/// Unit tests of `ariane config`, all against temporary files.
#[cfg(test)]
mod tests {
    use super::*;

    /// A temporary directory and the path of a config file inside it.
    fn temp() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("ariane/config.toml");
        (dir, file)
    }

    #[test]
    fn init_writes_the_defaults_and_refuses_to_overwrite() {
        let (_dir, file) = temp();
        init(&file, false, Some("~/content")).unwrap();
        assert_eq!(get(&file, "repo").unwrap(), "~/content");
        assert_eq!(get(&file, "server.port").unwrap(), "4317");
        assert!(init(&file, false, None).is_err());
        init(&file, true, None).unwrap();
        assert_eq!(get(&file, "repo").unwrap(), "");
    }

    #[test]
    fn get_falls_back_to_the_default_and_rejects_unknown_keys() {
        let (_dir, file) = temp();
        assert_eq!(get(&file, "server.port").unwrap(), "4317");
        assert_eq!(get(&file, "dashboard.pinned").unwrap(), "[]");
        assert!(get(&file, "server.host").is_err());
    }

    #[test]
    fn set_keeps_comments_order_and_unknown_keys() {
        let (_dir, file) = temp();
        write(
            &file,
            "# mine\nrepo = \"~/a\" # where\ncolour = \"blue\"\n\n[server]\n# the port\nport = 4317 # default\n",
        )
        .unwrap();
        set(&file, "server.port", "9000").unwrap();
        set(&file, "repo", "~/b").unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert_eq!(
            text,
            "# mine\nrepo = \"~/b\" # where\ncolour = \"blue\"\n\n[server]\n# the port\nport = 9000 # default\n"
        );
    }

    #[test]
    fn set_creates_the_file_and_the_table() {
        let (_dir, file) = temp();
        set(&file, "dashboard.pinned", "cmc, slashome/me").unwrap();
        assert_eq!(
            get(&file, "dashboard.pinned").unwrap(),
            r#"["cmc", "slashome/me"]"#
        );
        set(&file, "dashboard.pinned", r#"["a"]"#).unwrap();
        assert_eq!(load(&file).unwrap().config.dashboard.pinned, ["a"]);
    }

    #[test]
    fn set_validates_against_the_schema() {
        let (_dir, file) = temp();
        assert!(set(&file, "server.port", "0").is_err());
        assert!(set(&file, "server.port", "http").is_err());
        assert!(set(&file, "version", "2").is_err());
        assert!(set(&file, "repo", " ").is_err());
        assert!(set(&file, "dashboard.pinned", "[1, 2]").is_err());
        assert!(set(&file, "nope", "1").is_err());
        assert!(!file.exists());
    }
}

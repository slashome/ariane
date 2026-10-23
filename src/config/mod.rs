//! The configuration file, `config.toml`, and its schema (ADR-0013).
//!
//! The schema is declared once, in [`KEYS`](crate::config::KEYS): every key with its type, its
//! default and the sentence that documents it. Everything else is derived from
//! that table — the commented `config.example.toml` committed at the repository
//! root ([`example`](crate::config::example)), the defaults [`ariane config init`](crate::config::command) writes, the
//! validation of `ariane config set`, and the detection of keys the schema does
//! not know. [`Config`](crate::config::Config) is the typed view the rest of the CLI reads.
//!
//! Writing never goes through [`Config`](crate::config::Config): it edits the parsed document in place
//! with `toml_edit`, so the owner's comments, order and blank lines survive.
//! Secrets never live in this file (ADR-0011).

pub mod command;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use toml_edit::{DocumentMut, Item};

/// The schema version this CLI writes and reads.
pub const VERSION: i64 = 1;

/// The port `ariane serve` listens on when nothing else is configured.
pub const DEFAULT_PORT: u16 = 27317;

/// The name under which the front door serves the interface by default.
pub const DEFAULT_HOST: &str = "ariane.local";

/// The port of the front door, the local reverse proxy (ADR-0016), by default.
pub const DEFAULT_PROXY_PORT: u16 = 27380;

/// The shape a configuration value must have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A non-negative integer: the schema version.
    Version,
    /// A filesystem path; a leading `~/` stands for the home directory.
    Path,
    /// A TCP port, from 1 to 65535.
    Port,
    /// A host name, such as `ariane.local`: lowercase labels separated by dots.
    Host,
}

/// One key of the configuration file.
#[derive(Debug, Clone, Copy)]
pub struct Key {
    /// The dotted name: `server.port` is the key `port` of the `[server]` table.
    pub name: &'static str,
    /// The shape of its value.
    pub kind: Kind,
    /// Its default, written as a TOML value; `None` when the key is optional
    /// and has no default.
    pub default: Option<&'static str>,
    /// An example value shown commented out when the key has no default.
    pub example: Option<&'static str>,
    /// The sentence that documents it, copied into the example file.
    pub doc: &'static str,
    /// Whether `ariane config set` may change it; the version only moves through
    /// `ariane migrate`.
    pub settable: bool,
}

/// Every key the configuration knows, in the order of the example file.
pub const KEYS: [Key; 5] = [
    Key {
        name: "version",
        kind: Kind::Version,
        default: Some("1"),
        example: None,
        doc: "Schema version of this file. Changed only by `ariane migrate`.",
        settable: false,
    },
    Key {
        name: "repo",
        kind: Kind::Path,
        default: None,
        example: Some("\"~/path/to/_ariane\""),
        doc: "The content repository, used when the current directory leads to none.",
        settable: true,
    },
    Key {
        name: "server.port",
        kind: Kind::Port,
        default: Some("27317"),
        example: None,
        doc: "Port `ariane serve` listens on, on 127.0.0.1 only. `--port` overrides it.",
        settable: true,
    },
    Key {
        name: "server.host",
        kind: Kind::Host,
        default: Some("\"ariane.local\""),
        example: None,
        doc: "Name under which the front door serves the interface (`ariane proxy setup` declares it).",
        settable: true,
    },
    Key {
        name: "proxy.port",
        kind: Kind::Port,
        default: Some("27380"),
        example: None,
        doc: "Port of the front door, on 127.0.0.1, where port 80 of 127.0.0.2 is redirected.",
        settable: true,
    },
];

/// The typed configuration. Missing keys take their default, so an empty or
/// absent file is a valid configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Config {
    /// See [`KEYS`]: `version`.
    pub version: i64,
    /// See [`KEYS`]: `repo`, exactly as written in the file.
    pub repo: Option<String>,
    /// The `[server]` table.
    pub server: Server,
    /// The `[proxy]` table.
    pub proxy: Proxy,
}

/// The `[server]` table of the configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Server {
    /// See [`KEYS`]: `server.port`.
    pub port: u16,
    /// See [`KEYS`]: `server.host`.
    pub host: String,
}

/// The `[proxy]` table of the configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Proxy {
    /// See [`KEYS`]: `proxy.port`.
    pub port: u16,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            version: VERSION,
            repo: None,
            server: Server::default(),
            proxy: Proxy::default(),
        }
    }
}

impl Default for Server {
    fn default() -> Server {
        Server {
            port: DEFAULT_PORT,
            host: DEFAULT_HOST.to_owned(),
        }
    }
}

impl Default for Proxy {
    fn default() -> Proxy {
        Proxy {
            port: DEFAULT_PROXY_PORT,
        }
    }
}

impl Config {
    /// The configured content repository, with a leading `~/` expanded.
    pub fn repo_path(&self) -> Option<PathBuf> {
        self.repo.as_deref().map(expand_home)
    }
}

/// A configuration read from disk, with the keys the schema does not know.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Loaded {
    /// The typed configuration, defaults filled in.
    pub config: Config,
    /// Dotted names of the keys present in the file but absent from [`KEYS`].
    /// They are reported, never deleted.
    pub unknown: Vec<String>,
}

/// Path of `config.toml`: `$XDG_CONFIG_HOME/ariane/config.toml`, falling back
/// to `~/.config/ariane/config.toml` (PLAN §9).
pub fn file() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_default();
    base.join("ariane").join("config.toml")
}

/// Reads `file`. A missing file is the default configuration, not an error.
///
/// # Errors
///
/// Fails when the file exists but cannot be read, is not valid TOML, or holds a
/// value of the wrong type for a known key.
pub fn load(file: &Path) -> Result<Loaded> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Loaded::default());
        }
        Err(error) => return Err(error).with_context(|| format!("reading {}", file.display())),
    };
    parse(&text).with_context(|| format!("parsing {}", file.display()))
}

/// Parses the text of a configuration file.
///
/// # Errors
///
/// Fails on invalid TOML or a value of the wrong type for a known key.
pub fn parse(text: &str) -> Result<Loaded> {
    let document: DocumentMut = text.parse()?;
    let config: Config = toml::from_str(text)?;
    if config.server.port == 0 {
        bail!("`server.port` must be between 1 and 65535");
    }
    if config.proxy.port == 0 {
        bail!("`proxy.port` must be between 1 and 65535");
    }
    if !is_host(&config.server.host) {
        bail!(
            "`server.host` must be a host name such as `ariane.local`, not `{}`",
            config.server.host
        );
    }
    Ok(Loaded {
        config,
        unknown: unknown_keys(&document),
    })
}

/// The key of [`KEYS`] named `name`.
pub fn key(name: &str) -> Option<&'static Key> {
    KEYS.iter().find(|key| key.name == name)
}

/// The commented example file, generated from [`KEYS`].
///
/// With `repo`, the `repo` key is written with that value instead of a
/// commented-out example: this is what `ariane config init --repo` writes.
pub fn example(repo: Option<&str>) -> String {
    let mut text = String::from(
        "# Ariane configuration (ADR-0013).\n\
         # Generated from the schema compiled into `ariane`: `ariane config example`.\n",
    );
    let mut table = "";
    for key in KEYS {
        let (section, name) = key.name.rsplit_once('.').unwrap_or(("", key.name));
        if section != table {
            text.push_str(&format!("\n[{section}]\n"));
            table = section;
        } else {
            text.push('\n');
        }
        text.push_str(&format!("# {}\n", key.doc));
        match (key.name, repo, key.default, key.example) {
            ("repo", Some(repo), _, _) => {
                text.push_str(&format!("{name} = {}\n", toml_edit::Value::from(repo)));
            }
            (_, _, Some(default), _) => text.push_str(&format!("{name} = {default}\n")),
            (_, _, None, Some(example)) => text.push_str(&format!("# {name} = {example}\n")),
            (_, _, None, None) => {}
        }
    }
    text
}

/// Tells whether `name` is a host name the front door can route: labels of
/// lowercase ASCII letters, digits and inner hyphens, separated by dots, at
/// most 253 characters in all.
pub fn is_host(name: &str) -> bool {
    name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

/// Expands a leading `~/` into the home directory.
pub fn expand_home(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(rest))
            .unwrap_or_else(|| PathBuf::from(path)),
        None => PathBuf::from(path),
    }
}

/// Dotted names of the keys of `document` that [`KEYS`] does not declare.
///
/// A table is known when some key lives under it; anything else at any depth
/// is unknown. The walk uses `Item::as_table_like`, which covers both
/// `[table]` headers and inline `{ … }` tables.
fn unknown_keys(document: &DocumentMut) -> Vec<String> {
    let mut unknown = Vec::new();
    walk(document.as_item(), "", &mut unknown);
    unknown
}

/// Collects the unknown keys under `item`, whose dotted name is `prefix`.
fn walk(item: &Item, prefix: &str, unknown: &mut Vec<String>) {
    let Some(table) = item.as_table_like() else {
        return;
    };
    for (name, child) in table.iter() {
        let dotted = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}.{name}")
        };
        if key(&dotted).is_some() {
            continue;
        }
        let is_known_table = KEYS
            .iter()
            .any(|key| key.name.starts_with(&format!("{dotted}.")));
        if is_known_table && child.is_table_like() {
            walk(child, &dotted, unknown);
        } else {
            unknown.push(dotted);
        }
    }
}

/// Unit tests of the configuration schema.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_committed_example_is_the_generated_one() {
        let committed = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("config.example.toml"),
        )
        .unwrap();
        assert_eq!(
            committed,
            example(None),
            "config.example.toml is stale: regenerate it with `cargo run -- config example > config.example.toml`"
        );
    }

    #[test]
    fn the_example_parses_into_the_defaults() {
        let loaded = parse(&example(None)).unwrap();
        assert_eq!(loaded.config, Config::default());
        assert!(loaded.unknown.is_empty());
    }

    #[test]
    fn an_example_with_a_repository_sets_it() {
        let loaded = parse(&example(Some("~/content"))).unwrap();
        assert_eq!(loaded.config.repo.as_deref(), Some("~/content"));
    }

    #[test]
    fn a_missing_file_is_the_default_configuration() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load(&dir.path().join("config.toml")).unwrap(),
            Loaded::default()
        );
    }

    #[test]
    fn missing_keys_take_their_defaults() {
        let loaded = parse("repo = \"~/content\"\n").unwrap();
        assert_eq!(loaded.config.server.port, DEFAULT_PORT);
        assert_eq!(loaded.config.version, VERSION);
    }

    #[test]
    fn unknown_keys_are_reported_at_any_depth() {
        let loaded =
            parse("colour = 1\n[server]\nport = 9000\naddress = \"x\"\n[extra]\na = 1\n").unwrap();
        assert_eq!(loaded.unknown, ["colour", "server.address", "extra"]);
        assert_eq!(loaded.config.server.port, 9000);
    }

    #[test]
    fn a_value_of_the_wrong_type_is_an_error() {
        assert!(parse("[server]\nport = \"x\"\n").is_err());
        assert!(parse("[server]\nport = 0\n").is_err());
        assert!(parse("[server]\nhost = \"Ariane Local\"\n").is_err());
        assert!(parse("[proxy]\nport = 0\n").is_err());
    }

    #[test]
    fn host_names_are_lowercase_dotted_labels() {
        assert!(is_host("ariane.local"));
        assert!(is_host("dream-catchr.local"));
        assert!(is_host("localhost"));
        assert!(!is_host(""));
        assert!(!is_host("Ariane.local"));
        assert!(!is_host("ariane..local"));
        assert!(!is_host("-ariane.local"));
        assert!(!is_host("ariane.local:80"));
    }

    #[test]
    fn the_configured_repository_expands_the_home_directory() {
        let loaded = parse("repo = \"~/content\"\n").unwrap();
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap();
        assert_eq!(loaded.config.repo_path(), Some(home.join("content")));
    }
}

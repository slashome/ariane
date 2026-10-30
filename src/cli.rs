//! Command-line interface of `ariane`, declared with `clap`'s derive macros.
//!
//! Each struct below becomes a parser: `#[derive(Parser)]` for the top-level
//! command, `#[derive(Subcommand)]` for the list of commands and `#[derive(Args)]`
//! for the arguments of one command. The `about` and `help` attributes are the
//! texts shown by `ariane --help`; they are set explicitly so that the rustdoc on
//! these items never leaks into the help output.
//!
//! Struct fields carry no rustdoc of their own, and each struct keeps a
//! one-paragraph doc: on a `clap` item, a field doc or a multi-paragraph doc
//! becomes help text. `repo` is the content repository given by `--repo` or
//! `ARIANE_REPO`; `global = true` makes it accepted after any command, and
//! `None` means "discover it from the current directory".

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::item::Kind;

/// The whole command line: the global `repo` option and the chosen command.
#[derive(Parser)]
#[command(
    name = "ariane",
    version,
    about = "Ariadne's thread through your projects",
    arg_required_else_help = true
)]
pub struct Cli {
    #[arg(long, global = true, env = "ARIANE_REPO")]
    pub repo: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

/// The commands `ariane` understands.
#[derive(Subcommand)]
pub enum Command {
    #[command(about = "List the items of the tree")]
    Tasks(TasksArgs),
    #[command(
        about = "Find items by id, reference, title or body, ignoring case and accents, best first"
    )]
    Search(SearchArgs),
    #[command(about = "Show one item: its header, its body, its parent, children and dependencies")]
    Show(ShowArgs),
    #[command(
        about = "Attach the current agent session to an item: its branches and pull requests go there"
    )]
    Focus(FocusArgs),
    #[command(
        about = "Record in each item the pull requests that name it (monday id or <node>#<id>), from GitHub"
    )]
    Links(LinksArgs),
    #[command(about = "Run as an agent hook, reading the event on standard input")]
    Hook(HookArgs),
    #[command(about = "Store a tracker's API token, one per account, in secrets.toml")]
    Connect(ConnectArgs),
    #[command(
        about = "The people items may wait for: list, add, change, rename (items rewritten), remove"
    )]
    People(PeopleArgs),
    #[command(
        about = "Send the items' statuses to their tracker (monday), refusing to guess on conflicts"
    )]
    Sync(SyncArgs),
    #[command(
        about = "Check every item and node against the specification, without writing anything"
    )]
    Doctor(DoctorArgs),
    #[command(about = "Bring items to the current schema version")]
    Migrate(MigrateArgs),
    #[command(
        about = "Apply the repairs doctor names: item files named after their id, links kept"
    )]
    Fix(FixArgs),
    #[command(about = "Print the item header schema")]
    Schema(SchemaArgs),
    #[command(
        about = "Serve the web interface on 127.0.0.1, and open the front door of `ariane proxy`"
    )]
    Serve(ServeArgs),
    #[command(
        about = "Fast-forward the content repository to its upstream, never creating a commit"
    )]
    Pull,
    #[command(
        about = "Reach the interface and the projects' dev servers by name, such as http://ariane.local"
    )]
    Proxy(ProxyArgs),
    #[command(about = "Create, read and change the configuration file")]
    Config(ConfigArgs),
    #[command(
        about = "Read and change the settings versioned with the content: theme, dashboard pins"
    )]
    Settings(SettingsArgs),
    #[command(about = "Keep `ariane serve` running in the background (launchd, systemd)")]
    Service(ServiceArgs),
}

/// Arguments of `ariane tasks`: an optional node scope, whether to include
/// archived items, the kinds to keep, whether to add bodies and dates, and
/// `json` to print JSON instead of a table.
#[derive(Args)]
pub struct TasksArgs {
    #[arg(
        long,
        help = "Only the items of this node and its descendants (default: the node of the current directory)"
    )]
    pub node: Option<String>,

    #[arg(
        long,
        conflicts_with = "node",
        help = "Cover the whole tree, wherever the command runs"
    )]
    pub all: bool,

    #[arg(long, help = "Include archived items")]
    pub archived: bool,

    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        help = "Only items of these kinds, such as `task,story` (default: every kind)"
    )]
    pub kind: Vec<Kind>,

    #[arg(long, help = "Add each item's Markdown body to the JSON rows")]
    pub body: bool,

    #[arg(long, help = "Add the date of each item's latest commit (`updated`)")]
    pub dates: bool,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane search`: the words to find, an optional node scope,
/// the kinds to keep, whether to include archived items, and `json` to print
/// the results as JSON.
#[derive(Args)]
pub struct SearchArgs {
    #[arg(
        required = true,
        help = "The words to find; an item must hold every one of them"
    )]
    pub text: Vec<String>,

    #[arg(
        long,
        help = "Only the items of this node and its descendants (default: the node of the current directory)"
    )]
    pub node: Option<String>,

    #[arg(
        long,
        conflicts_with = "node",
        help = "Cover the whole tree, wherever the command runs"
    )]
    pub all: bool,

    #[arg(long, help = "Include archived items")]
    pub archived: bool,

    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        help = "Only items of these kinds, such as `task,story` (default: every kind)"
    )]
    pub kind: Vec<Kind>,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane show`: the item reference, and `json` to print the
/// header, body and relations as JSON.
#[derive(Args)]
pub struct ShowArgs {
    #[arg(help = "The item: `<node>#<id>`, or a bare id within the node of the current directory")]
    pub reference: String,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane focus`: the item, and the agent session to attach to
/// it, which the hook passes.
#[derive(Args)]
pub struct FocusArgs {
    #[arg(help = "The item: `<node>#<id>`, or a bare id within the node of the current directory")]
    pub reference: String,

    #[arg(
        long,
        help = "The agent session to attach (the Claude Code hook notes it on its own)"
    )]
    pub session: Option<String>,
}

/// Arguments of `ariane links`: an optional node scope, `apply` to write (a
/// dry run otherwise), and `json` to print the proposals as JSON.
#[derive(Args)]
pub struct LinksArgs {
    #[arg(
        long,
        help = "Only the items of this node and its descendants (default: the node of the current directory)"
    )]
    pub node: Option<String>,

    #[arg(
        long,
        conflicts_with = "node",
        help = "Cover the whole tree, wherever the command runs"
    )]
    pub all: bool,

    #[arg(long, help = "Only this item, `<node>#<id>`")]
    pub item: Option<String>,

    #[arg(
        long,
        help = "Record the pull requests found. Without it, print them and write nothing"
    )]
    pub apply: bool,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane people`: a subcommand, or none to list; a node scope
/// and `json` for the list.
#[derive(Args)]
pub struct PeopleArgs {
    #[command(subcommand)]
    pub command: Option<PeopleCommand>,

    #[arg(long, help = "Only the directories that concern this node")]
    pub node: Option<String>,

    #[arg(long)]
    pub json: bool,
}

/// The subcommands of `ariane people`. Each names the node that declares the
/// person (the one with the `[tracker]`).
#[derive(Subcommand)]
pub enum PeopleCommand {
    #[command(about = "Declare a person in a node's node.toml")]
    Add {
        node: String,
        handle: String,
        #[arg(long)]
        name: String,
        #[arg(long, help = "The person's monday user id")]
        monday: Option<i64>,
    },
    #[command(about = "Change a person's name or monday id; no item changes")]
    Set {
        node: String,
        handle: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        monday: Option<i64>,
        #[arg(long, conflicts_with = "monday", help = "Remove the monday id")]
        no_monday: bool,
    },
    #[command(about = "Rename a handle, in node.toml and in every item that waits for the person")]
    Rename {
        node: String,
        handle: String,
        to: String,
    },
    #[command(about = "Remove a person no item waits for any more")]
    Remove { node: String, handle: String },
    #[command(about = "Find monday users by name, to get their id")]
    Find {
        #[arg(long)]
        account: String,
        name: String,
    },
}

/// The trackers Ariane has a connector for (ADR-0011).
#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Connector {
    Monday,
}

/// Arguments of `ariane connect`: the tracker and the account whose token to store.
#[derive(Args)]
pub struct ConnectArgs {
    #[arg(value_enum)]
    pub connector: Connector,

    #[arg(
        long,
        help = "The account, as the node's [tracker] names it (`account = …`)"
    )]
    pub account: String,
}

/// Arguments of `ariane sync`: the tracker, a node scope or one item, `apply`
/// to send (a dry run otherwise), and `json` to print the plan as JSON.
#[derive(Args)]
pub struct SyncArgs {
    #[arg(value_enum)]
    pub connector: Connector,

    #[arg(
        long,
        help = "Only the items of this node and its descendants (default: the node of the current directory)"
    )]
    pub node: Option<String>,

    #[arg(
        long,
        conflicts_with = "node",
        help = "Cover the whole tree, wherever the command runs"
    )]
    pub all: bool,

    #[arg(long, help = "Only this item, `<node>#<id>`")]
    pub item: Option<String>,

    #[arg(
        long,
        help = "Update the tracker. Without it, print what would change and send nothing"
    )]
    pub apply: bool,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane hook`: the agent harness that runs it.
#[derive(Args)]
pub struct HookArgs {
    #[command(subcommand)]
    pub harness: HookHarness,
}

/// The harnesses `ariane hook` knows.
#[derive(Subcommand)]
pub enum HookHarness {
    #[command(
        about = "Claude Code `PostToolUse` hook on Bash: notes `ariane focus`, records branches and pull requests"
    )]
    ClaudeCode,
}

/// Arguments of `ariane doctor`: an optional node scope, `strict` to fail on
/// warnings, and `json` to print the findings as JSON.
#[derive(Args)]
pub struct DoctorArgs {
    #[arg(
        long,
        help = "Only check this node and its descendants (default: the node of the current directory)"
    )]
    pub node: Option<String>,

    #[arg(
        long,
        conflicts_with = "node",
        help = "Cover the whole tree, wherever the command runs"
    )]
    pub all: bool,

    #[arg(long, help = "Treat warnings as errors")]
    pub strict: bool,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane migrate`: an optional node scope, `apply` to write the
/// files (a dry run otherwise), and `json` to print the plan as JSON.
#[derive(Args)]
pub struct MigrateArgs {
    #[arg(
        long,
        help = "Only migrate this node and its descendants (default: the node of the current directory)"
    )]
    pub node: Option<String>,

    #[arg(
        long,
        conflicts_with = "node",
        help = "Cover the whole tree, wherever the command runs"
    )]
    pub all: bool,

    #[arg(
        long,
        help = "Write the migrated files. Without it, print the changes and write nothing"
    )]
    pub apply: bool,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane fix`: an optional node scope, `apply` to write the
/// repairs (a dry run otherwise), and `json` to print the plan as JSON.
#[derive(Args)]
pub struct FixArgs {
    #[arg(
        long,
        help = "Only repair the items of this node and its descendants (default: the node of the current directory)"
    )]
    pub node: Option<String>,

    #[arg(
        long,
        conflicts_with = "node",
        help = "Cover the whole tree, wherever the command runs"
    )]
    pub all: bool,

    #[arg(
        long,
        help = "Write the repairs. Without it, print the plan and write nothing"
    )]
    pub apply: bool,

    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane schema`: `json` to print the schema as JSON.
#[derive(Args)]
pub struct SchemaArgs {
    #[arg(long)]
    pub json: bool,
}

/// Arguments of `ariane serve`: the port to listen on, on the loopback interface
/// only (ADR-0012), and whether to open the browser once listening.
#[derive(Args)]
pub struct ServeArgs {
    #[arg(
        long,
        help = "Port to listen on, on 127.0.0.1 (default: `server.port` of the configuration, 27317)"
    )]
    pub port: Option<u16>,

    #[arg(
        long,
        help = "Open the interface in the browser once listening: by its name, http://<server.host>, when `ariane proxy setup` made it reach the front door, else on 127.0.0.1"
    )]
    pub open: bool,
}

/// Arguments of `ariane proxy`: a subcommand, or the nodes whose routes the
/// front door serves alone, all of them when none is named.
#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct ProxyArgs {
    #[command(subcommand)]
    pub command: Option<ProxyCommand>,

    #[arg(
        help = "Only the routes of these nodes, by path or last segment (default: every route); the interface's needs `ariane serve`"
    )]
    pub nodes: Vec<String>,
}

/// The subcommands of `ariane proxy`.
#[derive(Subcommand)]
pub enum ProxyCommand {
    #[command(
        about = "With sudo, until the next reboot: alias 127.0.0.2, redirect its port 80 to the front door, name the routes in /etc/hosts"
    )]
    Setup,
    #[command(
        about = "Check each link: the alias, the redirect, the front door, the names, each route's server"
    )]
    Status,
    #[command(about = "With sudo: undo what setup did")]
    Teardown,
}

/// Arguments of `ariane config`: the subcommand to run.
#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// The subcommands of `ariane config`. `Init` takes `force` to overwrite an
/// existing file, and sets `repo` from the global `--repo`: an option of its
/// own under the same name would clash with it; `Get` and `Set` take a dotted
/// key such as `server.port`.
#[derive(Subcommand)]
pub enum ConfigCommand {
    #[command(about = "Print the path of the configuration file")]
    Path,
    #[command(
        about = "Create the configuration file with its defaults, and `repo` set from --repo or ARIANE_REPO"
    )]
    Init {
        #[arg(long, help = "Overwrite an existing file")]
        force: bool,
    },
    #[command(about = "Print the value of a key, such as server.port")]
    Get {
        #[arg(help = "Dotted key, such as server.port")]
        key: String,
    },
    #[command(about = "Set a key, keeping the file's comments and layout")]
    Set {
        #[arg(help = "Dotted key, such as server.port")]
        key: String,
        #[arg(help = "New value; a list is written as `a, b` or `[\"a\", \"b\"]`")]
        value: String,
    },
    #[command(about = "Open the configuration in $VISUAL or $EDITOR, then validate it")]
    Edit,
    #[command(about = "Print the commented example generated from the schema")]
    Example,
}

/// Arguments of `ariane settings`: an optional subcommand, and `json` to print
/// the settings as JSON.
#[derive(Args)]
pub struct SettingsArgs {
    #[command(subcommand)]
    pub command: Option<SettingsCommand>,

    #[arg(long)]
    pub json: bool,
}

/// The subcommands of `ariane settings`. `Set` takes a dotted key such as
/// `theme.primary`.
#[derive(Subcommand)]
pub enum SettingsCommand {
    #[command(
        about = "Set a key: theme.primary, theme.secondary (#rrggbb) or dashboard.pinned (a, b)"
    )]
    Set {
        #[arg(help = "Dotted key, such as theme.primary")]
        key: String,
        #[arg(help = "New value: a color `#rrggbb`, or nodes written `a, b`")]
        value: String,
    },
}

/// Arguments of `ariane service`: the subcommand to run.
#[derive(Args)]
pub struct ServiceArgs {
    #[command(subcommand)]
    pub command: ServiceCommand,
}

/// The subcommands of `ariane service`. `Logs` takes `follow` to keep
/// printing new lines.
#[derive(Subcommand)]
pub enum ServiceCommand {
    #[command(about = "Install and start the service; it restarts at login and after a crash")]
    Install,
    #[command(about = "Stop the service and remove it")]
    Uninstall,
    #[command(about = "Tell whether the service is installed and running")]
    Status,
    #[command(about = "Print the last lines of the server's log")]
    Logs {
        #[arg(long, short, help = "Keep printing new lines")]
        follow: bool,
    },
}

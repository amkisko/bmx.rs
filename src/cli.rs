use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "bmx")]
#[command(about = "Best-effort app runner/installer from git or URLs")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,

    /// Use a throwaway `BMX_HOME` under the temp dir and delete it after the process (no persistent cache).
    #[arg(long, global = true)]
    pub(crate) rm: bool,

    /// Resolve the app from `.bmx/pin` in the current directory or a parent (see SPEC).
    #[arg(long, global = true)]
    pub(crate) pin: bool,

    /// Print resolved app id, binary path, and commit when running; show git and build output (otherwise quiet / spinner).
    #[arg(short, long, global = true)]
    pub(crate) verbose: bool,

    #[arg(help = "App/repo name to run (default command)")]
    pub(crate) app: Option<String>,

    #[arg(num_args = 0.., allow_hyphen_values = true)]
    pub(crate) args: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    Exec {
        /// App spec, or omit with `--pin` to use `.bmx/pin`.
        app: Option<String>,
        #[arg(num_args = 0.., allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Install under ~/.bmx/apps/<NAME>/ when you need a unique id (two repos that share the same default slot, or a second copy of the same source).
    Install {
        #[arg(long = "as", value_name = "NAME")]
        install_as: Option<String>,
        app: String,
    },
    Uninstall {
        app: String,
    },
    Reinstall {
        app: String,
    },
    SelfUpdate {
        app: String,
    },
    /// Rebuild from the cached clone: one installed app (errors if that app is not installed), or every install when APP is omitted.
    Update {
        app: Option<String>,
    },
    /// Print files under the install’s checkout, or with --would-remove the paths `bmx uninstall` would delete.
    Show {
        #[arg(
            long = "would-remove",
            help = "Dry run: list paths that uninstall would remove"
        )]
        would_remove: bool,
        app: String,
    },
    Source {
        #[command(subcommand)]
        command: SourceCommands,
    },
    Isolation {
        #[command(subcommand)]
        command: IsolationCommands,
    },
    Checkout {
        #[command(subcommand)]
        command: CheckoutCommands,
    },
    Shim {
        #[command(subcommand)]
        command: ShimCommands,
    },
    Doctor,
    /// Search for sources bmx can install: GitHub/GitLab (with optional root-file probe), AUR, or Homebrew (homepage → git URL).
    Search {
        /// Words passed to the search backend (combined with spaces).
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        #[arg(long, value_enum, default_value_t = SearchBackendArg::Auto)]
        backend: SearchBackendArg,
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        forks: bool,
        #[arg(long)]
        url: bool,
        /// List GitHub/GitLab hits without checking the API for root build files (Cargo.toml, PKGBUILD, …).
        #[arg(long)]
        no_probe: bool,
    },
    /// Show recent logged actions (see `bmx undo`).
    History {
        #[arg(short = 'n', long, default_value_t = 30)]
        limit: usize,
    },
    /// Roll back the most recent undoable action (`install` / `update` / `reinstall` / `update all`), or a specific id from `bmx history`.
    Undo {
        /// Action id (must be the latest undoable step unless you undo in order).
        id: Option<String>,
        /// Undo just this install from the **top** undo frame (e.g. one app after `bmx update` with no app argument). Not with `id`.
        #[arg(long = "only", value_name = "APP")]
        only_app: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ShimCommands {
    /// Create ~/.bmx/shims (prints the directory).
    Init,
    /// Print a POSIX `export PATH=...` snippet for ~/.bmx/shims.
    Path,
    /// Install a tiny shell stub that runs `bmx exec <app>` (Unix only).
    Add {
        app: String,
        /// Shim filename (default: derived app id).
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum SourceCommands {
    SetDefault { url: String },
    Show,
}

#[derive(Subcommand, Debug)]
pub(crate) enum IsolationCommands {
    SetDefault { mode: String },
    Show,
}

#[derive(Subcommand, Debug)]
pub(crate) enum CheckoutCommands {
    SetDefault { backend: String },
    Show,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub(crate) enum SearchBackendArg {
    /// Use `default_source` host (GitHub or GitLab API).
    #[default]
    Auto,
    /// GitHub repository search (`api.github.com`); short names resolve against `https://github.com`.
    Github,
    /// GitLab project search on `default_source` origin.
    Gitlab,
    /// AUR RPC (`aur.archlinux.org`); each hit is a PKGBUILD git clone URL.
    Aur,
    /// Homebrew formula index (`formulae.brew.sh`); only formulas whose homepage maps to a cloneable git URL.
    Homebrew,
}

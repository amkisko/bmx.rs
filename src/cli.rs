use clap::{Parser, Subcommand};

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
    Install {
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
    Update {
        app: Option<String>,
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

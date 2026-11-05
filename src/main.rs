mod commands;
mod config;
mod error;
mod fs_ops;

use clap::{Parser, Subcommand};
use colored::Colorize;

use crate::config::Paths;
use crate::error::Result;

#[derive(Parser)]
#[command(
    name = "jfs",
    version,
    about = "jfs ☕︎ Windows Java version manager for Java fans",
    arg_required_else_help = true,
    after_help = "\
JAVA_HOME stays pointed at %LOCALAPPDATA%\\jfs\\current.
Switching versions retargets that one NTFS junction, so new java processes pick it up immediately.

Examples:
  jfs add 21 \"C:\\Program Files\\Eclipse Adoptium\\jdk-21\"
  jfs list
  jfs use 21
  jfs current
  jfs rm 21"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List registered JDKs
    #[command(alias = "ls")]
    List,
    /// Make a registered JDK the active one
    Use {
        /// Registered version name
        version: String,
    },
    /// Register a local JDK directory
    Add {
        /// Version name, for example 21
        name: String,
        /// Path to the JDK directory that contains bin\java.exe
        path: String,
    },
    /// Unregister a JDK without deleting its files
    #[command(alias = "remove")]
    Rm {
        /// Registered version name
        version: String,
    },
    /// Print the active version name
    Current,
}

fn main() {
    #[cfg(windows)]
    let _ = colored::control::set_virtual_terminal(true);

    if let Err(err) = dispatch() {
        eprintln!("{} {err}", "error:".red().bold());
        std::process::exit(1);
    }
}

fn dispatch() -> Result<()> {
    let cli = Cli::parse();
    let paths = Paths::resolve()?;
    match cli.command {
        Commands::List => commands::list(&paths),
        Commands::Use { version } => commands::use_version(&paths, &version),
        Commands::Add { name, path } => commands::add(&paths, &name, &path),
        Commands::Rm { version } => commands::remove_version(&paths, &version),
        Commands::Current => commands::current(&paths),
    }
}

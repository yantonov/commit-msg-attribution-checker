mod commands;
mod config;
mod defaults;
mod engine;
mod identity;
mod message;
mod resolve;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, error::ErrorKind};

#[derive(Parser)]
#[command(name = "commit-msg-attribution-checker", version)]
struct Cli {
    /// Path to the config file
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check a commit message file
    Verify {
        /// Message file to check
        msg_file: PathBuf,

        /// Identity to test `author` patterns against instead of `git var`
        #[arg(long, value_name = "IDENT")]
        author: Option<String>,

        /// Identity to test `committer` patterns against instead of `git var`
        #[arg(long, value_name = "IDENT")]
        committer: Option<String>,

        /// Report the identity targets that were skipped
        #[arg(short, long)]
        verbose: bool,
    },

    /// Inspect or write the configuration
    #[command(arg_required_else_help = true)]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Write the built-in default config as TOML to stdout
    Defaults,

    /// Write the built-in default config to the resolved path
    Create,

    /// Write the effective config to stdout
    Show,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) if err.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            // spec §6: a bare `config` lists the subcommands on stderr, where clap would use stdout.
            eprint!("{err}");
            return ExitCode::from(2);
        }
        Err(err) => err.exit(),
    };

    match cli.command {
        Commands::Verify {
            msg_file,
            author,
            committer,
            verbose,
        } => commands::verify(
            &msg_file,
            // The program and the directory git runs in are inputs the command
            // never reads itself, which is what makes them testable.
            &identity::Inputs::new(author.as_deref(), committer.as_deref()),
            verbose,
        ),
        Commands::Config {
            action: ConfigAction::Defaults,
        } => commands::config_defaults(),
        Commands::Config {
            action: ConfigAction::Create,
        } => commands::config_create(cli.config),
        Commands::Config {
            action: ConfigAction::Show,
        } => commands::config_show(cli.config),
    }
}

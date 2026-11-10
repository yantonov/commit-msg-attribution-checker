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
        Commands::Verify { .. } => not_implemented("verify"),
        Commands::Config {
            action: ConfigAction::Defaults,
        } => not_implemented("config defaults"),
        Commands::Config {
            action: ConfigAction::Create,
        } => not_implemented("config create"),
        Commands::Config {
            action: ConfigAction::Show,
        } => not_implemented("config show"),
    }
}

fn not_implemented(command: &str) -> ExitCode {
    eprintln!("commit-msg-attribution-checker: {command}: not implemented");
    ExitCode::from(2)
}

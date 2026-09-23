#![forbid(unsafe_code)]
#![recursion_limit = "512"]

use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};

mod database_gen;
mod docker_api_gen;
mod openapi_gen;
mod parity;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate the complete pinned Docker v1.49 protocol crate.
    DockerApi {
        /// Regenerate in a temporary directory and reject any source differences.
        #[arg(long)]
        check: bool,
    },
    /// Check all reference .NET HTTP operation IDs, routes, and public exposure.
    Parity,
    /// Manage the Rust-owned database schema and migration artifacts.
    Database {
        #[command(subcommand)]
        command: DatabaseCommand,
    },
    /// Generate deterministic full/public OpenAPI and foundation frontend artifacts.
    Openapi {
        /// Exit non-zero when generated API artifacts differ from source control.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand)]
enum DatabaseCommand {
    /// Verify the Rust schema, migration catalog, and checksums without .NET inputs.
    Verify,
    /// Recreate the one-time Rust-v1 baseline from the frozen accepted .NET input.
    ImportBaseline,
    /// Regenerate the unreleased Rust baseline from the declarative schema.
    RefreshBaseline,
    /// Refresh the embedded baseline checksum and schema hash.
    RefreshCatalog,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::DockerApi { check } => docker_api_gen::generate(check),
        Command::Database { command } => match command {
            DatabaseCommand::Verify => database_gen::verify(),
            DatabaseCommand::ImportBaseline => database_gen::import_baseline(),
            DatabaseCommand::RefreshBaseline => database_gen::refresh_baseline(),
            DatabaseCommand::RefreshCatalog => database_gen::refresh_catalog(),
        },
        Command::Openapi { check } => openapi_gen::generate(check),
        Command::Parity => parity::check(),
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

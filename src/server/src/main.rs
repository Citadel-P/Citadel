mod app;
mod cli;
#[cfg(target_os = "linux")]
mod cli_updates;
mod composition;
mod logging;
mod restart;
mod router;
mod startup;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cli::run()
}

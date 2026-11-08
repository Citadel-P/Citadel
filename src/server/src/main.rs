mod app;
mod cli;
#[cfg(target_os = "linux")]
mod cli_updates;
mod composition;
mod logging;
mod router;
mod startup;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run().await
}

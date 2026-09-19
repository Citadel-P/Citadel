mod app;
mod cli;
mod jobs;
mod router;
mod startup;
mod state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run().await
}

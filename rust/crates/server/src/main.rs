mod app;
mod cli;
mod composition;
mod jobs;
mod router;
mod startup;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run().await
}

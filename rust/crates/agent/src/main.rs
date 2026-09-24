#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => citadel_agent::app::run().await,
        [command] if command == "--version" => {
            println!("citadel-agent {}", citadel_agent::INFORMATIONAL_VERSION);
            Ok(())
        }
        [command] if command == "healthcheck" => citadel_agent::cli::healthcheck().await,
        _ => Err("Usage: citadel-agent [--version|healthcheck]".into()),
    }
}

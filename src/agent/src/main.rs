use std::error::Error;

enum Command<'a> {
    Serve,
    Version,
    VersionJson,
    Healthcheck,
    Replace {
        source: &'a str,
        source_image: &'a str,
        image: &'a str,
        operation: &'a str,
    },
}

fn runtime(command: &Command<'_>) -> std::io::Result<tokio::runtime::Runtime> {
    let mut builder = if matches!(command, Command::Healthcheck) {
        tokio::runtime::Builder::new_current_thread()
    } else {
        tokio::runtime::Builder::new_multi_thread()
    };
    builder.enable_all().build()
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let command = match args.as_slice() {
        [command, source, source_image, image, operation] if command == "replace-agent" => {
            Command::Replace {
                source: source.to_str().ok_or("Invalid source")?,
                source_image: source_image.to_str().ok_or("Invalid image")?,
                image: image.to_str().ok_or("Invalid image")?,
                operation: operation.to_str().ok_or("Invalid operation")?,
            }
        }
        [] => Command::Serve,
        [command] if command == "--version" => Command::Version,
        [command] if command == "version-json" => Command::VersionJson,
        [command] if command == "healthcheck" => Command::Healthcheck,
        _ => return Err("Usage: citadel-agent [--version|version-json|healthcheck]".into()),
    };
    runtime(&command)?.block_on(dispatch(command))
}

async fn dispatch(command: Command<'_>) -> Result<(), Box<dyn Error + Send + Sync>> {
    match command {
        Command::Replace {
            source,
            source_image,
            image,
            operation,
        } => citadel_agent::updates::replace(source, source_image, image, operation).await,
        Command::Serve => citadel_agent::app::run().await,
        Command::Version => {
            println!("citadel-agent {}", citadel_agent::INFORMATIONAL_VERSION);
            Ok(())
        }
        Command::VersionJson => {
            println!(
                "{}",
                serde_json::json!({"version": citadel_agent::VERSION, "informationalVersion": citadel_agent::INFORMATIONAL_VERSION, "protocolVersion": citadel_contracts::EDGE_AGENT_PROTOCOL_VERSION})
            );
            Ok(())
        }
        Command::Healthcheck => citadel_agent::cli::healthcheck().await,
    }
}

#[test]
fn runtime_flavor_preserves_multithreaded_serving() {
    use tokio::runtime::RuntimeFlavor;
    assert_eq!(
        runtime(&Command::Serve).unwrap().handle().runtime_flavor(),
        RuntimeFlavor::MultiThread
    );
    assert_eq!(
        runtime(&Command::Healthcheck)
            .unwrap()
            .handle()
            .runtime_flavor(),
        RuntimeFlavor::CurrentThread
    );
}

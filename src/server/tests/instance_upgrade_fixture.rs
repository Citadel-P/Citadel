//! Test-only Core executable. Reuses the complete server and helper lifecycle;
//! only release discovery/trust comes from a private fixture instead of GitHub.
// The shared modules include unit-test helpers, but this executable has no test harness.
#![allow(dead_code, unused_imports)]
#[cfg(target_os = "linux")]
#[path = "../src/app.rs"]
mod app;
#[cfg(target_os = "linux")]
#[path = "../src/cli.rs"]
mod cli;
#[cfg(target_os = "linux")]
#[path = "../src/cli_updates.rs"]
mod cli_updates;
#[cfg(target_os = "linux")]
#[path = "../src/composition/mod.rs"]
mod composition;
#[cfg(target_os = "linux")]
#[path = "../src/router.rs"]
mod router;
#[cfg(target_os = "linux")]
#[path = "../src/startup.rs"]
mod startup;

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ordinary cargo test must not start a server. The Docker-only runner puts
    // this file inside its explicitly unpublishable test images.
    if std::path::Path::new("/app/upgrade-fixture.json").is_file() {
        app::run().await?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {}

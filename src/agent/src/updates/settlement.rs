use serde_json::{Value, json};

pub(crate) fn helper_name(operation: &str, source: &str) -> String {
    format!(
        "citadel-agent-updater-{operation}-{}",
        &source[..12.min(source.len())]
    )
}

/// After helper dispatch, only its completion and authenticated replacement establish settlement.
/// Source-side failures after dispatch stay owned: a timeout is not cancellation
/// of the Docker mutations the helper may already have dispatched.
pub(crate) fn settled(
    current: &Value,
    helper: &Value,
    source: &str,
    source_image: &str,
    operation: &str,
) -> Result<(), &'static str> {
    let image = current["Config"]["Image"].as_str().unwrap_or_default();
    super::validate_image(image)?;
    if current["Id"] == source
        || current["State"]["Running"] != true
        || current["Config"]["Labels"]["com.citadel.agent-update"] != operation
        || helper["Name"] != format!("/{}", helper_name(operation, source))
        || helper["Config"]["Labels"]["com.citadel.system-role"] != "agent-updater"
        || helper["Config"]["Labels"]["com.citadel.agent-update"] != operation
        || helper["Config"]["Image"] != image
        || helper["Config"]["Entrypoint"] != json!(["/app/citadel-agent"])
        || helper["Config"]["Cmd"]
            != json!(["replace-agent", source, source_image, image, operation])
        || helper["State"]["Status"] != "exited"
        || helper["State"]["Running"] != false
        || helper["State"]["ExitCode"] != 0
    {
        return Err(
            "Agent update recovery required: the owned helper has not completed a verified replacement. Inspect the retained source and helper; do not start another update.",
        );
    }
    // Retain exited helpers as idempotent evidence for retries and Core restarts.
    Ok(())
}

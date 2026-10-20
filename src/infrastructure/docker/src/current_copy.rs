//! Exact one-time snapshot command shared with the Agent's helper allowlist.
// Restore the complete /source node onto /target/source, including the volume
// root's ownership and mode. A subtree restore would discard that metadata.
pub const CURRENT_COPY_SCRIPT: &str = "restic init --quiet\nrestic backup --quiet /source\nrestic restore --quiet latest --target /target";
pub const COPY_JOB_LABEL: &str = "citadel.copy.job-id";

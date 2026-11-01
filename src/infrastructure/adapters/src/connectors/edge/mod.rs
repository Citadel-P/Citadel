//! Existing Citadel Edge protobuf transport. Registration is internal to the
//! authenticated intake; execution always resolves an exact resource/node.
mod intake;
mod runtime;
mod sessions;

/// Observation commands are finite even when the session remains connected.
pub const OBSERVATION_STREAM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3600);

pub use intake::EdgeIntake;
pub use runtime::EdgeRuntime;
pub use sessions::{
    EdgeCommandStream, EdgeError, EdgeOutbound, EdgeRegistry, EdgeSession, EdgeTarget, MAX_PAYLOAD,
};

pub mod observations;

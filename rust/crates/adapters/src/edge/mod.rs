//! Existing Citadel Edge protobuf transport. Registration is internal to the
//! authenticated intake; execution always resolves an exact resource/node.
mod intake;
mod runtime;
mod sessions;
mod store;

pub use intake::EdgeIntake;
pub use runtime::EdgeRuntime;
pub use sessions::{
    EdgeCommandStream, EdgeError, EdgeRegistry, EdgeSession, EdgeTarget, MAX_PAYLOAD,
};
pub use store::{EdgeBinding, EdgeStatus, EdgeStoreError, PostgresEdgeStore};

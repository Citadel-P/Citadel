//! Central update execution uses existing authenticated Platform and Build Pool transports.
mod runtime;
pub use runtime::AgentUpdateRuntimeRouter;

mod swarm;

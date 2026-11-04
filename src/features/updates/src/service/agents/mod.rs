//! One persisted selection flow for manual and scheduled Agent updates.
mod coordinator;
mod schedule;
pub use coordinator::AgentUpdateService;
#[cfg(test)]
mod tests;

//! One persisted selection flow for manual and scheduled Agent updates.
mod coordinator;
pub use coordinator::AgentUpdateService;
#[cfg(test)]
mod tests;

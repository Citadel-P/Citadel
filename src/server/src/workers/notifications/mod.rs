//! Shared PostgreSQL notification fanout and worker wakeups.
mod hub;
mod scoped;
pub(crate) use hub::{DatabaseNotificationHub, wait};
pub(crate) use scoped::ScopedTopic;

#[cfg(test)]
mod postgres_tests;
#[cfg(test)]
mod tests;

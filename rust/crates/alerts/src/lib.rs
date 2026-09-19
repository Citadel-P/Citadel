#![forbid(unsafe_code)]
pub mod channels;
mod configuration;
pub mod configuration_patch;
mod error;
pub mod events;
pub mod permissions;
mod quiet_hours;
mod repository;
pub mod rules;
mod runtime;
mod service;
mod windows_time_zones;
pub use channels::{AlertChannel, AlertChannelConfiguration};
pub use error::AlertError;
pub use events::{AlertEvent, AlertEventFilter, AlertEventPage, NewAlertEvent};
pub use quiet_hours::is_in_quiet_hours;
pub use repository::AlertRepository;
pub use rules::{
    AlertRule, AlertRuleConfiguration, AlertRuleListItem, RenameAlertRuleInput, description_patch,
};
pub use runtime::{
    AlertDelivery, AlertDeliveryClaim, AlertEntitlements, AlertEventSink, AlertObservation,
};
pub use service::AlertDeliveryService;
#[cfg(test)]
mod tests;

use crate::*;
use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub trait AlertEntitlements: Send + Sync {
    fn advanced_alerting(&self) -> BoxFuture<'_, Result<bool, AlertError>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertObservation {
    pub alert_type: String,
    pub info: Value,
    pub resource_id: Uuid,
    pub resource_name: String,
    pub resource_type: String,
    pub deduplication_component: String,
    pub observed_at: DateTime<Utc>,
    pub value: Option<f64>,
    pub matched: bool,
}

pub trait AlertEventSink: Send + Sync {
    fn observe<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEvent>, AlertError>>;
}

impl<T> AlertEventSink for T
where
    T: AlertRepository + ?Sized,
{
    fn observe<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEvent>, AlertError>> {
        self.process_event(observation)
    }
}

pub trait AlertDelivery: Send + Sync {
    fn send<'a>(
        &'a self,
        channel: &'a AlertChannel,
        event: &'a AlertEvent,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), AlertError>>;
}

#[derive(Debug, Clone)]
pub struct AlertDeliveryClaim {
    pub id: Uuid,
    pub attempt_count: i32,
    pub channel: AlertChannel,
    pub event: AlertEvent,
}

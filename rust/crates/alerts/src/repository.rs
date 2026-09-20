use crate::*;
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde_json::Value;
use uuid::Uuid;

pub trait AlertRepository: Send + Sync {
    fn list_channels(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AlertChannel>, AlertError>>;
    fn get_channel(&self, id: Uuid) -> BoxFuture<'_, Result<AlertChannel, AlertError>>;
    fn create_channel<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AlertChannelConfiguration,
    ) -> BoxFuture<'a, Result<AlertChannel, AlertError>>;
    fn update_channel<'a>(
        &'a self,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<AlertChannel, AlertError>>;
    fn delete_channels<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), AlertError>>;
    fn list_rules(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AlertRuleListItem>, AlertError>>;
    fn get_rule(&self, id: Uuid) -> BoxFuture<'_, Result<AlertRule, AlertError>>;
    fn create_rule<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AlertRuleConfiguration,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>>;
    fn update_rule<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>>;
    fn delete_rules<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), AlertError>>;
    fn rename_rule<'a>(
        &'a self,
        actor: ActorId,
        input: &'a RenameAlertRuleInput,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>>;
    fn update_rule_description<'a>(
        &'a self,
        id: Uuid,
        description: Option<Option<&'a str>>,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>>;
    fn list_events<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        filter: &'a AlertEventFilter,
    ) -> BoxFuture<'a, Result<AlertEventPage, AlertError>>;
    fn get_event(&self, id: Uuid) -> BoxFuture<'_, Result<AlertEvent, AlertError>>;
    fn unresolved_count(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<i64, AlertError>>;
    fn acknowledge<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), AlertError>>;
    fn resolve<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
        note: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), AlertError>>;
    fn raise<'a>(
        &'a self,
        event: &'a NewAlertEvent,
    ) -> BoxFuture<'a, Result<Option<AlertEvent>, AlertError>>;
    fn process_event<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEvent>, AlertError>>;
    fn claim_delivery(
        &self,
        owner: Uuid,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<AlertDeliveryClaim>, AlertError>>;
    fn complete_delivery(&self, id: Uuid, owner: Uuid) -> BoxFuture<'_, Result<bool, AlertError>>;
    fn retry_delivery<'a>(
        &'a self,
        id: Uuid,
        owner: Uuid,
        next_attempt_at: DateTime<Utc>,
        dead_letter: bool,
        error: &'a str,
    ) -> BoxFuture<'a, Result<bool, AlertError>>;
    fn maintain_deliveries(
        &self,
        stale_before: DateTime<Utc>,
        dead_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), AlertError>>;
}

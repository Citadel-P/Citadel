use super::*;

#[derive(Clone)]
pub struct PostgresAlertRepository {
    pub(super) pool: PgPool,
    pub(super) configuration: Arc<super::configuration::ConfigurationCache>,
    pub(super) on_change: Option<Arc<dyn Fn() + Send + Sync>>,
    pub(super) entitlements: Arc<dyn citadel_alerts::AlertEntitlements>,
}

impl PostgresAlertRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            entitlements: Arc::new(crate::persistence::postgres::licensing::store::PostgresLicenseEntitlementService::new(
                pool.clone(),
            )),
            pool,
            on_change: None,
            configuration: Default::default(),
        }
    }
}

impl PostgresAlertRepository {
    pub fn with_entitlements(
        mut self,
        entitlements: Arc<dyn citadel_alerts::AlertEntitlements>,
    ) -> Self {
        self.entitlements = entitlements;
        self
    }
}

impl PostgresAlertRepository {
    pub fn with_change_notifier(mut self, notifier: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.on_change = Some(notifier);
        self
    }
}

impl PostgresAlertRepository {
    pub(super) fn changed(&self) {
        if let Some(notifier) = &self.on_change {
            notifier();
        }
    }
}

impl AlertRepository for PostgresAlertRepository {
    fn list_channels(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AlertChannel>, AlertError>> {
        self.list_channels_impl(actor, administrator)
    }

    fn get_channel(&self, id: Uuid) -> BoxFuture<'_, Result<AlertChannel, AlertError>> {
        self.get_channel_impl(id)
    }

    fn create_channel<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AlertChannelConfiguration,
    ) -> BoxFuture<'a, Result<AlertChannel, AlertError>> {
        self.create_channel_impl(actor, input)
    }

    fn update_channel<'a>(
        &'a self,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<AlertChannel, AlertError>> {
        self.update_channel_impl(id, patch)
    }

    fn delete_channels<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), AlertError>> {
        self.delete_channels_impl(ids)
    }

    fn list_rules(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AlertRuleListItem>, AlertError>> {
        self.list_rules_impl(actor, administrator)
    }

    fn get_rule(&self, id: Uuid) -> BoxFuture<'_, Result<AlertRule, AlertError>> {
        self.get_rule_impl(id)
    }

    fn create_rule<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AlertRuleConfiguration,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>> {
        self.create_rule_impl(actor, input)
    }

    fn update_rule<'a>(
        &'a self,
        request_actor: ActorId,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>> {
        self.update_rule_impl(request_actor, id, patch)
    }

    fn delete_rules<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), AlertError>> {
        self.delete_rules_impl(ids)
    }

    fn rename_rule<'a>(
        &'a self,
        actor: ActorId,
        input: &'a citadel_alerts::RenameAlertRuleInput,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>> {
        self.rename_rule_impl(actor, input)
    }

    fn update_rule_description<'a>(
        &'a self,
        id: Uuid,
        description: Option<Option<&'a str>>,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>> {
        self.update_rule_description_impl(id, description)
    }

    fn list_events<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        filter: &'a AlertEventFilter,
    ) -> BoxFuture<'a, Result<AlertEventPage, AlertError>> {
        self.list_events_impl(actor, administrator, filter)
    }

    fn get_event(&self, id: Uuid) -> BoxFuture<'_, Result<AlertEvent, AlertError>> {
        self.get_event_impl(id)
    }

    fn unresolved_count(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<i64, AlertError>> {
        self.unresolved_count_impl(actor, administrator)
    }

    fn acknowledge<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        self.acknowledge_impl(actor, ids)
    }

    fn resolve<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
        note: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        self.resolve_impl(actor, ids, note)
    }

    fn raise<'a>(
        &'a self,
        event: &'a NewAlertEvent,
    ) -> BoxFuture<'a, Result<Option<AlertEvent>, AlertError>> {
        self.raise_impl(event)
    }

    fn process_event<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEvent>, AlertError>> {
        self.process_event_impl(observation)
    }

    fn claim_delivery(
        &self,
        owner: Uuid,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<AlertDeliveryClaim>, AlertError>> {
        self.claim_delivery_impl(owner, stale_before)
    }

    fn complete_delivery(&self, id: Uuid, owner: Uuid) -> BoxFuture<'_, Result<bool, AlertError>> {
        self.complete_delivery_impl(id, owner)
    }

    fn retry_delivery<'a>(
        &'a self,
        id: Uuid,
        owner: Uuid,
        next_attempt_at: DateTime<Utc>,
        dead_letter: bool,
        error: &'a str,
    ) -> BoxFuture<'a, Result<bool, AlertError>> {
        self.retry_delivery_impl(id, owner, next_attempt_at, dead_letter, error)
    }

    fn maintain_deliveries(
        &self,
        stale_before: DateTime<Utc>,
        dead_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), AlertError>> {
        self.maintain_deliveries_impl(stale_before, dead_before)
    }
}

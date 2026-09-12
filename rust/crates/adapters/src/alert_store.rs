use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use citadel_alerts::{
    AlertChannelInput, AlertChannelView, AlertDelivery, AlertDeliveryClaim, AlertError,
    AlertEventFilter, AlertEventPage, AlertEventView, AlertObservation, AlertRuleInput,
    AlertRuleListItem, AlertRuleView, AlertStore, NewAlertEvent, is_in_quiet_hours,
};
use citadel_domain::{ActorId, ResourceType};
use citadel_execution::{OutputLimitPolicy, ProcessLimits, ProcessRequest, run};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const READ_MASK: i32 = 1 | 2 | 4;
const AUTHORIZED_CTE: &str = r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid FROM actors actor
    WHERE actor.id=$1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors team_actor ON team_actor.id=team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid=$1
), global_access AS (
    SELECT EXISTS (
        SELECT 1 FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid=scope.actorid
        JOIN permissions permission ON permission.roleid=assignment.roleid
        WHERE permission.resourcetype=$2
          AND (permission.permissionlevel & $3) <> 0
    ) AS allowed
)
"#;

#[derive(Clone)]
pub struct PostgresAlertStore {
    pool: PgPool,
    on_change: Option<Arc<dyn Fn() + Send + Sync>>,
    entitlements: Arc<dyn citadel_alerts::AlertEntitlements>,
}

impl PostgresAlertStore {
    pub fn new(pool: PgPool) -> Self {
        Self {
            entitlements: Arc::new(crate::license::PostgresLicenseEntitlementService::new(
                pool.clone(),
            )),
            pool,
            on_change: None,
        }
    }

    pub fn with_entitlements(
        mut self,
        entitlements: Arc<dyn citadel_alerts::AlertEntitlements>,
    ) -> Self {
        self.entitlements = entitlements;
        self
    }

    pub fn with_change_notifier(mut self, notifier: Arc<dyn Fn() + Send + Sync>) -> Self {
        self.on_change = Some(notifier);
        self
    }

    fn changed(&self) {
        if let Some(notifier) = &self.on_change {
            notifier();
        }
    }

    async fn process_rule(
        &self,
        rule: &AlertRuleView,
        observation: &AlertObservation,
    ) -> Result<Option<AlertEventView>, AlertError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        let lock_key = format!("{}:{}", rule.id, observation.resource_id);
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(lock_key)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
        let state = sqlx::query(
            "SELECT consecutivematches,lasttriggeredat FROM alertrulestates WHERE alertruleid=$1 AND resourceid=$2",
        )
        .bind(rule.id)
        .bind(observation.resource_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(storage)?;
        let consecutive = state
            .as_ref()
            .map(|row| row.try_get::<i32, _>("consecutivematches"))
            .transpose()
            .map_err(storage)?
            .unwrap_or_default();
        let last_triggered = state
            .as_ref()
            .map(|row| row.try_get::<Option<DateTime<Utc>>, _>("lasttriggeredat"))
            .transpose()
            .map_err(storage)?
            .flatten();
        let deduplication_key = format!(
            "{}:{}:{}",
            rule.id, observation.resource_id, observation.deduplication_component
        );
        if !observation_matches(rule, observation) {
            sqlx::query("INSERT INTO alertrulestates(alertruleid,resourceid,consecutivematches,createdbyactorid,lasttriggeredat) VALUES($1,$2,0,$3,$4) ON CONFLICT(alertruleid,resourceid) DO UPDATE SET consecutivematches=0")
                .bind(rule.id)
                .bind(observation.resource_id)
                .bind(rule.created_by_actor_id)
                .bind(last_triggered)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            let resolved = sqlx::query("UPDATE alertevents SET resolvedat=$2,resolutionnote='Condition returned to normal.',updatedat=$2,openincidentkey=NULL WHERE openincidentkey=$1 AND resolvedat IS NULL")
                .bind(&deduplication_key)
                .bind(observation.observed_at)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            transaction.commit().await.map_err(storage)?;
            if resolved.rows_affected() > 0 {
                self.changed();
            }
            return Ok(None);
        }
        let next_consecutive = consecutive.saturating_add(1);
        sqlx::query("INSERT INTO alertrulestates(alertruleid,resourceid,consecutivematches,createdbyactorid,lasttriggeredat) VALUES($1,$2,$3,$4,$5) ON CONFLICT(alertruleid,resourceid) DO UPDATE SET consecutivematches=EXCLUDED.consecutivematches")
            .bind(rule.id)
            .bind(observation.resource_id)
            .bind(next_consecutive)
            .bind(rule.created_by_actor_id)
            .bind(last_triggered)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
        if next_consecutive < rule.required_matches.unwrap_or(1).max(1) {
            transaction.commit().await.map_err(storage)?;
            return Ok(None);
        }
        if last_triggered.is_some_and(|last| {
            rule.cooldown_seconds.is_some_and(|seconds| {
                observation.observed_at < last + chrono::Duration::seconds(i64::from(seconds))
            })
        }) {
            transaction.commit().await.map_err(storage)?;
            return Ok(None);
        }
        let id = Uuid::now_v7();
        let inserted = sqlx::query("INSERT INTO alertevents(id,alertruleid,type,severity,info,resourceid,resourcename,resourcetype,deduplicationkey,openincidentkey) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$9) ON CONFLICT (openincidentkey) WHERE openincidentkey IS NOT NULL DO NOTHING RETURNING id")
            .bind(id)
            .bind(rule.id)
            .bind(&observation.alert_type)
            .bind(&rule.severity)
            .bind(&observation.info)
            .bind(observation.resource_id)
            .bind(&observation.resource_name)
            .bind(&observation.resource_type)
            .bind(&deduplication_key)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage)?;
        if inserted.is_none() {
            transaction.commit().await.map_err(storage)?;
            return Ok(None);
        }
        sqlx::query(
            "UPDATE alertrulestates SET lasttriggeredat=$3,consecutivematches=0 WHERE alertruleid=$1 AND resourceid=$2",
        )
        .bind(rule.id)
        .bind(observation.resource_id)
        .bind(observation.observed_at)
        .execute(&mut *transaction)
        .await
        .map_err(storage)?;
        enqueue_deliveries(&mut transaction, id, rule.id).await?;
        transaction.commit().await.map_err(storage)?;
        self.changed();
        self.get_event(id).await.map(Some)
    }
}

impl AlertStore for PostgresAlertStore {
    fn list_channels(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AlertChannelView>, AlertError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT channel.* FROM alertchannels channel
WHERE $4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=$2 AND access.resourceid=channel.id
      AND (access.permissionlevel & $3) <> 0
)
ORDER BY channel.name,channel.id"#
            );
            let rows = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::AlertChannel as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            rows.into_iter().map(map_channel).collect()
        })
    }
    fn get_channel(&self, id: Uuid) -> BoxFuture<'_, Result<AlertChannelView, AlertError>> {
        Box::pin(async move {
            let row = sqlx::query("SELECT * FROM alertchannels WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(AlertError::NotFound)?;
            map_channel(row)
        })
    }
    fn create_channel<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AlertChannelInput,
    ) -> BoxFuture<'a, Result<AlertChannelView, AlertError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            sqlx::query("INSERT INTO alertchannels(id,name,alertdestination,url,isactive,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6)")
                .bind(id).bind(&input.name).bind(&input.alert_destination).bind(&input.url).bind(input.is_active).bind(actor.value())
                .execute(&self.pool).await.map_err(storage)?;
            self.get_channel(id).await
        })
    }
    fn update_channel<'a>(
        &'a self,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<AlertChannelView, AlertError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query("SELECT * FROM alertchannels WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(AlertError::NotFound)?;
            let input =
                citadel_alerts::configuration_patch::apply_channel(&map_channel(row)?, patch)?;
            let changed = sqlx::query("UPDATE alertchannels SET name=$2,alertdestination=$3,url=$4,isactive=$5 WHERE id=$1")
                .bind(id).bind(&input.name).bind(&input.alert_destination).bind(&input.url).bind(input.is_active)
                .execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if changed == 0 {
                return Err(AlertError::NotFound);
            }
            tx.commit().await.map_err(storage)?;
            self.get_channel(id).await
        })
    }
    fn delete_channels<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async move {
            let ids = normalized_ids(ids, "Alert Channel")?;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            require_complete_set(&mut tx, "alertchannels", &ids).await?;
            sqlx::query("DELETE FROM alertchannels WHERE id = ANY($1)")
                .bind(&ids)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
    fn list_rules(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AlertRuleListItem>, AlertError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT r.*,COALESCE(array_agg(c.alertchannelid) FILTER (WHERE c.alertchannelid IS NOT NULL),'{{}}') AS channelids
FROM alertrules r
LEFT JOIN alertrulechannels c ON c.alertruleid=r.id
WHERE $4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=$2 AND access.resourceid=r.id
      AND (access.permissionlevel & $3) <> 0
)
GROUP BY r.id"#
            );
            let rows = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Alert as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            let rules = rows
                .into_iter()
                .map(map_rule)
                .collect::<Result<Vec<_>, _>>()?;
            // Rule access does not imply access to its linked channels.
            let channels_by_id: HashMap<_, _> = self
                .list_channels(actor, administrator)
                .await?
                .into_iter()
                .map(|channel| (channel.id, channel))
                .collect();
            Ok(rules
                .into_iter()
                .map(|rule| AlertRuleListItem {
                    channels: rule
                        .channel_ids
                        .iter()
                        .filter_map(|id| channels_by_id.get(id).cloned())
                        .collect(),
                    rule,
                })
                .collect())
        })
    }
    fn get_rule(&self, id: Uuid) -> BoxFuture<'_, Result<AlertRuleView, AlertError>> {
        Box::pin(async move {
            let row = sqlx::query("SELECT r.*,COALESCE(array_agg(c.alertchannelid) FILTER (WHERE c.alertchannelid IS NOT NULL),'{}') AS channelids FROM alertrules r LEFT JOIN alertrulechannels c ON c.alertruleid=r.id WHERE r.id=$1 GROUP BY r.id")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(AlertError::RuleNotFound)?;
            map_rule(row)
        })
    }
    fn create_rule<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AlertRuleInput,
    ) -> BoxFuture<'a, Result<AlertRuleView, AlertError>> {
        Box::pin(async move {
            if !self.entitlements.advanced_alerting().await? {
                return Err(AlertError::LicenseRequired);
            }
            let id = Uuid::now_v7();
            let mut tx = self.pool.begin().await.map_err(storage)?;
            write_rule(&mut tx, id, actor.value(), input, true).await?;
            write_rule_activity(
                &mut tx,
                id,
                &input.name,
                actor,
                citadel_domain::ActivityEventInfo::AlertRuleCreated {
                    alert_rule: input.snapshot(id),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            self.get_rule(id).await
        })
    }
    fn update_rule<'a>(
        &'a self,
        request_actor: ActorId,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<AlertRuleView, AlertError>> {
        Box::pin(async move {
            // Evaluate before taking a connection/row lock; the entitlement
            // provider may query the same bounded pool.
            let advanced_allowed = self.entitlements.advanced_alerting().await?;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let actor: Option<Uuid> = sqlx::query_scalar(
                "SELECT createdbyactorid FROM alertrules WHERE id=$1 FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?;
            let actor = actor.ok_or(AlertError::RuleNotFound)?;
            // Read child-channel state only after acquiring the parent lock.
            // Merging before the lock would lose disjoint concurrent patches.
            let row = sqlx::query("SELECT r.*,COALESCE(array_agg(c.alertchannelid) FILTER (WHERE c.alertchannelid IS NOT NULL),'{}') AS channelids FROM alertrules r LEFT JOIN alertrulechannels c ON c.alertruleid=r.id WHERE r.id=$1 GROUP BY r.id")
                .bind(id).fetch_one(&mut *tx).await.map_err(storage)?;
            let current = map_rule(row)?;
            let input = citadel_alerts::configuration_patch::apply_rule(&current, patch)?;
            if !advanced_allowed
                && citadel_alerts::configuration_patch::requires_advanced_alerting(&current, &input)
            {
                return Err(AlertError::LicenseRequired);
            }
            write_rule(&mut tx, id, actor, &input, false).await?;
            write_rule_activity(
                &mut tx,
                id,
                &input.name,
                request_actor,
                citadel_domain::ActivityEventInfo::AlertRuleUpdated {
                    old_rule: current.snapshot(),
                    new_rule: input.snapshot(id),
                },
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            self.get_rule(id).await
        })
    }
    fn delete_rules<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async move {
            let ids = normalized_ids(ids, "Alert Rule")?;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            require_complete_set(&mut tx, "alertrules", &ids).await?;
            let system: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM alertrules WHERE id=ANY($1) AND createdbyactorid='00000000-0000-0000-0000-000000000001')").bind(&ids).fetch_one(&mut *tx).await.map_err(storage)?;
            if system {
                return Err(AlertError::Conflict(
                    "Built-in Alert Rules cannot be deleted.".into(),
                ));
            }
            sqlx::query("DELETE FROM alertrules WHERE id=ANY($1)")
                .bind(&ids)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
    fn rename_rule<'a>(
        &'a self,
        actor: ActorId,
        input: &'a citadel_alerts::RenameAlertRuleInput,
    ) -> BoxFuture<'a, Result<AlertRuleView, AlertError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let old_name: String =
                sqlx::query_scalar("SELECT name FROM alertrules WHERE id=$1 FOR UPDATE")
                    .bind(input.id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(storage)?
                    .ok_or(AlertError::RuleNotFound)?;
            sqlx::query("UPDATE alertrules SET name=$2 WHERE id=$1")
                .bind(input.id)
                .bind(&input.name)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            let activity = citadel_domain::ActivityEvent::new_alert_rule_event(
                input.id,
                input.name.clone(),
                actor,
                citadel_domain::ActivityEventInfo::AlertRuleRenamed {
                    old_name,
                    new_name: input.name.clone(),
                },
                Utc::now(),
            )
            .map_err(|error| AlertError::Storage(error.to_string()))?;
            crate::activity_store::insert_activity(&mut tx, &activity)
                .await
                .map_err(|error| AlertError::Storage(error.to_string()))?;
            tx.commit().await.map_err(storage)?;
            self.get_rule(input.id).await
        })
    }
    fn update_rule_description<'a>(
        &'a self,
        id: Uuid,
        description: Option<Option<&'a str>>,
    ) -> BoxFuture<'a, Result<AlertRuleView, AlertError>> {
        Box::pin(async move {
            if let Some(description) = description {
                let count = sqlx::query("UPDATE alertrules SET description=$2 WHERE id=$1")
                    .bind(id)
                    .bind(description)
                    .execute(&self.pool)
                    .await
                    .map_err(storage)?
                    .rows_affected();
                if count == 0 {
                    return Err(AlertError::RuleNotFound);
                }
            }
            self.get_rule(id).await
        })
    }
    fn list_events<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        filter: &'a AlertEventFilter,
    ) -> BoxFuture<'a, Result<AlertEventPage, AlertError>> {
        Box::pin(async move {
            let page = filter.page.max(1);
            let page_size = filter.page_size.clamp(1, 1000);
            let offset = i64::from(page - 1) * i64::from(page_size);
            let predicate = r#"
($4::uuid IS NULL OR event.resourceid=$4)
AND ($5::text IS NULL OR event.type=$5)
AND ($6::text IS NULL OR event.resourcetype=$6)
AND (NOT $7 OR event.resolvedat IS NULL)
AND ($8 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=$2 AND access.resourceid=event.id
      AND (access.permissionlevel & $3) <> 0
))"#;
            let count_query = format!(
                "{AUTHORIZED_CTE} SELECT COUNT(*) FROM alertevents event WHERE {predicate}"
            );
            let total_count = sqlx::query_scalar(AssertSqlSafe(count_query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Alert as i32)
                .bind(READ_MASK)
                .bind(filter.resource_id)
                .bind(&filter.alert_type)
                .bind(&filter.resource_type)
                .bind(filter.unresolved_only)
                .bind(administrator)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)?;
            let select_query = format!(
                "{AUTHORIZED_CTE} SELECT event.* FROM alertevents event WHERE {predicate} ORDER BY event.createdat DESC,event.id DESC LIMIT $9 OFFSET $10"
            );
            let rows = sqlx::query(AssertSqlSafe(select_query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Alert as i32)
                .bind(READ_MASK)
                .bind(filter.resource_id)
                .bind(&filter.alert_type)
                .bind(&filter.resource_type)
                .bind(filter.unresolved_only)
                .bind(administrator)
                .bind(i64::from(page_size))
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            Ok(AlertEventPage {
                items: rows.into_iter().map(map_event).collect::<Result<_, _>>()?,
                total_count,
                page,
                page_size,
            })
        })
    }
    fn get_event(&self, id: Uuid) -> BoxFuture<'_, Result<AlertEventView, AlertError>> {
        Box::pin(async move {
            let row = sqlx::query("SELECT * FROM alertevents WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(AlertError::NotFound)?;
            map_event(row)
        })
    }
    fn unresolved_count(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<i64, AlertError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT COUNT(*) FROM alertevents event
WHERE event.resolvedat IS NULL
  AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=event.id
        AND (access.permissionlevel & $3) <> 0
  ))"#
            );
            sqlx::query_scalar(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Alert as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)
        })
    }
    fn acknowledge<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async move {
            let ids = normalized_ids(ids, "Alert Event")?;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            require_complete_set(&mut tx, "alertevents", &ids).await?;
            let changed = sqlx::query("UPDATE alertevents SET acknowledgedbyactorid=$1,acknowledgedat=COALESCE(acknowledgedat,CURRENT_TIMESTAMP),updatedat=CURRENT_TIMESTAMP WHERE id=ANY($2) AND resolvedat IS NULL")
                .bind(actor.value()).bind(&ids).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if usize::try_from(changed).ok() != Some(ids.len()) {
                return Err(AlertError::Conflict(
                    "One or more Alert Events are already resolved.".into(),
                ));
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
    fn resolve<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
        note: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async move {
            let ids = normalized_ids(ids, "Alert Event")?;
            if note.is_some_and(|value| value.chars().count() > 1000) {
                return Err(AlertError::Validation(
                    "Alert resolution note cannot exceed 1000 characters.".into(),
                ));
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            require_complete_set(&mut tx, "alertevents", &ids).await?;
            let changed = sqlx::query("UPDATE alertevents SET acknowledgedbyactorid=COALESCE(acknowledgedbyactorid,$1),acknowledgedat=COALESCE(acknowledgedat,CURRENT_TIMESTAMP),resolvedbyactorid=$1,resolvedat=CURRENT_TIMESTAMP,resolutionnote=$3,openincidentkey=NULL,updatedat=CURRENT_TIMESTAMP WHERE id=ANY($2) AND resolvedat IS NULL")
                .bind(actor.value()).bind(&ids).bind(note).execute(&mut *tx).await.map_err(storage)?.rows_affected();
            if usize::try_from(changed).ok() != Some(ids.len()) {
                return Err(AlertError::Conflict(
                    "One or more Alert Events are already resolved.".into(),
                ));
            }
            tx.commit().await.map_err(storage)?;
            Ok(())
        })
    }
    fn raise<'a>(
        &'a self,
        event: &'a NewAlertEvent,
    ) -> BoxFuture<'a, Result<Option<AlertEventView>, AlertError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let inserted = sqlx::query("INSERT INTO alertevents(id,alertruleid,type,severity,info,resourceid,resourcename,resourcetype,deduplicationkey,openincidentkey) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$9) ON CONFLICT (openincidentkey) WHERE openincidentkey IS NOT NULL DO NOTHING RETURNING id")
                .bind(id).bind(event.alert_rule_id).bind(&event.alert_type).bind(&event.severity).bind(&event.info).bind(event.resource_id).bind(&event.resource_name).bind(&event.resource_type).bind(&event.deduplication_key)
                .fetch_optional(&mut *transaction).await.map_err(storage)?;
            match inserted {
                Some(_) => {
                    enqueue_deliveries(&mut transaction, id, event.alert_rule_id).await?;
                    transaction.commit().await.map_err(storage)?;
                    self.changed();
                    self.get_event(id).await.map(Some)
                }
                None => {
                    transaction.rollback().await.map_err(storage)?;
                    Ok(None)
                }
            }
        })
    }

    fn process_event<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEventView>, AlertError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"SELECT rule.*,
COALESCE(array_agg(relation.alertchannelid) FILTER (WHERE relation.alertchannelid IS NOT NULL),'{}') AS channelids
FROM alertrules rule
LEFT JOIN alertrulechannels relation ON relation.alertruleid=rule.id
WHERE rule.status='Enabled' AND rule.type=$1
GROUP BY rule.id
ORDER BY CASE rule.severity WHEN 'Critical' THEN 3 WHEN 'Warning' THEN 2 ELSE 1 END DESC,
         rule.createdat,rule.id"#,
            )
            .bind(&observation.alert_type)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
            let rules = rows
                .into_iter()
                .map(map_rule)
                .collect::<Result<Vec<_>, _>>()?;
            let advanced_alerting = !rules
                .iter()
                .any(|rule| rule.created_by_actor_id != Uuid::from_u128(1))
                || self.entitlements.advanced_alerting().await?;
            let rules = rules
                .into_iter()
                .filter(|rule| {
                    (advanced_alerting || rule.created_by_actor_id == Uuid::from_u128(1))
                        && rule_applies(rule, observation.resource_id)
                        && !is_in_quiet_hours(&rule.quiet_hours, observation.observed_at)
                })
                .collect::<Vec<_>>();
            // Rules are ordered by severity above. Select before cooldown so a
            // suppressed winner cannot fall through to a less severe duplicate.
            // Non-matches still reset their counters and resolve old incidents.
            let mut winner = None;
            for rule in &rules {
                if observation_matches(rule, observation) {
                    winner.get_or_insert(rule);
                } else {
                    self.process_rule(rule, observation).await?;
                }
            }
            match winner {
                Some(rule) => self.process_rule(rule, observation).await,
                None => Ok(None),
            }
        })
    }

    fn claim_delivery(
        &self,
        owner: Uuid,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<AlertDeliveryClaim>, AlertError>> {
        Box::pin(async move {
            let claimed = sqlx::query(
                r#"WITH candidate AS (
    SELECT queue.id
    FROM alertdeliveryoutbox queue
    JOIN alertchannels channel ON channel.id=queue.alertchannelid AND channel.isactive
    WHERE (queue.status='Pending' AND queue.nextattemptat<=CURRENT_TIMESTAMP)
       OR (queue.status='Delivering' AND queue.claimedat<$2)
    ORDER BY queue.nextattemptat,queue.id
    FOR UPDATE OF queue SKIP LOCKED
    LIMIT 1
)
UPDATE alertdeliveryoutbox queue
SET status='Delivering',claimowner=$1,claimedat=CURRENT_TIMESTAMP,updatedat=CURRENT_TIMESTAMP
FROM candidate
WHERE queue.id=candidate.id
RETURNING queue.id,queue.alerteventid,queue.alertchannelid,queue.attemptcount"#,
            )
            .bind(owner)
            .bind(stale_before)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
            let Some(claimed) = claimed else {
                return Ok(None);
            };
            let id: Uuid = claimed.try_get("id").map_err(storage)?;
            let event_id: Uuid = claimed.try_get("alerteventid").map_err(storage)?;
            let channel_id: Uuid = claimed.try_get("alertchannelid").map_err(storage)?;
            let attempt_count: i32 = claimed.try_get("attemptcount").map_err(storage)?;
            let event = sqlx::query("SELECT * FROM alertevents WHERE id=$1")
                .bind(event_id)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)
                .and_then(map_event)?;
            let channel = sqlx::query("SELECT * FROM alertchannels WHERE id=$1 AND isactive")
                .bind(channel_id)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)
                .and_then(map_channel)?;
            Ok(Some(AlertDeliveryClaim {
                id,
                attempt_count,
                channel,
                event,
            }))
        })
    }

    fn complete_delivery(&self, id: Uuid, owner: Uuid) -> BoxFuture<'_, Result<bool, AlertError>> {
        Box::pin(async move {
            let changed = sqlx::query("DELETE FROM alertdeliveryoutbox WHERE id=$1 AND status='Delivering' AND claimowner=$2")
                .bind(id)
                .bind(owner)
                .execute(&self.pool)
                .await
                .map_err(storage)?
                .rows_affected();
            Ok(changed == 1)
        })
    }

    fn retry_delivery<'a>(
        &'a self,
        id: Uuid,
        owner: Uuid,
        next_attempt_at: DateTime<Utc>,
        dead_letter: bool,
        error: &'a str,
    ) -> BoxFuture<'a, Result<bool, AlertError>> {
        Box::pin(async move {
            let error = error.chars().take(2_048).collect::<String>();
            let status = if dead_letter { "DeadLetter" } else { "Pending" };
            let changed = sqlx::query("UPDATE alertdeliveryoutbox SET status=$3,attemptcount=attemptcount+1,nextattemptat=$4,lasterror=$5,claimowner=NULL,claimedat=NULL,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Delivering' AND claimowner=$2")
                .bind(id)
                .bind(owner)
                .bind(status)
                .bind(next_attempt_at)
                .bind(error)
                .execute(&self.pool)
                .await
                .map_err(storage)?
                .rows_affected();
            Ok(changed == 1)
        })
    }

    fn maintain_deliveries(
        &self,
        stale_before: DateTime<Utc>,
        dead_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), AlertError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            sqlx::query("DELETE FROM alertdeliveryoutbox queue USING alertchannels channel WHERE queue.alertchannelid=channel.id AND NOT channel.isactive")
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            sqlx::query(
                "DELETE FROM alertdeliveryoutbox WHERE status='DeadLetter' AND updatedat<$1",
            )
            .bind(dead_before)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            sqlx::query("UPDATE alertdeliveryoutbox SET status='Pending',claimowner=NULL,claimedat=NULL,updatedat=CURRENT_TIMESTAMP WHERE status='Delivering' AND claimedat<$1")
                .bind(stale_before)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            transaction.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}

async fn write_rule(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    actor: Uuid,
    input: &AlertRuleInput,
    insert: bool,
) -> Result<(), AlertError> {
    if !input.channel_ids.is_empty() {
        let channels: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM alertchannels WHERE id=ANY($1) FOR SHARE")
                .bind(&input.channel_ids)
                .fetch_all(&mut **tx)
                .await
                .map_err(storage)?;
        if channels.len() != input.channel_ids.len() {
            return Err(AlertError::Validation(
                "One or more Alert Channels do not exist.".into(),
            ));
        }
    }
    let limited = Value::Array(input.limited_to.clone());
    let quiet = Value::Array(input.quiet_hours.clone());
    if insert {
        sqlx::query("INSERT INTO alertrules(id,name,description,type,severity,cooldownseconds,requiredmatches,threshold,status,limitedto,quiethours,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
            .bind(id).bind(&input.name).bind(&input.description).bind(&input.alert_type).bind(&input.severity).bind(input.cooldown_seconds).bind(input.required_matches).bind(input.threshold).bind(&input.status).bind(limited).bind(quiet).bind(actor).execute(&mut **tx).await.map_err(storage)?;
    } else {
        sqlx::query("UPDATE alertrules SET name=$2,description=$3,type=$4,severity=$5,cooldownseconds=$6,requiredmatches=$7,threshold=$8,status=$9,limitedto=$10,quiethours=$11 WHERE id=$1")
            .bind(id).bind(&input.name).bind(&input.description).bind(&input.alert_type).bind(&input.severity).bind(input.cooldown_seconds).bind(input.required_matches).bind(input.threshold).bind(&input.status).bind(limited).bind(quiet).execute(&mut **tx).await.map_err(storage)?;
        sqlx::query("DELETE FROM alertrulechannels WHERE alertruleid=$1")
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(storage)?;
    }
    for channel in &input.channel_ids {
        sqlx::query("INSERT INTO alertrulechannels(alertruleid,alertchannelid) VALUES($1,$2)")
            .bind(id)
            .bind(channel)
            .execute(&mut **tx)
            .await
            .map_err(storage)?;
    }
    Ok(())
}

async fn write_rule_activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: &str,
    actor: ActorId,
    info: citadel_domain::ActivityEventInfo,
) -> Result<(), AlertError> {
    let activity = citadel_domain::ActivityEvent::new_alert_rule_event(
        id,
        name.to_owned(),
        actor,
        info,
        Utc::now(),
    )
    .map_err(|error| AlertError::Storage(error.to_string()))?;
    crate::activity_store::insert_activity(tx, &activity)
        .await
        .map_err(|error| AlertError::Storage(error.to_string()))
}

fn rule_applies(rule: &AlertRuleView, resource_id: Uuid) -> bool {
    rule.limited_to.is_empty()
        || rule.limited_to.iter().any(|scope| {
            ["resourceId", "ResourceId"]
                .iter()
                .find_map(|key| scope.get(*key).and_then(Value::as_str))
                .and_then(|value| Uuid::parse_str(value).ok())
                == Some(resource_id)
        })
}

fn observation_matches(rule: &AlertRuleView, observation: &AlertObservation) -> bool {
    match (rule.threshold, observation.value) {
        (Some(threshold), Some(value)) => value.is_finite() && value >= threshold,
        (Some(_), None) => false,
        (None, _) => observation.matched,
    }
}

async fn enqueue_deliveries(
    transaction: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    rule_id: Uuid,
) -> Result<(), AlertError> {
    sqlx::query(
        r#"INSERT INTO alertdeliveryoutbox(id,alerteventid,alertchannelid)
SELECT gen_random_uuid(),$1,relation.alertchannelid
FROM alertrulechannels relation
JOIN alertchannels channel ON channel.id=relation.alertchannelid AND channel.isactive
WHERE relation.alertruleid=$2
ON CONFLICT(alerteventid,alertchannelid) DO NOTHING"#,
    )
    .bind(event_id)
    .bind(rule_id)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

fn map_channel(row: sqlx::postgres::PgRow) -> Result<AlertChannelView, AlertError> {
    Ok(AlertChannelView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        alert_destination: row.try_get("alertdestination").map_err(storage)?,
        url: row.try_get("url").map_err(storage)?,
        is_active: row.try_get("isactive").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}
fn map_rule(row: sqlx::postgres::PgRow) -> Result<AlertRuleView, AlertError> {
    let limited: Value = row.try_get("limitedto").map_err(storage)?;
    let quiet: Value = row.try_get("quiethours").map_err(storage)?;
    Ok(AlertRuleView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        alert_type: row.try_get("type").map_err(storage)?,
        severity: row.try_get("severity").map_err(storage)?,
        cooldown_seconds: row.try_get("cooldownseconds").map_err(storage)?,
        required_matches: row.try_get("requiredmatches").map_err(storage)?,
        threshold: row.try_get("threshold").map_err(storage)?,
        status: row.try_get("status").map_err(storage)?,
        channel_ids: row.try_get("channelids").map_err(storage)?,
        limited_to: limited.as_array().cloned().unwrap_or_default(),
        quiet_hours: quiet.as_array().cloned().unwrap_or_default(),
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}
fn map_event(row: sqlx::postgres::PgRow) -> Result<AlertEventView, AlertError> {
    let acknowledged_at: Option<DateTime<Utc>> = row.try_get("acknowledgedat").map_err(storage)?;
    let resolved_at: Option<DateTime<Utc>> = row.try_get("resolvedat").map_err(storage)?;
    let info: Value = row.try_get("info").map_err(storage)?;
    let message = info
        .get("HumanMessage")
        .or_else(|| info.get("humanMessage"))
        .and_then(Value::as_str)
        .unwrap_or("Alert condition matched.")
        .to_owned();
    Ok(AlertEventView {
        id: row.try_get("id").map_err(storage)?,
        alert_rule_id: row.try_get("alertruleid").map_err(storage)?,
        alert_type: row.try_get("type").map_err(storage)?,
        severity: row.try_get("severity").map_err(storage)?,
        status: if resolved_at.is_some() {
            "Resolved"
        } else if acknowledged_at.is_some() {
            "Acknowledged"
        } else {
            "Active"
        }
        .into(),
        message,
        info,
        resource_id: row.try_get("resourceid").map_err(storage)?,
        resource_name: row.try_get("resourcename").map_err(storage)?,
        resource_type: row.try_get("resourcetype").map_err(storage)?,
        acknowledged_by_actor_id: row.try_get("acknowledgedbyactorid").map_err(storage)?,
        acknowledged_at,
        resolved_by_actor_id: row.try_get("resolvedbyactorid").map_err(storage)?,
        resolved_at,
        resolution_note: row.try_get("resolutionnote").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        updated_at: row.try_get("updatedat").map_err(storage)?,
    })
}
fn normalized_ids(ids: &[Uuid], label: &str) -> Result<Vec<Uuid>, AlertError> {
    if ids.is_empty() || ids.iter().any(Uuid::is_nil) {
        return Err(AlertError::Validation(format!(
            "{label} IDs must not be empty."
        )));
    }
    let mut normalized = ids.to_vec();
    normalized.sort_unstable();
    normalized.dedup();
    Ok(normalized)
}

async fn require_complete_set(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    ids: &[Uuid],
) -> Result<(), AlertError> {
    let sql = match table {
        "alertchannels" => "SELECT id FROM alertchannels WHERE id=ANY($1) FOR SHARE",
        "alertrules" => "SELECT id FROM alertrules WHERE id=ANY($1) FOR SHARE",
        "alertevents" => "SELECT id FROM alertevents WHERE id=ANY($1) FOR UPDATE",
        _ => unreachable!("closed Alert table inventory"),
    };
    let found: Vec<Uuid> = sqlx::query_scalar(sql)
        .bind(ids)
        .fetch_all(&mut **tx)
        .await
        .map_err(storage)?;
    if found.len() != ids.len() {
        return Err(AlertError::NotFound);
    }
    Ok(())
}

fn storage(error: sqlx::Error) -> AlertError {
    match &error {
        sqlx::Error::Database(database) if database.code().as_deref() == Some("23505") => {
            AlertError::Conflict("An Alert resource with the same value already exists.".into())
        }
        sqlx::Error::Database(database) if database.code().as_deref() == Some("23503") => {
            AlertError::Validation("An Alert resource reference does not exist.".into())
        }
        _ => AlertError::Storage(error.to_string()),
    }
}

#[derive(Clone)]
pub struct ShoutrrrAlertDelivery {
    executable: OsString,
    timeout: Duration,
}
impl ShoutrrrAlertDelivery {
    pub fn new(executable: impl Into<OsString>, timeout: Duration) -> Self {
        Self {
            executable: executable.into(),
            timeout,
        }
    }
}
impl AlertDelivery for ShoutrrrAlertDelivery {
    fn send<'a>(
        &'a self,
        channel: &'a AlertChannelView,
        event: &'a AlertEventView,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), AlertError>> {
        Box::pin(async move {
            let output = run(
                ProcessRequest::new(self.executable.clone())
                    .args([
                        "send",
                        "--url",
                        channel.url.as_str(),
                        "--title",
                        &format!("[{}] Citadel alert", event.severity),
                        "--message",
                        event.message.as_str(),
                    ])
                    .limits(ProcessLimits {
                        timeout: self.timeout,
                        maximum_stdout_bytes: 64 * 1024,
                        maximum_stderr_bytes: 64 * 1024,
                        output_limit_policy: OutputLimitPolicy::Truncate,
                    }),
                cancellation,
            )
            .await
            .map_err(|error| AlertError::Delivery(error.to_string()))?;
            if output.succeeded() {
                Ok(())
            } else {
                Err(AlertError::Delivery(format!(
                    "Shoutrrr exited with code {}.",
                    output
                        .exit_code
                        .map_or_else(|| "unknown".to_owned(), |code| code.to_string())
                )))
            }
        })
    }
}

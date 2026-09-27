use super::*;

impl PostgresAlertRepository {
    pub(super) fn list_events_impl<'a>(
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
      AND access.permissionlevel = ANY($3)
))"#;
            let count_query = format!(
                "{AUTHORIZED_CTE} SELECT COUNT(*) FROM alertevents event WHERE {predicate}"
            );
            let total_count = sqlx::query_scalar(AssertSqlSafe(count_query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Alert as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
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
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
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
}

impl PostgresAlertRepository {
    pub(super) fn get_event_impl(&self, id: Uuid) -> BoxFuture<'_, Result<AlertEvent, AlertError>> {
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
}

impl PostgresAlertRepository {
    pub(super) fn unresolved_count_impl(
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
        AND access.permissionlevel = ANY($3)
  ))"#
            );
            sqlx::query_scalar(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Alert as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn acknowledge_impl<'a>(
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
}

impl PostgresAlertRepository {
    pub(super) fn resolve_impl<'a>(
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
}

impl PostgresAlertRepository {
    pub(super) fn raise_impl<'a>(
        &'a self,
        event: &'a NewAlertEvent,
    ) -> BoxFuture<'a, Result<Option<AlertEvent>, AlertError>> {
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
}

impl PostgresAlertRepository {
    pub(super) fn process_event_impl<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEvent>, AlertError>> {
        self.process_observation(observation, None)
    }

    pub(super) fn process_observation<'a>(
        &'a self,
        observation: &'a AlertObservation,
        receipt: Option<Uuid>,
    ) -> BoxFuture<'a, Result<Option<AlertEvent>, AlertError>> {
        Box::pin(async move {
            let rules = self.configured_rules(&observation.alert_type).await?;
            let advanced_alerting = !rules
                .iter()
                .any(|rule| rule.created_by_actor_id != Uuid::from_u128(1))
                || self.entitlements.advanced_alerting().await?;
            let rules = rules
                .iter()
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
                    self.process_rule(rule, observation, receipt).await?;
                }
            }
            match winner {
                Some(rule) => self.process_rule(rule, observation, receipt).await,
                None => Ok(None),
            }
        })
    }
}

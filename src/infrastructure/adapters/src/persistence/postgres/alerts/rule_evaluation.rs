use super::*;

impl PostgresAlertRepository {
    pub(super) async fn process_rule_in(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        changed: &mut bool,
        rule: &AlertRule,
        observation: &AlertObservation,
        receipt: Option<Uuid>,
    ) -> Result<Option<AlertEvent>, AlertError> {
        let instance_update = super::instance_updates::is_instance_update(&observation.alert_type);
        if instance_update && !super::instance_updates::is_current(transaction, observation).await?
        {
            return Ok(None);
        }
        if super::update_resolution::already_applied(transaction, observation).await? {
            return Ok(None);
        }
        // A snapshot can outlive a concurrent deletion. Hold the parent while
        // applying mutable state so deletion cannot race the foreign key writes.
        let enabled: Option<String> =
            sqlx::query_scalar("SELECT status FROM alertrules WHERE id=$1 FOR SHARE")
                .bind(rule.id)
                .fetch_optional(&mut **transaction)
                .await
                .map_err(storage)?;
        let enabled = enabled
            .as_deref()
            .map(str::parse::<citadel_alerts::AlertRuleStatus>)
            .transpose()
            .map_err(AlertError::Storage)?;
        if enabled != Some(citadel_alerts::AlertRuleStatus::Enabled) {
            return Ok(None);
        }
        if let Some(id) = receipt {
            let inserted = sqlx::query("INSERT INTO alertobservationreceipts(observationid,ruleid) VALUES($1,$2) ON CONFLICT DO NOTHING")
                .bind(id).bind(rule.id).execute(&mut **transaction).await.map_err(storage)?;
            if inserted.rows_affected() == 0 {
                return Ok(None);
            }
        }
        let lock_key = format!("{}:{}", rule.id, observation.resource_id);
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(lock_key)
            .execute(&mut **transaction)
            .await
            .map_err(storage)?;
        let state = sqlx::query(
            "SELECT consecutivematches,lasttriggeredat,lastobservedat FROM alertrulestates WHERE alertruleid=$1 AND resourceid=$2",
        )
        .bind(rule.id)
        .bind(observation.resource_id)
        .fetch_optional(&mut **transaction)
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
        let update_available =
            citadel_alerts::update_resolution::is_update_available(&observation.alert_type)
                || instance_update;
        let last_observed = state
            .as_ref()
            .map(|row| row.try_get::<Option<DateTime<Utc>>, _>("lastobservedat"))
            .transpose()
            .map_err(storage)?
            .flatten();
        // Rechecks and suppressed observations also fence delayed targets. Keep
        // their timestamp independent of lasttriggeredat, which owns cooldown.
        if update_available && last_observed.is_some_and(|last| observation.observed_at < last) {
            return Ok(None);
        }
        let deduplication_key = format!(
            "{}:{}:{}",
            rule.id, observation.resource_id, observation.deduplication_component
        );
        if !observation_matches(rule, observation) {
            // The rule/resource advisory lock fences this state read. Healthy
            // repeats need no new tuple/WAL when their counter is already zero.
            // Incident resolution below still runs: triggering resets the counter
            // too, so zero does not mean there is no open incident.
            if state.is_none() || consecutive != 0 {
                sqlx::query("INSERT INTO alertrulestates(alertruleid,resourceid,consecutivematches,createdbyactorid,lasttriggeredat) VALUES($1,$2,0,$3,$4) ON CONFLICT(alertruleid,resourceid) DO UPDATE SET consecutivematches=0")
                    .bind(rule.id)
                    .bind(observation.resource_id)
                    .bind(rule.audit.created_by_actor_id.value())
                    .bind(last_triggered)
                    .execute(&mut **transaction)
                    .await
                    .map_err(storage)?;
            }
            let resolved = sqlx::query("UPDATE alertevents SET resolvedat=$2,resolvedbyactorid='00000000-0000-0000-0000-000000000001',resolutionnote='Condition returned to normal.',updatedat=$2,openincidentkey=NULL WHERE openincidentkey=$1 AND resolvedat IS NULL")
                .bind(&deduplication_key)
                .bind(observation.observed_at)
                .execute(&mut **transaction)
                .await
                .map_err(storage)?;
            if resolved.rows_affected() > 0 {
                *changed = true;
            }
            return Ok(None);
        }
        let next_consecutive = consecutive.saturating_add(1);
        sqlx::query("INSERT INTO alertrulestates(alertruleid,resourceid,consecutivematches,createdbyactorid,lasttriggeredat,lastobservedat) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(alertruleid,resourceid) DO UPDATE SET consecutivematches=EXCLUDED.consecutivematches,lastobservedat=EXCLUDED.lastobservedat")
            .bind(rule.id)
            .bind(observation.resource_id)
            .bind(next_consecutive)
            .bind(rule.audit.created_by_actor_id.value())
            .bind(last_triggered)
            .bind(update_available.then_some(observation.observed_at))
            .execute(&mut **transaction)
            .await
            .map_err(storage)?;
        if next_consecutive < rule.required_matches.unwrap_or(1).max(1) {
            return Ok(None);
        }
        if last_triggered.is_some_and(|last| {
            rule.cooldown_seconds.is_some_and(|seconds| {
                observation.observed_at < last + chrono::Duration::seconds(i64::from(seconds))
            })
        }) {
            return Ok(None);
        }
        let id = Uuid::now_v7();
        let inserted = sqlx::query("INSERT INTO alertevents(id,alertruleid,type,severity,info,resourceid,resourcename,resourcetype,deduplicationkey,openincidentkey) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$9) ON CONFLICT (openincidentkey) WHERE openincidentkey IS NOT NULL DO NOTHING RETURNING id")
            .bind(id)
            .bind(rule.id)
            .bind(&observation.alert_type)
            .bind(rule.severity.as_str())
            .bind(&observation.info)
            .bind(observation.resource_id)
            .bind(&observation.resource_name)
            .bind(&observation.resource_type)
            .bind(&deduplication_key)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(storage)?;
        let superseded = if update_available {
            super::update_resolution::supersede_pending(
                transaction,
                rule.id,
                observation,
                &deduplication_key,
            )
            .await?
        } else {
            false
        };
        if inserted.is_none() {
            let refreshed = if instance_update {
                sqlx::query("UPDATE alertevents SET info=$2,updatedat=$3 WHERE openincidentkey=$1 AND info::jsonb IS DISTINCT FROM $2::jsonb")
                    .bind(&deduplication_key).bind(&observation.info).bind(observation.observed_at)
                    .execute(&mut **transaction).await.map_err(storage)?.rows_affected() > 0
            } else {
                false
            };
            if superseded || refreshed {
                *changed = true;
            }
            return Ok(None);
        }
        sqlx::query(
            "UPDATE alertrulestates SET lasttriggeredat=$3,consecutivematches=0 WHERE alertruleid=$1 AND resourceid=$2",
        )
        .bind(rule.id)
        .bind(observation.resource_id)
        .bind(observation.observed_at)
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
        enqueue_deliveries(transaction, id, rule.id).await?;
        *changed = true;
        let query = format!("{} WHERE event.id=$1", super::queries::EVENT_SELECT);
        let row = sqlx::query(AssertSqlSafe(query))
            .bind(id)
            .fetch_one(&mut **transaction)
            .await
            .map_err(storage)?;
        map_event(row).map(Some)
    }
}

impl PostgresAlertRepository {
    pub(super) async fn process_rule(
        &self,
        rule: &AlertRule,
        observation: &AlertObservation,
        receipt: Option<Uuid>,
    ) -> Result<Option<AlertEvent>, AlertError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let mut changed = false;
        let event = self
            .process_rule_in(&mut tx, &mut changed, rule, observation, receipt)
            .await?;
        tx.commit().await.map_err(storage)?;
        if changed {
            self.changed();
        }
        Ok(event)
    }
}

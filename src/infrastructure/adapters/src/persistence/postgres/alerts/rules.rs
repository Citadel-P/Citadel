use super::*;

impl PostgresAlertRepository {
    pub(super) fn list_rules_impl(
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
      AND access.permissionlevel = ANY($3)
)
GROUP BY r.id"#
            );
            let rows = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Alert as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
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
}

impl PostgresAlertRepository {
    pub(super) fn get_rule_impl(&self, id: Uuid) -> BoxFuture<'_, Result<AlertRule, AlertError>> {
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
}

impl PostgresAlertRepository {
    pub(super) fn create_rule_impl<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AlertRuleConfiguration,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>> {
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
                citadel_activities::ActivityEventInfo::AlertRuleCreated {
                    alert_rule: input.snapshot(id),
                },
            )
            .await?;
            self.commit_configuration(tx).await?;
            self.get_rule(id).await
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn update_rule_impl<'a>(
        &'a self,
        request_actor: ActorId,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>> {
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
                citadel_activities::ActivityEventInfo::AlertRuleUpdated {
                    old_rule: current.snapshot(),
                    new_rule: input.snapshot(id),
                },
            )
            .await?;
            self.commit_configuration(tx).await?;
            self.get_rule(id).await
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn delete_rules_impl<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), AlertError>> {
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
            self.commit_configuration(tx).await?;
            Ok(())
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn rename_rule_impl<'a>(
        &'a self,
        actor: ActorId,
        input: &'a citadel_alerts::RenameAlertRuleInput,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>> {
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
            let activity = citadel_activities::ActivityEvent::new_alert_rule_event(
                input.id,
                input.name.clone(),
                actor,
                citadel_activities::ActivityEventInfo::AlertRuleRenamed {
                    old_name,
                    new_name: input.name.clone(),
                },
                Utc::now(),
            )
            .map_err(|error| AlertError::Storage(error.to_string()))?;
            crate::persistence::postgres::activities::store::insert_activity(&mut tx, &activity)
                .await
                .map_err(|error| AlertError::Storage(error.to_string()))?;
            self.commit_configuration(tx).await?;
            self.get_rule(input.id).await
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn update_rule_description_impl<'a>(
        &'a self,
        id: Uuid,
        description: Option<Option<&'a str>>,
    ) -> BoxFuture<'a, Result<AlertRule, AlertError>> {
        Box::pin(async move {
            if let Some(description) = description {
                let mut tx = self.pool.begin().await.map_err(storage)?;
                let count = sqlx::query("UPDATE alertrules SET description=$2 WHERE id=$1")
                    .bind(id)
                    .bind(description)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?
                    .rows_affected();
                if count == 0 {
                    return Err(AlertError::RuleNotFound);
                }
                self.commit_configuration(tx).await?;
            }
            self.get_rule(id).await
        })
    }
}

pub(super) async fn write_rule(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    actor: Uuid,
    input: &AlertRuleConfiguration,
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
            .bind(id).bind(&input.name).bind(&input.description).bind(&input.alert_type).bind(input.severity.as_str()).bind(input.cooldown_seconds).bind(input.required_matches).bind(input.threshold).bind(input.status.as_str()).bind(limited).bind(quiet).bind(actor).execute(&mut **tx).await.map_err(storage)?;
    } else {
        sqlx::query("UPDATE alertrules SET name=$2,description=$3,type=$4,severity=$5,cooldownseconds=$6,requiredmatches=$7,threshold=$8,status=$9,limitedto=$10,quiethours=$11 WHERE id=$1")
            .bind(id).bind(&input.name).bind(&input.description).bind(&input.alert_type).bind(input.severity.as_str()).bind(input.cooldown_seconds).bind(input.required_matches).bind(input.threshold).bind(input.status.as_str()).bind(limited).bind(quiet).execute(&mut **tx).await.map_err(storage)?;
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

use super::*;

impl PostgresBackupPersistence {
    pub(super) fn create_policy_impl<'a>(
        &'a self,
        actor: ActorId,
        input: &'a BackupPolicyConfiguration,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            sqlx::query("INSERT INTO backuppolicies(id,name,normalizedname,description,source,backuprepositoryid,enabled,cron,timezone,webhook,keeplastsuccessful,timeoutseconds,alertonfailure,runasactorid,createdbyactorid) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)").bind(id).bind(&input.name).bind(input.name.to_uppercase()).bind(&input.description).bind(&input.source).bind(input.backup_repository_id).bind(input.enabled).bind(&input.cron).bind(&input.time_zone).bind(&input.webhook).bind(input.keep_last_successful.unwrap_or(14)).bind(input.timeout_seconds.unwrap_or(14400)).bind(input.alert_on_failure).bind(input.run_as_actor_id.unwrap_or(actor.value())).bind(actor.value()).execute(&mut *transaction).await.map_err(storage)?;
            resource_tags::insert(
                &mut transaction,
                "BackupPolicy",
                id,
                &input.tag_ids,
                actor.value(),
            )
            .await
            .map_err(backup_tag_error)?;
            let policy = sqlx::query("SELECT * FROM backuppolicies WHERE id=$1")
                .bind(id)
                .fetch_one(&mut *transaction)
                .await
                .map_err(storage)
                .and_then(map_policy)?;
            let activity = citadel_activities::ActivityEvent::new_backup_policy_event(
                policy.id,
                policy.name.clone(),
                actor,
                citadel_activities::ActivityEventInfo::BackupPolicyCreated {
                    policy: policy_metadata::activity_snapshot(&policy)?,
                },
                Utc::now(),
            )
            .map_err(storage)?;
            crate::persistence::postgres::activities::store::insert_activity(
                &mut transaction,
                &activity,
            )
            .await
            .map_err(storage)?;
            transaction.commit().await.map_err(storage)?;
            Ok(policy)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn list_policies_impl(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<BackupPolicy>, BackupError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT policy.* FROM backuppolicies policy
WHERE policy.archivedat IS NULL
  AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid=scope.actorid
      WHERE access.resourcetype=$2 AND access.resourceid=policy.id
        AND access.permissionlevel = ANY($3)
  ))
ORDER BY policy.name,policy.id"#
            );
            let mut policies = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BackupPolicy as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_policy)
                .collect::<Result<Vec<_>, _>>()?;
            self.attach_policy_summaries(&mut policies).await?;
            Ok(policies)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn get_policy_impl(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<BackupPolicy, BackupError>> {
        Box::pin(async move {
            let mut policy =
                sqlx::query("SELECT * FROM backuppolicies WHERE id=$1 AND archivedat IS NULL")
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(storage)?
                    .ok_or(BackupError::NotFound)
                    .and_then(map_policy)?;
            self.attach_policy_summaries(std::slice::from_mut(&mut policy))
                .await?;
            Ok(policy)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn rename_policy_impl<'a>(
        &'a self,
        actor: ActorId,
        input: &'a policy_metadata::RenameBackupPolicyInput,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let old = sqlx::query(
                "SELECT * FROM backuppolicies WHERE id=$1 AND archivedat IS NULL FOR UPDATE",
            )
            .bind(input.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(BackupError::NotFound)
            .and_then(map_policy)?;
            let row = sqlx::query("UPDATE backuppolicies SET name=$2,normalizedname=$3,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 RETURNING *")
                .bind(input.id).bind(&input.name).bind(input.name.to_uppercase())
                .fetch_one(&mut *tx).await.map_err(|error| {
                    if error.as_database_error().is_some_and(|db| db.is_unique_violation() && db.constraint() == Some("ix_backuppolicies_normalizedname")) {
                        BackupError::Conflict("Backup Policy name already exists.".into())
                    } else { storage(error) }
                })?;
            let policy = map_policy(row)?;
            let activity = citadel_activities::ActivityEvent::new_backup_policy_event(
                policy.id,
                policy.name.clone(),
                actor,
                citadel_activities::ActivityEventInfo::BackupPolicyRenamed {
                    old_name: old.name,
                    new_name: policy.name.clone(),
                },
                Utc::now(),
            )
            .map_err(storage)?;
            crate::persistence::postgres::activities::store::insert_activity(&mut tx, &activity)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(policy)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn update_policy_impl<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        row_version: i64,
        input: &'a BackupPolicyConfiguration,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let old = sqlx::query(
                "SELECT * FROM backuppolicies WHERE id=$1 AND archivedat IS NULL FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(BackupError::NotFound)
            .and_then(map_policy)?;
            if old.row_version != row_version {
                return Err(BackupError::Conflict(
                    "Backup Policy changed; reload before saving.".into(),
                ));
            }
            citadel_backups::policies::patch::guard(&old, input)?;
            let row = sqlx::query("UPDATE backuppolicies SET description=$2,source=$3,backuprepositoryid=$4,enabled=$5,cron=$6,timezone=$7,webhook=$8,keeplastsuccessful=$9,timeoutseconds=$10,alertonfailure=$11,runasactorid=$12,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 RETURNING *")
                .bind(id).bind(&input.description).bind(&input.source).bind(input.backup_repository_id).bind(input.enabled).bind(&input.cron).bind(&input.time_zone).bind(&input.webhook).bind(input.keep_last_successful).bind(input.timeout_seconds).bind(input.alert_on_failure).bind(input.run_as_actor_id)
                .fetch_one(&mut *tx).await.map_err(storage)?;
            let policy = map_policy(row)?;
            let activity = citadel_activities::ActivityEvent::new_backup_policy_event(
                policy.id,
                policy.name.clone(),
                actor,
                citadel_activities::ActivityEventInfo::BackupPolicyUpdated {
                    old_policy: policy_metadata::activity_snapshot(&old)?,
                    new_policy: policy_metadata::activity_snapshot(&policy)?,
                },
                Utc::now(),
            )
            .map_err(storage)?;
            crate::persistence::postgres::activities::store::insert_activity(&mut tx, &activity)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(policy)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn update_policy_description_impl<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<BackupPolicy, BackupError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let old = sqlx::query(
                "SELECT * FROM backuppolicies WHERE id=$1 AND archivedat IS NULL FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(BackupError::NotFound)
            .and_then(map_policy)?;
            let old_policy = policy_metadata::activity_snapshot(&old)?;
            let row = sqlx::query("UPDATE backuppolicies SET description=$2,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 RETURNING *")
                .bind(id).bind(description).fetch_one(&mut *tx).await.map_err(storage)?;
            let policy = map_policy(row)?;
            let activity = citadel_activities::ActivityEvent::new_backup_policy_event(
                policy.id,
                policy.name.clone(),
                actor,
                citadel_activities::ActivityEventInfo::BackupPolicyUpdated {
                    old_policy,
                    new_policy: policy_metadata::activity_snapshot(&policy)?,
                },
                Utc::now(),
            )
            .map_err(storage)?;
            crate::persistence::postgres::activities::store::insert_activity(&mut tx, &activity)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            Ok(policy)
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn archive_policy_impl(&self, id: Uuid) -> BoxFuture<'_, Result<(), BackupError>> {
        Box::pin(async move {
            let n=sqlx::query("UPDATE backuppolicies SET archivedat=CURRENT_TIMESTAMP,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 AND archivedat IS NULL AND controlstate='Idle'").bind(id).execute(&self.pool).await.map_err(storage)?.rows_affected();
            if n == 0 {
                Err(BackupError::Conflict(
                    "Backup Policy is active or missing.".into(),
                ))
            } else {
                Ok(())
            }
        })
    }
}

impl PostgresBackupPersistence {
    pub(super) fn list_scheduled_policies_impl(
        &self,
    ) -> BoxFuture<'_, Result<Vec<BackupPolicy>, BackupError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM backuppolicies WHERE enabled=true AND archivedat IS NULL AND cron IS NOT NULL ORDER BY id")
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_policy)
                .collect()
        })
    }
}

impl PostgresBackupPersistence {
    // Constant query count for the whole list. Latest runs are selected using the policy index.
    async fn attach_policy_summaries(
        &self,
        policies: &mut [BackupPolicy],
    ) -> Result<(), BackupError> {
        if policies.is_empty() {
            return Ok(());
        }
        let ids = policies.iter().map(|policy| policy.id).collect::<Vec<_>>();
        let mut connection = self.pool.acquire().await.map_err(storage)?;
        let mut tags = resource_tags::load(&mut connection, "BackupPolicy", &ids)
            .await
            .map_err(storage)?;
        let rows = sqlx::query("SELECT latest.* FROM unnest($1::uuid[]) policy_id CROSS JOIN LATERAL (SELECT * FROM backupruns WHERE backuppolicyid=policy_id ORDER BY queuedat DESC,id DESC LIMIT 1) latest")
            .bind(&ids).fetch_all(&mut *connection).await.map_err(storage)?;
        let mut latest = HashMap::new();
        for row in rows {
            let run = map_run(row)?;
            latest.insert(run.backup_policy_id, run);
        }
        for policy in policies {
            policy.tags = tags.remove(&policy.id).unwrap_or_default();
            policy.latest_run = latest.remove(&policy.id);
        }
        Ok(())
    }
}

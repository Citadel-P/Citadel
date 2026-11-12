use super::*;

pub(super) async fn release(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    repo: Uuid,
    run: Uuid,
    policy: Option<Uuid>,
    success: bool,
) -> Result<(), BackupError> {
    let unsafe_owner: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2 AND (NOT cleanupconfirmed OR coreworkactive OR EXISTS(SELECT 1 FROM dataexecutorreservations WHERE ownerid=$2)))")
        .bind(repo).bind(run).fetch_one(&mut **tx).await.map_err(storage)?;
    if unsafe_owner {
        return Ok(());
    }
    sqlx::query("DELETE FROM backuprepositoryleases WHERE backuprepositoryid=$1 AND ownerrunid=$2")
        .bind(repo)
        .bind(run)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    sqlx::query("DELETE FROM backupsourceleases WHERE ownerrunid=$1")
        .bind(run)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    // Match enqueue: policy before repository. Releasing the repository
    // first can deadlock with a concurrent request holding the policy.
    if let Some(policy) = policy {
        sqlx::query("UPDATE backuppolicies SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,firstsuccessfulrunat=CASE WHEN $3 THEN COALESCE(firstsuccessfulrunat,CURRENT_TIMESTAMP) ELSE firstsuccessfulrunat END,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(policy).bind(run).bind(success).execute(&mut **tx).await.map_err(storage)?;
    }
    sqlx::query("UPDATE backuprepositories SET controlstate='Idle',currentrunid=NULL,controlstartedat=NULL,updatedat=CURRENT_TIMESTAMP,rowversion=rowversion+1 WHERE id=$1 AND currentrunid=$2").bind(repo).bind(run).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}

pub(super) async fn write_logs(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    table: &str,
    column: &str,
    id: Uuid,
    logs: &[BackupLog],
) -> Result<(), BackupError> {
    let sql = match (table, column) {
        ("backuprunlogs", "backuprunid") => {
            "INSERT INTO backuprunlogs(id,backuprunid,createdat,stream,message) SELECT id,$1,CURRENT_TIMESTAMP,stream,message FROM UNNEST($2::uuid[],$3::text[],$4::text[]) AS batch(id,stream,message)"
        }
        ("backuprestorerunlogs", "backuprestorerunid") => {
            "INSERT INTO backuprestorerunlogs(id,backuprestorerunid,createdat,stream,message) SELECT id,$1,CURRENT_TIMESTAMP,stream,message FROM UNNEST($2::uuid[],$3::text[],$4::text[]) AS batch(id,stream,message)"
        }
        _ => return Err(BackupError::Storage("invalid Backup log target".into())),
    };
    let mut start = 0;
    while start < logs.len() {
        let mut end = start;
        let mut bytes = 0usize;
        while end < logs.len() && end - start < 128 {
            let cost = logs[end].message.len().saturating_add(128);
            if end > start && bytes.saturating_add(cost) > 64 * 1024 {
                break;
            }
            bytes = bytes.saturating_add(cost);
            end += 1;
        }
        let batch = &logs[start..end];
        let ids: Vec<_> = batch.iter().map(|_| Uuid::now_v7()).collect();
        let streams: Vec<_> = batch.iter().map(|log| log.stream.as_str()).collect();
        let messages: Vec<_> = batch.iter().map(|log| log.message.as_str()).collect();
        sqlx::query(sql)
            .bind(id)
            .bind(&ids)
            .bind(&streams)
            .bind(&messages)
            .execute(&mut **tx)
            .await
            .map_err(storage)?;
        start = end;
    }
    Ok(())
}

pub(super) fn logs<'a>(
    pool: &'a PgPool,
    table: &str,
    column: &str,
    id: Uuid,
) -> BoxFuture<'a, Result<Vec<BackupLog>, BackupError>> {
    let sql = match (table, column) {
        ("backuprunlogs", "backuprunid") => {
            "SELECT stream,message FROM backuprunlogs WHERE backuprunid=$1 ORDER BY createdat,id"
        }
        ("backuprestorerunlogs", "backuprestorerunid") => {
            "SELECT stream,message FROM backuprestorerunlogs WHERE backuprestorerunid=$1 ORDER BY createdat,id"
        }
        _ => {
            return Box::pin(async {
                Err(BackupError::Storage("invalid Backup log target".into()))
            });
        }
    };
    Box::pin(async move {
        sqlx::query(sql)
            .bind(id)
            .fetch_all(pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(|r| {
                Ok(BackupLog {
                    stream: r.try_get("stream").map_err(storage)?,
                    message: r.try_get("message").map_err(storage)?,
                })
            })
            .collect()
    })
}

pub(super) fn map_repository(r: sqlx::postgres::PgRow) -> Result<BackupRepository, BackupError> {
    Ok(BackupRepository {
        id: r.try_get("id").map_err(storage)?,
        name: r.try_get("name").map_err(storage)?,
        normalized_name: r.try_get("normalizedname").map_err(storage)?,
        repository_type: r.try_get("type").map_err(storage)?,
        spec: r
            .try_get::<sqlx::types::Json<citadel_backups::spec::BackupRepositorySpec>, _>("spec")
            .map_err(storage)?
            .0,
        password_secret_id: r.try_get("passwordsecretid").map_err(storage)?,
        status: r
            .try_get::<String, _>("status")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        control_state: r
            .try_get::<String, _>("controlstate")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        current_run_id: r.try_get("currentrunid").map_err(storage)?,
        control_started_at: r.try_get("controlstartedat").map_err(storage)?,
        last_pruned_at: r.try_get("lastprunedat").map_err(storage)?,
        last_checked_at: r.try_get("lastcheckedat").map_err(storage)?,

        updated_at: r.try_get("updatedat").map_err(storage)?,
        archived_at: r.try_get("archivedat").map_err(storage)?,
        row_version: r.try_get("rowversion").map_err(storage)?,

        audit: citadel_primitives::AuditMetadata {
            created_at: r.try_get("createdat").map_err(storage)?,
            created_by_actor_id: citadel_primitives::ActorId::new(
                r.try_get("createdbyactorid").map_err(storage)?,
            ),
        },
    })
}

pub(super) fn map_policy(r: sqlx::postgres::PgRow) -> Result<BackupPolicy, BackupError> {
    Ok(BackupPolicy {
        tags: Vec::new(),
        latest_run: None,
        id: r.try_get("id").map_err(storage)?,
        name: r.try_get("name").map_err(storage)?,
        normalized_name: r.try_get("normalizedname").map_err(storage)?,
        description: r.try_get("description").map_err(storage)?,
        source: r
            .try_get::<sqlx::types::Json<citadel_backups::spec::BackupSourceSpec>, _>("source")
            .map_err(storage)?
            .0,
        backup_repository_id: r.try_get("backuprepositoryid").map_err(storage)?,
        enabled: r.try_get("enabled").map_err(storage)?,
        cron: r.try_get("cron").map_err(storage)?,
        time_zone: r.try_get("timezone").map_err(storage)?,
        webhook: r
            .try_get::<Option<sqlx::types::Json<Option<citadel_primitives::WebhookConfig>>>, _>(
                "webhook",
            )
            .map_err(storage)?
            .and_then(|v| v.0),
        keep_last_successful: r.try_get("keeplastsuccessful").map_err(storage)?,
        timeout_seconds: r.try_get("timeoutseconds").map_err(storage)?,
        alert_on_failure: r.try_get("alertonfailure").map_err(storage)?,
        run_as_actor_id: r.try_get("runasactorid").map_err(storage)?,
        control_state: r
            .try_get::<String, _>("controlstate")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        current_run_id: r.try_get("currentrunid").map_err(storage)?,
        last_scheduled_run_at: r.try_get("lastscheduledrunat").map_err(storage)?,
        first_successful_run_at: r.try_get("firstsuccessfulrunat").map_err(storage)?,

        updated_at: r.try_get("updatedat").map_err(storage)?,
        archived_at: r.try_get("archivedat").map_err(storage)?,
        row_version: r.try_get("rowversion").map_err(storage)?,

        audit: citadel_primitives::AuditMetadata {
            created_at: r.try_get("createdat").map_err(storage)?,
            created_by_actor_id: citadel_primitives::ActorId::new(
                r.try_get("createdbyactorid").map_err(storage)?,
            ),
        },
    })
}

pub(super) fn map_run(r: sqlx::postgres::PgRow) -> Result<BackupRun, BackupError> {
    Ok(BackupRun {
        id: r.try_get("id").map_err(storage)?,
        backup_policy_id: r.try_get("backuppolicyid").map_err(storage)?,
        policy_name_snapshot: r.try_get("policynamesnapshot").map_err(storage)?,
        backup_repository_id: r.try_get("backuprepositoryid").map_err(storage)?,
        repository_type_snapshot: r.try_get("repositorytypesnapshot").map_err(storage)?,
        source_snapshot: r
            .try_get::<sqlx::types::Json<citadel_backups::spec::BackupSourceSpec>, _>(
                "sourcesnapshot",
            )
            .map_err(storage)?
            .0,
        trigger: r.try_get("trigger").map_err(storage)?,
        status: r
            .try_get::<String, _>("status")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        snapshot_availability: r
            .try_get::<String, _>("snapshotavailability")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        restic_snapshot_id: r.try_get("resticsnapshotid").map_err(storage)?,
        parent_snapshot_id: r.try_get("parentsnapshotid").map_err(storage)?,
        files_processed: r.try_get("filesprocessed").map_err(storage)?,
        bytes_processed: r.try_get("bytesprocessed").map_err(storage)?,
        bytes_added: r.try_get("bytesadded").map_err(storage)?,
        warnings: r
            .try_get::<sqlx::types::Json<Vec<String>>, _>("warnings")
            .map_err(storage)?
            .0,
        queued_at: r.try_get("queuedat").map_err(storage)?,
        started_at: r.try_get("startedat").map_err(storage)?,
        completed_at: r.try_get("completedat").map_err(storage)?,
        exit_code: r.try_get("exitcode").map_err(storage)?,
        error_code: r.try_get("errorcode").map_err(storage)?,
        error_message: r.try_get("errormessage").map_err(storage)?,
        triggered_by_actor_id: r.try_get("triggeredbyactorid").map_err(storage)?,
        items: vec![],
    })
}

pub(super) async fn attach_run_items(
    pool: &PgPool,
    runs: &mut [BackupRun],
) -> Result<(), BackupError> {
    if runs.is_empty() {
        return Ok(());
    }
    let ids = runs.iter().map(|run| run.id).collect::<Vec<_>>();
    let rows =
        sqlx::query("SELECT * FROM backuprunitems WHERE backuprunid=ANY($1) ORDER BY createdat,id")
            .bind(&ids)
            .fetch_all(pool)
            .await
            .map_err(storage)?;
    let mut grouped = HashMap::<Uuid, Vec<BackupRunItem>>::new();
    for row in rows {
        let item = map_run_item(row)?;
        grouped.entry(item.backup_run_id).or_default().push(item);
    }
    for run in runs {
        run.items = grouped.remove(&run.id).unwrap_or_default();
    }
    Ok(())
}

pub(super) fn map_run_item(row: sqlx::postgres::PgRow) -> Result<BackupRunItem, BackupError> {
    Ok(BackupRunItem {
        id: row.try_get("id").map_err(storage)?,
        backup_run_id: row.try_get("backuprunid").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        volume_name: row.try_get("volumename").map_err(storage)?,
        host_path: row.try_get("hostpath").map_err(storage)?,
        docker_node_id: row.try_get("dockernodeid").map_err(storage)?,
        node_hostname: row.try_get("nodehostname").map_err(storage)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        restic_snapshot_id: row.try_get("resticsnapshotid").map_err(storage)?,
        parent_snapshot_id: row.try_get("parentsnapshotid").map_err(storage)?,
        files_processed: row.try_get("filesprocessed").map_err(storage)?,
        bytes_processed: row.try_get("bytesprocessed").map_err(storage)?,
        bytes_added: row.try_get("bytesadded").map_err(storage)?,
        started_at: row.try_get("startedat").map_err(storage)?,
        completed_at: row.try_get("completedat").map_err(storage)?,
        exit_code: row.try_get("exitcode").map_err(storage)?,
        error_code: row.try_get("errorcode").map_err(storage)?,
        error_message: row.try_get("errormessage").map_err(storage)?,
    })
}

pub(super) fn map_restore(r: sqlx::postgres::PgRow) -> Result<BackupRestoreRun, BackupError> {
    Ok(BackupRestoreRun {
        id: r.try_get("id").map_err(storage)?,
        backup_run_id: r.try_get("backuprunid").map_err(storage)?,
        backup_repository_id: r.try_get("backuprepositoryid").map_err(storage)?,
        source_backup_run_item_id: r.try_get("sourcebackuprunitemid").map_err(storage)?,
        target_platform_id: r.try_get("targetplatformid").map_err(storage)?,
        target_docker_node_id: r.try_get("targetdockernodeid").map_err(storage)?,
        target_volume_name: r.try_get("targetvolumename").map_err(storage)?,
        target_host_path: r.try_get("targethostpath").map_err(storage)?,
        overwrite_existing: r.try_get("overwriteexisting").map_err(storage)?,
        status: r
            .try_get::<String, _>("status")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        queued_at: r.try_get("queuedat").map_err(storage)?,
        started_at: r.try_get("startedat").map_err(storage)?,
        completed_at: r.try_get("completedat").map_err(storage)?,
        exit_code: r.try_get("exitcode").map_err(storage)?,
        error_code: r.try_get("errorcode").map_err(storage)?,
        error_message: r.try_get("errormessage").map_err(storage)?,
        triggered_by_actor_id: r.try_get("triggeredbyactorid").map_err(storage)?,
    })
}

pub(super) fn backup_source_key(source: &Value) -> Result<String, BackupError> {
    let source: citadel_backups::spec::BackupSourceSpec =
        serde_json::from_value(source.clone()).map_err(storage)?;
    source.validate()?;
    Ok(source.key())
}

pub(super) fn docker_volume_key(platform: Uuid, node: Option<&str>, volume: &str) -> String {
    citadel_backups::spec::docker_volume_source_key(platform, node, volume)
}

pub(super) fn storage(e: impl std::fmt::Display) -> BackupError {
    BackupError::Storage(e.to_string())
}

pub(super) fn backup_tag_error(error: resource_tags::ResourceTagError) -> BackupError {
    match error {
        resource_tags::ResourceTagError::Missing => {
            BackupError::Validation("One or more selected Tags do not exist.".to_owned())
        }
        resource_tags::ResourceTagError::Database(error) => storage(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Connection;

    #[tokio::test]
    #[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
    async fn mixed_log_batches_use_one_statement_preserve_order_and_rollback() {
        let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
        let mut connection = sqlx::PgConnection::connect(&url).await.unwrap();
        sqlx::raw_sql("CREATE TEMP TABLE backuprunlogs(id uuid,backuprunid uuid,createdat timestamptz,stream text,message text)")
            .execute(&mut connection).await.unwrap();
        let run = Uuid::now_v7();
        let logs: Vec<_> = (0..128)
            .map(|n| BackupLog {
                stream: "stdout".into(),
                message: format!("{n}: données 日本語"),
            })
            .collect();
        // Same connection/settings and payloads; timing is diagnostic, not a CI threshold.
        for arrays in [false, true] {
            connection.clear_cached_statements().await.unwrap();
            let start = std::time::Instant::now();
            for _ in 0..2 {
                for size in 1..=128 {
                    let mut tx = connection.begin().await.unwrap();
                    if arrays {
                        write_logs(&mut tx, "backuprunlogs", "backuprunid", run, &logs[..size])
                            .await
                            .unwrap();
                    } else {
                        let mut query = sqlx::QueryBuilder::<sqlx::Postgres>::new(
                            "INSERT INTO backuprunlogs(id,backuprunid,createdat,stream,message) ",
                        );
                        query.push_values(&logs[..size], |mut row, log| {
                            row.push_bind(Uuid::now_v7())
                                .push_bind(run)
                                .push("CURRENT_TIMESTAMP")
                                .push_bind(&log.stream)
                                .push_bind(&log.message);
                        });
                        query.build().execute(&mut *tx).await.unwrap();
                    }
                    tx.rollback().await.unwrap();
                }
            }
            let cached = connection.cached_statements_size();
            eprintln!(
                "log batches arrays={arrays}: {:?}, cached statements={cached}",
                start.elapsed()
            );
            if arrays {
                assert_eq!(cached, 1);
            } else {
                assert!(cached > 1);
            }
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM backuprunlogs")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        assert_eq!(count, 0, "all benchmark transactions rolled back");
        let mut tx = connection.begin().await.unwrap();
        // Span row and byte boundaries; array insertion must retain UTF-8 and order.
        let mut logs = logs;
        logs.extend((128..300).map(|n| BackupLog {
            stream: "stderr".into(),
            message: format!("{n}: {}", "é".repeat(1024)),
        }));
        write_logs(&mut tx, "backuprunlogs", "backuprunid", run, &logs)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let persisted: Vec<(String, String)> =
            sqlx::query_as("SELECT stream,message FROM backuprunlogs ORDER BY createdat,id")
                .fetch_all(&mut connection)
                .await
                .unwrap();
        assert_eq!(
            persisted,
            logs.into_iter()
                .map(|log| (log.stream, log.message))
                .collect::<Vec<_>>()
        );
        connection.close().await.unwrap();
    }
}

use super::*;

pub(super) async fn exists_run(pool: &PgPool, id: Uuid) -> Result<bool, BuildError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM buildruns WHERE id=$1)")
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(storage)
}

pub(super) async fn get_run(pool: &PgPool, id: Uuid) -> Result<BuildRun, BuildError> {
    sqlx::query("SELECT * FROM buildruns WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(BuildError::NotFound)
        .and_then(map_run)
}

pub(super) async fn enrich_pools(
    connection: &mut sqlx::PgConnection,
    pools: &mut [BuildAgentPool],
) -> Result<(), BuildError> {
    let ids: Vec<_> = pools.iter().map(|pool| pool.id).collect();
    let mut tags = resource_tags::load(connection, "BuildAgentPool", &ids)
        .await
        .map_err(storage)?;
    for pool in pools {
        pool.tags = tags.remove(&pool.id).unwrap_or_default();
    }
    Ok(())
}

pub(super) async fn enrich_projects(
    connection: &mut sqlx::PgConnection,
    projects: Vec<BuildProject>,
) -> Result<Vec<BuildProject>, BuildError> {
    if projects.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<_> = projects.iter().map(|project| project.id).collect();
    let mut tags = resource_tags::load(&mut *connection, "Build", &ids)
        .await
        .map_err(storage)?;
    let rows = sqlx::query("SELECT run.* FROM unnest($1::uuid[]) ids(id) CROSS JOIN LATERAL (SELECT * FROM buildruns WHERE buildprojectid=ids.id ORDER BY queuedat DESC,id DESC LIMIT 1) run").bind(&ids).fetch_all(connection).await.map_err(storage)?;
    let mut runs = std::collections::HashMap::new();
    for row in rows {
        let run = map_run(row)?;
        runs.insert(run.build_project_id, run);
    }
    Ok(projects
        .into_iter()
        .map(|project| BuildProject {
            tags: tags.remove(&project.id).unwrap_or_default(),
            latest_run: runs.remove(&project.id),
            ..project
        })
        .collect())
}

pub(super) fn map_project(row: sqlx::postgres::PgRow) -> Result<BuildProject, BuildError> {
    Ok(BuildProject {
        tags: Vec::new(),
        latest_run: None,
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        normalized_name: row.try_get("normalizedname").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        enabled: row.try_get("enabled").map_err(storage)?,
        git_repository_id: row.try_get("gitrepositoryid").map_err(storage)?,
        branch: row.try_get("branch").map_err(storage)?,
        context_path: row.try_get("contextpath").map_err(storage)?,
        dockerfile_path: row.try_get("dockerfilepath").map_err(storage)?,
        target: row.try_get("target").map_err(storage)?,
        build_args: serde_json::from_value(row.try_get("buildargs").map_err(storage)?)
            .map_err(storage)?,
        build_secrets: serde_json::from_value(row.try_get("buildsecrets").map_err(storage)?)
            .map_err(storage)?,
        builder_kind: row.try_get("builderkind").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        build_agent_pool_id: row.try_get("buildagentpoolid").map_err(storage)?,
        registry_id: row.try_get("registryid").map_err(storage)?,
        image_repository: row.try_get("imagerepository").map_err(storage)?,
        tag_templates: serde_json::from_value(row.try_get("tagtemplates").map_err(storage)?)
            .map_err(storage)?,
        webhook: row
            .try_get::<Option<sqlx::types::Json<Option<citadel_primitives::WebhookConfig>>>, _>(
                "webhook",
            )
            .map_err(storage)?
            .and_then(|v| v.0),
        timeout_seconds: row.try_get("timeoutseconds").map_err(storage)?,
        retention_run_count: row.try_get("retentionruncount").map_err(storage)?,
        current_run_id: row.try_get("currentrunid").map_err(storage)?,
        control_state: row
            .try_get::<String, _>("controlstate")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        control_started_at: row.try_get("controlstartedat").map_err(storage)?,

        updated_at: row.try_get("updatedat").map_err(storage)?,
        archived_at: row.try_get("archivedat").map_err(storage)?,
        row_version: row.try_get("rowversion").map_err(storage)?,

        audit: citadel_primitives::AuditMetadata {
            created_at: row.try_get("createdat").map_err(storage)?,
            created_by_actor_id: citadel_primitives::ActorId::new(
                row.try_get("createdbyactorid").map_err(storage)?,
            ),
        },
    })
}

pub(super) fn map_pool(row: sqlx::postgres::PgRow) -> Result<BuildAgentPool, BuildError> {
    Ok(BuildAgentPool {
        tags: Vec::new(),
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        normalized_name: row.try_get("normalizedname").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        enabled: row.try_get("enabled").map_err(storage)?,
        provider: row.try_get("provider").map_err(storage)?,
        provider_spec: row.try_get("providerspec").map_err(storage)?,
        max_active_builders: row.try_get("maxactivebuilders").map_err(storage)?,
        queue_timeout_seconds: row.try_get("queuetimeoutseconds").map_err(storage)?,
        provisioning_timeout_seconds: row.try_get("provisioningtimeoutseconds").map_err(storage)?,
        registration_timeout_seconds: row.try_get("registrationtimeoutseconds").map_err(storage)?,
        heartbeat_timeout_seconds: row.try_get("heartbeattimeoutseconds").map_err(storage)?,
        cleanup_timeout_seconds: row.try_get("cleanuptimeoutseconds").map_err(storage)?,
        maximum_instance_lifetime_seconds: row
            .try_get("maximuminstancelifetimeseconds")
            .map_err(storage)?,
        failure_retention_minutes: row.try_get("failureretentionminutes").map_err(storage)?,
        last_validation_status: row
            .try_get::<String, _>("lastvalidationstatus")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        last_validation_message: row.try_get("lastvalidationmessage").map_err(storage)?,
        last_validated_at: row.try_get("lastvalidatedat").map_err(storage)?,
        control_state: row
            .try_get::<String, _>("controlstate")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        control_triggered_by: row.try_get("controltriggeredby").map_err(storage)?,
        control_started_at: row.try_get("controlstartedat").map_err(storage)?,

        updated_at: row.try_get("updatedat").map_err(storage)?,
        archived_at: row.try_get("archivedat").map_err(storage)?,
        row_version: row.try_get("rowversion").map_err(storage)?,

        audit: citadel_primitives::AuditMetadata {
            created_at: row.try_get("createdat").map_err(storage)?,
            created_by_actor_id: citadel_primitives::ActorId::new(
                row.try_get("createdbyactorid").map_err(storage)?,
            ),
        },
    })
}

pub(super) fn map_run(row: sqlx::postgres::PgRow) -> Result<BuildRun, BuildError> {
    let registry_snapshot: serde_json::Value = row.try_get("registrysnapshot").map_err(storage)?;
    let registry_id = registry_snapshot
        .get("id")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| BuildError::Storage("Build Run Registry snapshot is invalid.".to_owned()))?;
    let registry_host = registry_snapshot
        .get("registryHost")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| BuildError::Storage("Build Run Registry host is missing.".to_owned()))?
        .to_owned();
    Ok(BuildRun {
        id: row.try_get("id").map_err(storage)?,
        build_project_id: row.try_get("buildprojectid").map_err(storage)?,
        project_name_snapshot: row.try_get("projectnamesnapshot").map_err(storage)?,
        git_repository_id: row.try_get("gitrepositoryid").map_err(storage)?,
        git_repository_name_snapshot: row.try_get("gitrepositorynamesnapshot").map_err(storage)?,
        platform_snapshot: serde_json::from_value(
            row.try_get("platformsnapshot").map_err(storage)?,
        )
        .map_err(storage)?,
        branch: row.try_get("branch").map_err(storage)?,
        resolved_commit_sha: row.try_get("resolvedcommitsha").map_err(storage)?,
        context_path: row.try_get("contextpath").map_err(storage)?,
        dockerfile_path: row.try_get("dockerfilepath").map_err(storage)?,
        target: row.try_get("target").map_err(storage)?,
        registry_id,
        registry_host,
        image_repository: row.try_get("imagerepository").map_err(storage)?,
        image_references: serde_json::from_value(row.try_get("imagereferences").map_err(storage)?)
            .map_err(storage)?,
        trigger: row.try_get("trigger").map_err(storage)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
        image_digest: row.try_get("imagedigest").map_err(storage)?,
        timeout_seconds: row.try_get("timeoutseconds").map_err(storage)?,
        queued_at: row.try_get("queuedat").map_err(storage)?,
        started_at: row.try_get("startedat").map_err(storage)?,
        completed_at: row.try_get("completedat").map_err(storage)?,
        exit_code: row.try_get("exitcode").map_err(storage)?,
        error_code: row.try_get("errorcode").map_err(storage)?,
        error_message: row.try_get("errormessage").map_err(storage)?,
        triggered_by_actor_id: row.try_get("triggeredbyactorid").map_err(storage)?,
    })
}

pub(super) fn storage(error: impl std::fmt::Display) -> BuildError {
    BuildError::Storage(error.to_string())
}

pub(super) fn build_tag_error(error: resource_tags::ResourceTagError) -> BuildError {
    match error {
        resource_tags::ResourceTagError::Missing => {
            BuildError::Validation("One or more selected Tags do not exist.".to_owned())
        }
        resource_tags::ResourceTagError::Database(error) => database(error),
    }
}

pub(super) fn database(error: sqlx::Error) -> BuildError {
    if error
        .as_database_error()
        .is_some_and(|value| value.is_unique_violation())
    {
        BuildError::Conflict("Build Project already exists or has an active Run.".to_owned())
    } else {
        storage(error)
    }
}

use citadel_deployments::{DeploymentError, ResolvedDeploymentBuild};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub(crate) async fn local_image(
    pool: &PgPool,
    platform_id: Uuid,
    id: Uuid,
) -> Result<String, DeploymentError> {
    sqlx::query_scalar::<_, String>(
        "SELECT dockerimageid FROM images WHERE id=$1 AND platformid=$2",
    )
    .bind(id)
    .bind(platform_id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?
    .ok_or_else(|| {
        DeploymentError::Validation(
            "The selected local image is not available on this platform.".to_owned(),
        )
    })
}

pub(crate) async fn build_image(
    pool: &PgPool,
    build_project_id: Uuid,
    resolved_build_run_id: Option<Uuid>,
) -> Result<(Uuid, ResolvedDeploymentBuild), DeploymentError> {
    let row = sqlx::query(
            r#"SELECT (run.registrysnapshot->>'id')::uuid AS registryid,run.id,run.imagereferences,run.imagedigest
FROM buildprojects project
JOIN LATERAL (
    SELECT candidate.id,candidate.imagereferences,candidate.imagedigest,candidate.registrysnapshot
    FROM buildruns candidate
    WHERE candidate.buildprojectid=project.id
      AND candidate.status='Succeeded'
      AND candidate.registrysnapshot <> 'null'::jsonb
      AND ($2::uuid IS NULL OR candidate.id=$2)
    ORDER BY candidate.completedat DESC NULLS LAST,candidate.id DESC
    LIMIT 1
) run ON TRUE
WHERE project.id=$1 AND project.enabled AND project.archivedat IS NULL"#,
        )
        .bind(build_project_id)
        .bind(resolved_build_run_id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or_else(|| {
            DeploymentError::Validation(
                "The selected Build Project has no successful Build Run to deploy.".to_owned(),
            )
        })?;
    let references: Value = row.try_get("imagereferences").map_err(storage)?;
    let reference = references
        .as_array()
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            DeploymentError::Validation(
                "The selected Build Run has no deployable image reference.".to_owned(),
            )
        })?
        .to_owned();
    let build = ResolvedDeploymentBuild {
        image_reference: reference.clone(),
        digest: row.try_get("imagedigest").map_err(storage)?,
        build_run_id: row.try_get("id").map_err(storage)?,
    };
    Ok((row.try_get("registryid").map_err(storage)?, build))
}
fn storage(error: sqlx::Error) -> DeploymentError {
    DeploymentError::Storage(error.to_string())
}

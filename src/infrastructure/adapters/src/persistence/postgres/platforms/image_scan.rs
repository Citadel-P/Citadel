use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) const IMAGE_PAGE: i64 = 100;
// A Web Editor definition can contain 2 MiB of Compose. Release the query
// connection before reading Git files without retaining 100 such documents.
pub(crate) const STACK_PAGE: i64 = 4;

pub(crate) async fn external_images(
    pool: &PgPool,
    after: Uuid,
    swarm: bool,
) -> Result<Vec<(Uuid, Uuid, Option<String>, Option<String>)>, sqlx::Error> {
    sqlx::query_as(r#"
        WITH workloads AS (
            SELECT id, platformid, spec FROM deployments WHERE controlstate='Idle' AND NOT $2
            UNION ALL SELECT id, platformid, spec FROM swarmservices WHERE controlstate='Idle' AND $2
        ), idle AS (
            SELECT d.id, d.platformid, COALESCE(d.spec->'Image',d.spec->'image') image
            FROM workloads d JOIN platforms p ON p.id=d.platformid
            WHERE d.id>$1 AND p.status='Online'
            AND p.platformdescriptor->>'$type'=CASE WHEN $2 THEN 'DockerSwarm' ELSE 'Docker' END
            AND COALESCE(d.spec->>'UpdateBehavior',d.spec->>'updateBehavior') IN ('Notify','notify','AutoDeploy','autoDeploy','autodeploy')
        )
        SELECT id, platformid, COALESCE(image->>'RegistryId',image->>'registryId'),
            COALESCE(image->>'ImageTag',image->>'imageTag') FROM idle
        WHERE image->>'$type'='External' ORDER BY id LIMIT $3
    "#).bind(after).bind(swarm).bind(IMAGE_PAGE).fetch_all(pool).await
}

pub(crate) async fn stack_images(
    pool: &PgPool,
    after: Uuid,
) -> Result<Vec<(Uuid, Uuid, Value, Option<Value>)>, sqlx::Error> {
    sqlx::query_as(r#"
        SELECT s.id,r.platformid,r.spec,r.source FROM stacks s
        JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid
        WHERE s.id>$1 AND s.controlstate='Idle' AND p.status='Online'
        AND p.platformdescriptor->>'$type' IN ('Docker','DockerStandalone','DockerSwarm')
        AND r.status IN ('Healthy','Degraded')
        AND lower(COALESCE(r.spec->>'UpdateBehavior',r.spec->>'updateBehavior')) IN ('notify','stackautodeploy','serviceautodeploy')
        AND COALESCE(r.spec->>'RegistryId',r.spec->>'registryId','') NOT IN ('','00000000-0000-0000-0000-000000000000')
        ORDER BY s.id LIMIT $2
    "#).bind(after).bind(STACK_PAGE).fetch_all(pool).await
}

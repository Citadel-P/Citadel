use sqlx::PgPool;
use uuid::Uuid;

pub(crate) async fn candidates(
    pool: &PgPool,
    kind: &str,
    after: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM (SELECT d.id,'Deployment' kind FROM deployments d JOIN platforms p ON p.id=d.platformid WHERE p.platformdescriptor->>'$type'='Docker' AND p.status='Online' AND d.controlstate='Idle' UNION ALL SELECT s.id,'Stack' FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE p.platformdescriptor->>'$type' IN ('Docker','DockerStandalone','DockerSwarm') AND p.status='Online' AND s.controlstate='Idle' UNION ALL SELECT s.id,'SwarmService' FROM swarmservices s JOIN platforms p ON p.id=s.platformid WHERE p.platformdescriptor->>'$type'='DockerSwarm' AND p.status='Online' AND s.controlstate='Idle') candidates WHERE kind=$1 AND id>$2 ORDER BY id LIMIT 100")
                    .bind(kind).bind(after).fetch_all(pool).await
}

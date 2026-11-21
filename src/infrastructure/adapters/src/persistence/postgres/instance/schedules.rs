use chrono::{DateTime, Utc};
use citadel_instance::{InstanceError, ScheduleDefinition, SchedulePurpose, ScheduleReader};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

pub struct PostgresSchedules(pub PgPool);

fn storage(e: sqlx::Error) -> InstanceError {
    InstanceError::Storage(e.to_string())
}
// These projections share one result shape. Filtering and ordering stay inside one
// resource at a time so PostgreSQL can stop at the page limit on its primary key.
fn projection(kind: SchedulePurpose) -> &'static str {
    use SchedulePurpose::*;
    match kind {
        Script => "SELECT id, name, NULL::uuid, enabled AND scheduleenabled, schedulecron,
                          scheduletimezone, NULL::bigint, lastscheduledrunat, NULL::text, NULL::text, timeoutseconds
                   FROM scripts WHERE schedulecron IS NOT NULL",
        Backup => "SELECT p.id, p.name, NULL::uuid, p.enabled, p.cron, COALESCE(p.timezone,'UTC'),
                          NULL::bigint, p.lastscheduledrunat, NULL::text,
                          CASE WHEN r.archivedat IS NOT NULL THEN 'The backup repository is archived.' END
                   FROM backuppolicies p JOIN backuprepositories r ON r.id=p.backuprepositoryid
                   WHERE p.archivedat IS NULL AND p.cron IS NOT NULL",
        Install => "SELECT singleton.id, 'Core and Agent installation'::text, NULL::uuid,
                           COALESCE((a.schedule->>'enabled')::boolean,false),
                           COALESCE(a.schedule->>'cron','0 3 * * *'), COALESCE(a.schedule->>'timeZone','UTC'),
                           NULL::bigint, a.lastscheduledminute, NULL::text, NULL::text
                    FROM (VALUES ('00000000-0000-0000-0000-000000000000'::uuid)) singleton(id)
                    LEFT JOIN agentupdatesettings a ON a.id=1",
        Release => "SELECT '00000000-0000-0000-0000-000000000000'::uuid, 'Core release checks'::text, NULL::uuid,
                           COALESCE(s.initializedat IS NOT NULL AND s.updatechecksenabled,false),
                           NULL::text, NULL::text, COALESCE(s.updatecheckintervalhours,6)::bigint*3600,
                           c.lastattempt, NULL::text, NULL::text
                    FROM (VALUES (1)) singleton(id)
                    LEFT JOIN instancesetupstates s ON s.id=singleton.id
                    LEFT JOIN instanceupdatechecks c ON c.id=singleton.id",
        Git => "SELECT r.id, r.name, NULL::uuid, r.syncmode='PullInterval', NULL::text, NULL::text,
                       COALESCE(r.syncintervalminutes,5)::bigint*60,
                       (SELECT greatest(min(due),now()) FROM git_schedule_branches b
                        WHERE b.id=r.id HAVING count(*)>0), NULL::text,
                       CASE WHEN r.controlstate<>'Idle' THEN 'Repository synchronization is busy.' END
                FROM gitrepositories r",
        DeploymentImages => "SELECT d.id, d.name, d.platformid, COALESCE(d.spec->>'UpdateBehavior' IN ('Notify','AutoDeploy'),false),
                                    NULL::text, NULL::text, NULL::bigint, NULL::timestamptz, NULL::text,
                                    CASE WHEN p.status IS DISTINCT FROM 'Online' THEN 'Platform is offline.'
                                         WHEN p.platformdescriptor->>'$type' IS DISTINCT FROM 'Docker' THEN 'Requires a standalone Docker platform.'
                                         WHEN d.spec->'Image'->>'$type' IS DISTINCT FROM 'External'
                                           OR COALESCE(d.spec->'Image'->>'ResolvedDigest','')=''
                                           OR position('@' in COALESCE(d.spec->'Image'->>'ImageTag',''))>0
                                         THEN 'Requires an applied external image with a tracked tag.' END
                             FROM deployments d LEFT JOIN platforms p ON p.id=d.platformid",
        ServiceImages => "SELECT s.id, s.name, s.platformid, COALESCE(s.spec->>'UpdateBehavior' IN ('Notify','AutoDeploy'),false),
                                 NULL::text, NULL::text, NULL::bigint, NULL::timestamptz, NULL::text,
                                 CASE WHEN p.status IS DISTINCT FROM 'Online' THEN 'Platform is offline.'
                                      WHEN p.platformdescriptor->>'$type' IS DISTINCT FROM 'DockerSwarm' THEN 'Requires a Docker Swarm platform.'
                                      WHEN s.spec->'Image'->>'$type' IS DISTINCT FROM 'External'
                                        OR COALESCE(s.appliedimagedigest,'')='' OR s.spec->'Image'->>'ImageTag' IS NULL
                                        OR position('@' in COALESCE(s.spec->'Image'->>'ImageTag',''))>0
                                      THEN 'Requires an applied external image with a tracked tag.' END
                          FROM swarmservices s LEFT JOIN platforms p ON p.id=s.platformid",
        StackImages => "SELECT s.id, s.name, r.platformid, COALESCE(r.spec->>'UpdateBehavior' IN ('Notify','StackAutoDeploy','ServiceAutoDeploy'),false),
                               NULL::text, NULL::text, NULL::bigint, NULL::timestamptz, NULL::text,
                               CASE WHEN p.status IS DISTINCT FROM 'Online' THEN 'Platform is offline.'
                                    WHEN r.status NOT IN ('Healthy','Degraded') THEN 'Waiting for an applied release.'
                                    WHEN COALESCE(r.spec->>'RegistryId','') IN ('','00000000-0000-0000-0000-000000000000')
                                    THEN 'A registry is required for image checks.' END
                        FROM stacks s LEFT JOIN stackreleases r ON r.id=s.currentstackreleaseid
                        LEFT JOIN platforms p ON p.id=r.platformid WHERE s.stacksource IN ('WebEditor','Git')",
        StackGit => "SELECT s.id, s.name, r.platformid, COALESCE(r.spec->>'UpdateBehavior' IN ('Notify','StackAutoDeploy','ServiceAutoDeploy'),false),
                            NULL::text, NULL::text, NULL::bigint, NULL::timestamptz, NULL::text,
                            CASE WHEN COALESCE(r.spec->>'CommitSha','')<>'' THEN 'The stack is pinned to a commit.'
                                 WHEN p.status IS DISTINCT FROM 'Online' THEN 'Platform is offline.'
                                 WHEN r.status NOT IN ('Healthy','Degraded') THEN 'Waiting for an applied release.' END
                     FROM stacks s LEFT JOIN stackreleases r ON r.id=s.currentstackreleaseid
                     LEFT JOIN platforms p ON p.id=r.platformid WHERE s.stacksource='Git'",
        Image => "SELECT i.id, i.name, i.platformid, true, NULL::text, NULL::text, NULL::bigint,
                         CASE WHEN d.id IS NOT NULL THEN greatest(d.due,now()) END, i.dockerimageid, NULL::text
                  FROM images i LEFT JOIN inventory_image_deadlines d ON d.id=i.id WHERE json_array_length(i.tags)>0",
    }
}
fn definition(purpose: SchedulePurpose, row: PgRow) -> Result<ScheduleDefinition, InstanceError> {
    Ok(ScheduleDefinition {
        purpose,
        script_timeout_seconds: if purpose == SchedulePurpose::Script {
            Some(row.try_get("timeoutseconds").map_err(storage)?)
        } else {
            None
        },
        blocker: row.try_get("blocker").map_err(storage)?,
        docker_image_id: row.try_get("dockerimageid").map_err(storage)?,
        owner: row.try_get("ownerid").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        platform: row.try_get("platformid").map_err(storage)?,
        enabled: row.try_get("enabled").map_err(storage)?,
        cron: row.try_get("cron").map_err(storage)?,
        zone: row.try_get("zone").map_err(storage)?,
        seconds: purpose
            .fixed_interval()
            .or(row.try_get("seconds").map_err(storage)?),
        anchor: row.try_get("anchor").map_err(storage)?,
    })
}
const COLUMNS: &str =
    "(ownerid,name,platformid,enabled,cron,zone,seconds,anchor,dockerimageid,blocker)";
impl PostgresSchedules {
    async fn resource_page(
        &self,
        p: SchedulePurpose,
        after: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<ScheduleDefinition>, InstanceError> {
        let base = projection(p);
        let (predicate, lower) = after.map_or((">=", Uuid::nil()), |id| (">", id));
        let query = format!(
            "SELECT * FROM ({base}) AS schedule{COLUMNS} WHERE ownerid {predicate} $1 ORDER BY ownerid LIMIT $2"
        );
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .bind(lower)
            .bind(limit)
            .fetch_all(&self.0)
            .await
            .map_err(storage)?
            .into_iter()
            .map(|row| definition(p, row))
            .collect()
    }
}
impl ScheduleReader for PostgresSchedules {
    fn page(
        &self,
        purpose: Option<SchedulePurpose>,
        after: Option<(SchedulePurpose, Uuid)>,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<ScheduleDefinition>, InstanceError>> {
        Box::pin(async move {
            let limit = limit.clamp(1, 129);
            if let Some(p) = purpose {
                return self
                    .resource_page(
                        p,
                        after.filter(|(kind, _)| *kind == p).map(|(_, id)| id),
                        limit,
                    )
                    .await;
            }
            let mut rows = Vec::new();
            // Enum order is the stable catalog order, shared with the opaque page cursor.
            for p in SchedulePurpose::ALL {
                if after.is_some_and(|(kind, _)| p < kind) {
                    continue;
                }
                let cursor = after.filter(|(kind, _)| *kind == p).map(|(_, id)| id);
                rows.extend(
                    self.resource_page(p, cursor, limit - rows.len() as i64)
                        .await?,
                );
                if rows.len() == limit as usize {
                    break;
                }
            }
            Ok(rows)
        })
    }
    fn one(
        &self,
        p: SchedulePurpose,
        owner: Uuid,
    ) -> BoxFuture<'_, Result<Option<ScheduleDefinition>, InstanceError>> {
        Box::pin(async move {
            let base = projection(p);
            let query = format!("SELECT * FROM ({base}) AS schedule{COLUMNS} WHERE ownerid=$1");
            sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .bind(owner)
                .fetch_optional(&self.0)
                .await
                .map_err(storage)?
                .map(|row| definition(p, row))
                .transpose()
        })
    }
    fn batch_deadline(
        &self,
        purpose: SchedulePurpose,
    ) -> BoxFuture<'_, Result<Option<DateTime<Utc>>, InstanceError>> {
        Box::pin(async move {
            let query = match purpose {
                SchedulePurpose::Git => {
                    "SELECT greatest(due,now()) FROM git_schedule_branches ORDER BY due LIMIT 1"
                }
                SchedulePurpose::Image => {
                    "SELECT greatest(CASE WHEN i.checkregistrygeneration<g.generation THEN '-infinity'::timestamptz ELSE i.nextupdatecheck END,now()) FROM images i JOIN platforms p ON p.id=i.platformid CROSS JOIN imageupdateregistrygeneration g WHERE i.controlstate='Idle' AND json_array_length(i.tags)>0 AND p.status='Online' AND p.platformdescriptor->>'$type' IN ('Docker','DockerStandalone') ORDER BY i.checkregistrygeneration,i.nextupdatecheck LIMIT 1"
                }
                _ => {
                    return Err(InstanceError::Validation(
                        "Expected a batch schedule".into(),
                    ));
                }
            };
            sqlx::query_scalar(query)
                .fetch_optional(&self.0)
                .await
                .map_err(storage)
        })
    }
}

mod upcoming;
use chrono::{DateTime, Utc};
use citadel_instance::{
    InstanceError, ScheduleCatalogPolicy, ScheduleCount, ScheduleDefinition, SchedulePurpose,
    ScheduleReader, ScheduledRun,
};
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
                          scheduletimezone, NULL::bigint, lastscheduledrunat, NULL::text, NULL::text,
                          CASE WHEN enabled THEN 'Enabled' ELSE 'Disabled' END AS resourcestatus, timeoutseconds,
                          CASE WHEN currentrunid IS NOT NULL THEN
                            (SELECT status FROM scriptruns WHERE id=scripts.currentrunid AND trigger='Schedule'
                             AND status IN ('Queued','Running')) END AS activerunstatus
                   FROM scripts WHERE schedulecron IS NOT NULL",
        Backup => "SELECT p.id, p.name, NULL::uuid, p.enabled, p.cron, COALESCE(p.timezone,'UTC'),
                          NULL::bigint, p.lastscheduledrunat, NULL::text,
                          CASE WHEN r.archivedat IS NOT NULL THEN 'The backup repository is archived.' END,
                          CASE WHEN p.enabled THEN 'Enabled' ELSE 'Disabled' END AS resourcestatus,
                          CASE WHEN p.currentrunid IS NOT NULL THEN
                            (SELECT status FROM backupruns WHERE id=p.currentrunid AND trigger='Schedule'
                             AND status IN ('Queued','Preparing','Running','ApplyingRetention','Processing')) END AS activerunstatus
                   FROM backuppolicies p JOIN backuprepositories r ON r.id=p.backuprepositoryid
                   WHERE p.archivedat IS NULL AND p.cron IS NOT NULL",
        Install => "SELECT singleton.id, 'Core and Agent installation'::text, NULL::uuid,
                           COALESCE((a.schedule->>'enabled')::boolean,false),
                           COALESCE(a.schedule->>'cron','0 3 * * *'), COALESCE(a.schedule->>'timeZone','UTC'),
                           NULL::bigint, a.lastscheduledminute, NULL::text, NULL::text, NULL::text AS resourcestatus
                    FROM (VALUES ('00000000-0000-0000-0000-000000000000'::uuid)) singleton(id)
                    LEFT JOIN agentupdatesettings a ON a.id=1",
        Release => "SELECT '00000000-0000-0000-0000-000000000000'::uuid, 'Core release checks'::text, NULL::uuid,
                           COALESCE(s.initializedat IS NOT NULL AND s.updatechecksenabled,false),
                           NULL::text, NULL::text, COALESCE(s.updatecheckintervalhours,6)::bigint*3600,
                           c.lastattempt, NULL::text, NULL::text, NULL::text AS resourcestatus
                    FROM (VALUES (1)) singleton(id)
                    LEFT JOIN instancesetupstates s ON s.id=singleton.id
                    LEFT JOIN instanceupdatechecks c ON c.id=singleton.id",
        Git => "SELECT r.id, r.name, NULL::uuid, r.syncmode='PullInterval', NULL::text, NULL::text,
                       COALESCE(r.syncintervalminutes,5)::bigint*60,
                       (SELECT greatest(min(due),now()) FROM git_schedule_branches b
                        WHERE b.id=r.id HAVING count(*)>0), NULL::text,
                       CASE WHEN r.controlstate<>'Idle' THEN 'Repository synchronization is busy.' END, r.status AS resourcestatus
                FROM gitrepositories r",
        DeploymentImages => "SELECT d.id, d.name, d.platformid, COALESCE(d.spec->>'UpdateBehavior' IN ('Notify','AutoDeploy'),false),
                                    NULL::text, NULL::text, NULL::bigint, NULL::timestamptz, NULL::text,
                                    CASE WHEN p.status='Offline' THEN 'Platform is offline.'
                                 WHEN p.status IS DISTINCT FROM 'Online' THEN 'Platform availability is unknown.'
                                         WHEN p.platformdescriptor->>'$type' IS DISTINCT FROM 'Docker' THEN 'Requires a standalone Docker platform.'
                                         WHEN d.spec->'Image'->>'$type' IS DISTINCT FROM 'External'
                                           OR COALESCE(d.spec->'Image'->>'ResolvedDigest','')=''
                                           OR position('@' in COALESCE(d.spec->'Image'->>'ImageTag',''))>0
                                         THEN 'Requires an applied external image with a tracked tag.' END, d.status AS resourcestatus, p.status IN ('Online','Offline') AS actionable
                             FROM deployments d LEFT JOIN platforms p ON p.id=d.platformid",
        ServiceImages => "SELECT s.id, s.name, s.platformid, COALESCE(s.spec->>'UpdateBehavior' IN ('Notify','AutoDeploy'),false),
                                 NULL::text, NULL::text, NULL::bigint, NULL::timestamptz, NULL::text,
                                 CASE WHEN p.status='Offline' THEN 'Platform is offline.'
                                 WHEN p.status IS DISTINCT FROM 'Online' THEN 'Platform availability is unknown.'
                                      WHEN p.platformdescriptor->>'$type' IS DISTINCT FROM 'DockerSwarm' THEN 'Requires a Docker Swarm platform.'
                                      WHEN s.spec->'Image'->>'$type' IS DISTINCT FROM 'External'
                                        OR COALESCE(s.appliedimagedigest,'')='' OR s.spec->'Image'->>'ImageTag' IS NULL
                                        OR position('@' in COALESCE(s.spec->'Image'->>'ImageTag',''))>0
                                      THEN 'Requires an applied external image with a tracked tag.' END, s.health AS resourcestatus, p.status IN ('Online','Offline') AS actionable
                          FROM swarmservices s LEFT JOIN platforms p ON p.id=s.platformid",
        StackImages => "SELECT s.id, s.name, r.platformid, COALESCE(r.spec->>'UpdateBehavior' IN ('Notify','StackAutoDeploy','ServiceAutoDeploy'),false),
                               NULL::text, NULL::text, NULL::bigint, NULL::timestamptz, NULL::text,
                               CASE WHEN p.status='Offline' THEN 'Platform is offline.'
                                 WHEN p.status IS DISTINCT FROM 'Online' THEN 'Platform availability is unknown.'
                                    WHEN r.status NOT IN ('Healthy','Degraded') THEN 'Waiting for an applied release.'
                                    WHEN COALESCE(r.spec->>'RegistryId','') IN ('','00000000-0000-0000-0000-000000000000')
                                    THEN 'A registry is required for image checks.' END, COALESCE(r.status,'Unknown') AS resourcestatus,
                               (p.status='Offline' OR (p.status='Online' AND r.status IN ('Healthy','Degraded'))) AS actionable
                        FROM stacks s LEFT JOIN stackreleases r ON r.id=s.currentstackreleaseid
                        LEFT JOIN platforms p ON p.id=r.platformid WHERE s.stacksource IN ('WebEditor','Git')",
        StackGit => "SELECT s.id, s.name, r.platformid, COALESCE(r.spec->>'UpdateBehavior' IN ('Notify','StackAutoDeploy','ServiceAutoDeploy'),false),
                            NULL::text, NULL::text, NULL::bigint, NULL::timestamptz, NULL::text,
                            CASE WHEN COALESCE(r.spec->>'CommitSha','')<>'' THEN 'The stack is pinned to a commit.'
                                 WHEN p.status='Offline' THEN 'Platform is offline.'
                                 WHEN p.status IS DISTINCT FROM 'Online' THEN 'Platform availability is unknown.'
                                 WHEN r.status NOT IN ('Healthy','Degraded') THEN 'Waiting for an applied release.' END, COALESCE(r.status,'Unknown') AS resourcestatus,
                            (COALESCE(r.spec->>'CommitSha','')='' AND p.status='Offline') AS actionable
                     FROM stacks s LEFT JOIN stackreleases r ON r.id=s.currentstackreleaseid
                     LEFT JOIN platforms p ON p.id=r.platformid WHERE s.stacksource='Git'",
        Image => "SELECT i.id, i.name, i.platformid, true, NULL::text, NULL::text, NULL::bigint,
                         (SELECT greatest(d.due,now()) FROM inventory_image_deadlines d WHERE d.id=i.id), i.dockerimageid,
                         CASE WHEN p.status='Offline' THEN 'Platform is offline.'
                              WHEN p.status='Online' AND p.platformdescriptor->>'$type' NOT IN ('Docker','DockerStandalone')
                              THEN 'Requires a standalone Docker platform.' END,
                         CASE WHEN i.containers > 0 THEN 'In use' ELSE 'Unused' END AS resourcestatus
                  FROM images i JOIN platforms p ON p.id=i.platformid WHERE json_array_length(i.tags)>0",
    }
}
fn definition(purpose: SchedulePurpose, row: PgRow) -> Result<ScheduleDefinition, InstanceError> {
    Ok(ScheduleDefinition {
        purpose,
        resource_status: row.try_get("resourcestatus").map_err(storage)?,
        needs_attention: row.try_get("needsattention").map_err(storage)?,
        latest_run: match row
            .try_get::<Option<Uuid>, _>("latestrunid")
            .map_err(storage)?
        {
            Some(id) => Some(ScheduledRun {
                queued_at: row.try_get("lastqueued").map_err(storage)?,
                id,
                status: row.try_get("latestrunstatus").map_err(storage)?,
            }),
            None => None,
        },
        active_run_status: match purpose {
            SchedulePurpose::Script | SchedulePurpose::Backup => {
                row.try_get("activerunstatus").map_err(storage)?
            }
            _ => None,
        },
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
// Both the group count and the filtered page use this predicate.
fn attention(p: SchedulePurpose, policy: &ScheduleCatalogPolicy) -> String {
    use SchedulePurpose::*;
    if matches!(
        policy.blockers.get(&p),
        Some(citadel_instance::SchedulePolicyBlocker::Blocked(_))
    ) {
        return "enabled".into();
    }
    match p {
        Script => format!(
            "enabled AND (timeoutseconds < 1 OR timeoutseconds > {})",
            policy.max_script_timeout
        ),
        Backup | Image => "enabled AND blocker IS NOT NULL".into(),
        DeploymentImages | ServiceImages | StackImages | StackGit => {
            "enabled AND blocker IS NOT NULL AND actionable IS TRUE".into()
        }
        Git | Release | Install => "false".into(),
    }
}
impl ScheduleReader for PostgresSchedules {
    fn counts<'a>(
        &'a self,
        search: &'a str,
        policy: &'a ScheduleCatalogPolicy,
    ) -> BoxFuture<'a, Result<Vec<ScheduleCount>, InstanceError>> {
        Box::pin(async move {
            // Count the same catalog membership as page(), without decoding definitions or calculating deadlines.
            let filter = if search.is_empty() {
                ""
            } else {
                " WHERE strpos(lower(name),lower($1)) > 0"
            };
            let branches = SchedulePurpose::ALL.map(|purpose| format!(
                "SELECT '{}' AS purpose, count(*) AS total, count(*) FILTER (WHERE {}) AS attention FROM ({}) AS schedule{COLUMNS}{filter} HAVING count(*) > 0",
                purpose.name(), attention(purpose,policy), projection(purpose)
            ));
            let sql = branches.join(" UNION ALL ");
            let mut query =
                sqlx::query_as::<_, (String, i64, i64)>(sqlx::AssertSqlSafe(sql.as_str()));
            if !search.is_empty() {
                query = query.bind(search);
            }
            let rows = query.fetch_all(&self.0).await.map_err(storage)?;
            Ok(rows
                .into_iter()
                .map(|(purpose, total, attention)| ScheduleCount {
                    purpose: SchedulePurpose::parse(&purpose).expect("catalog purpose"),
                    total,
                    attention,
                })
                .collect())
        })
    }
    fn page<'a>(
        &'a self,
        purposes: &'a [SchedulePurpose],
        search: &'a str,
        offset: i64,
        limit: i64,
        policy: &'a ScheduleCatalogPolicy,
    ) -> BoxFuture<'a, Result<Vec<ScheduleDefinition>, InstanceError>> {
        Box::pin(async move {
            if purposes.is_empty() {
                return Ok(Vec::new());
            }
            let name_filter = if search.is_empty() {
                ""
            } else {
                " AND strpos(lower(name),lower($3)) > 0"
            };
            let mut branches = Vec::new();
            let mut runs = Vec::new();
            for &p in purposes {
                let blocker = attention(p, policy);
                let filter = if policy.attention_only {
                    blocker.as_str()
                } else {
                    "true"
                };
                let timeout = if p == SchedulePurpose::Script {
                    "timeoutseconds"
                } else {
                    "NULL::integer"
                };
                let status = if matches!(p, SchedulePurpose::Script | SchedulePurpose::Backup) {
                    "activerunstatus"
                } else {
                    "NULL::text"
                };
                let base = projection(p);
                let purpose = p.name();
                let purpose_order = p as i32;
                branches.push(format!("SELECT {purpose_order} AS purposeorder, '{purpose}'::text AS purpose, ownerid,name,platformid,enabled,cron,zone,seconds,anchor,dockerimageid,blocker,resourcestatus,
                    ({blocker}) AS needsattention,{timeout} AS timeoutseconds,{status} AS activerunstatus
                    FROM ({base}) AS schedule{COLUMNS} WHERE ({filter}){name_filter}"));
                let source = match p {
                    SchedulePurpose::Script => Some(("scriptruns", "scriptid")),
                    SchedulePurpose::Backup => Some(("backupruns", "backuppolicyid")),
                    _ => None,
                };
                if let Some((table, owner)) = source {
                    runs.push(format!("(SELECT id,status,queuedat FROM {table} WHERE page.purpose='{purpose}' AND {owner}=page.ownerid AND trigger='Schedule'
                        ORDER BY queuedat DESC,id DESC LIMIT 1)"));
                }
            }
            let pages = branches.join(" UNION ALL ");
            let page = format!(
                "SELECT * FROM ({pages}) schedules ORDER BY purposeorder,ownerid LIMIT $1 OFFSET $2"
            );
            // Fetch outcomes only for the visible page. Worker discovery never reads run history.
            let sql = if runs.is_empty() {
                format!(
                    "SELECT page.*, NULL::uuid AS latestrunid, NULL::text AS latestrunstatus FROM ({page}) page ORDER BY purposeorder,ownerid"
                )
            } else {
                let runs = runs.join(" UNION ALL ");
                format!("SELECT page.*, run.id AS latestrunid, run.status AS latestrunstatus, run.queuedat AS lastqueued
                    FROM ({page}) page LEFT JOIN LATERAL ({runs}) run ON true ORDER BY purposeorder,ownerid")
            };
            let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                .bind(limit.clamp(1, 200))
                .bind(offset.max(0));
            if !search.is_empty() {
                query = query.bind(search);
            }
            query
                .fetch_all(&self.0)
                .await
                .map_err(storage)?
                .into_iter()
                .map(|row| {
                    let purpose = SchedulePurpose::parse(row.try_get("purpose").map_err(storage)?)
                        .expect("catalog purpose");
                    definition(purpose, row)
                })
                .collect()
        })
    }
    fn upcoming<'a>(
        &'a self,
        tags: &'a [String],
        limit: usize,
        now: DateTime<Utc>,
        fixed: std::collections::BTreeMap<SchedulePurpose, DateTime<Utc>>,
        policy: &'a ScheduleCatalogPolicy,
    ) -> BoxFuture<'a, Result<Vec<ScheduleDefinition>, InstanceError>> {
        Box::pin(upcoming::read(self, tags, limit, now, fixed, policy))
    }
    fn discover(
        &self,
        p: SchedulePurpose,
        after: Option<Uuid>,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<ScheduleDefinition>, InstanceError>> {
        Box::pin(async move {
            let base = projection(p);
            let (op, lower) = after.map_or((">=", Uuid::nil()), |id| (">", id));
            let sql = format!("SELECT *, false AS needsattention, NULL::uuid AS latestrunid, NULL::text AS latestrunstatus
                FROM ({base}) AS schedule{COLUMNS} WHERE ownerid {op} $1 ORDER BY ownerid LIMIT $2");
            sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                .bind(lower)
                .bind(limit.clamp(1, 129))
                .fetch_all(&self.0)
                .await
                .map_err(storage)?
                .into_iter()
                .map(|row| definition(p, row))
                .collect()
        })
    }
    fn one(
        &self,
        p: SchedulePurpose,
        owner: Uuid,
    ) -> BoxFuture<'_, Result<Option<ScheduleDefinition>, InstanceError>> {
        Box::pin(async move {
            let base = projection(p);
            let query = format!(
                "SELECT *, false AS needsattention, NULL::uuid AS latestrunid, NULL::text AS latestrunstatus FROM ({base}) AS schedule{COLUMNS} WHERE ownerid=$1"
            );
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

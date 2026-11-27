use super::{COLUMNS, PostgresSchedules, definition, projection, storage};
use chrono::{DateTime, Utc};
use citadel_instance::{
    InstanceError, ScheduleCatalogPolicy, ScheduleDefinition, ScheduleMode, SchedulePurpose,
};
use futures_util::TryStreamExt;
use sqlx::Row;
use std::collections::BTreeMap;

pub(super) async fn read(
    store: &PostgresSchedules,
    tags: &[String],
    limit: usize,
    now: DateTime<Utc>,
    fixed: BTreeMap<SchedulePurpose, DateTime<Utc>>,
    policy: &ScheduleCatalogPolicy,
) -> Result<Vec<ScheduleDefinition>, InstanceError> {
    let mut branches = Vec::new();
    let mut deadlines = serde_json::Map::new();
    for p in SchedulePurpose::ALL {
        if policy.blockers.contains_key(&p) {
            continue;
        }
        let kind = match p {
            SchedulePurpose::Script => "Script",
            SchedulePurpose::Backup => "BackupPolicy",
            SchedulePurpose::Git => "GitRepository",
            SchedulePurpose::DeploymentImages => "Deployment",
            SchedulePurpose::ServiceImages => "SwarmService",
            SchedulePurpose::StackImages | SchedulePurpose::StackGit => "Stack",
            _ => "",
        };
        if !tags.is_empty() && kind.is_empty() {
            continue;
        }
        let owner = if tags.is_empty() {
            String::new()
        } else {
            format!(
                " AND NOT EXISTS(SELECT FROM unnest($2::text[]) req(name) WHERE NOT EXISTS(SELECT FROM resourcetags link JOIN tags t ON t.id=link.tagid WHERE link.resourceid=ownerid AND link.resourcetype='{kind}' AND t.normalizedname=lower(req.name)))"
            )
        };
        let eligibility = if p == SchedulePurpose::Script {
            format!(
                " AND timeoutseconds BETWEEN 1 AND {}",
                policy.max_script_timeout
            )
        } else {
            String::new()
        };
        let name = p.name();
        let base = projection(p);
        let timeout = if p == SchedulePurpose::Script {
            "timeoutseconds"
        } else {
            "NULL::integer"
        };
        let fields = format!(
            "'{name}'::text AS purpose,ownerid,name,platformid,enabled,cron,zone,seconds,dockerimageid,blocker,resourcestatus,{timeout} AS timeoutseconds"
        );
        let table =
            format!("({base}) s{COLUMNS} WHERE enabled AND blocker IS NULL {eligibility}{owner}");
        let sql = match p.mode() {
            ScheduleMode::Calendar => {
                // Identical expressions, zones and effective last occurrences have the
                // same next occurrence. Only their first K owners can enter the result.
                format!(
                    "SELECT {fields},anchor FROM (SELECT s.*,row_number() OVER (PARTITION BY cron,zone,CASE WHEN anchor>=date_trunc('minute',$1::timestamptz) THEN anchor END ORDER BY ownerid) AS rank FROM {table}) ranked WHERE rank<=$3"
                )
            }
            mode => {
                let due = match mode {
                    ScheduleMode::Fixed { .. } => {
                        let Some(at) = fixed.get(&p) else { continue };
                        deadlines.insert(name.into(), serde_json::Value::String(at.to_rfc3339()));
                        format!("($4::jsonb->>'{name}')::timestamptz")
                    }
                    _ if p == SchedulePurpose::Release => {
                        "COALESCE(anchor + seconds * interval '1 second',$1::timestamptz)".into()
                    }
                    _ => "anchor".into(),
                };
                format!(
                    "SELECT {fields},anchor FROM (SELECT s.*,{due} AS due FROM {table}) deadline WHERE due IS NOT NULL ORDER BY due,ownerid LIMIT $3"
                )
            }
        };
        branches.push(format!("({sql})"));
    }
    if branches.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "WITH args AS (SELECT $1::timestamptz,$2::text[],$3::bigint,$4::jsonb) SELECT candidates.*,false AS needsattention,NULL::uuid AS latestrunid,NULL::text AS activerunstatus FROM ({}) candidates",
        branches.join(" UNION ALL ")
    );
    let sql = sqlx::AssertSqlSafe(sql.as_str());
    let mut rows = sqlx::query(sql)
        .bind(now)
        .bind(tags)
        .bind(limit as i64)
        .bind(serde_json::Value::Object(deadlines))
        .fetch(&store.0);
    let mut best = Vec::with_capacity(limit + 1);
    while let Some(row) = rows.try_next().await.map_err(storage)? {
        let p = SchedulePurpose::parse(row.try_get("purpose").map_err(storage)?)
            .expect("known purpose");
        let mut item = definition(p, row)?;
        if p.fixed_interval().is_some() {
            item.anchor = fixed.get(&p).copied()
        }
        if let Some(at) = item.next_time(now) {
            let key = (at, p, item.owner);
            let index = best.partition_point(|(other, _)| other < &key);
            if index < limit {
                best.insert(index, (key, item));
                best.truncate(limit);
            }
        }
    }
    Ok(best.into_iter().map(|(_, v)| v).collect())
}

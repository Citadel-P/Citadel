//! Bounded history compaction and daily expiry, owned by Core's maintenance lease.
use sqlx::PgPool;

pub(super) async fn transaction(
    pool: &PgPool,
) -> Result<sqlx::Transaction<'static, sqlx::Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::raw_sql("SET LOCAL lock_timeout='500ms'; SET LOCAL statement_timeout='5s'")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

/// Keep recent captures at the operator's configured polling resolution. Older
/// minutes retain weighted gauges, extrema and reset-aware physical counters.
pub(crate) async fn compact(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let now = chrono::Utc::now();
    let cutoff = now.timestamp().saturating_sub(86_400) / 60 * 60;
    let retained_from = now.timestamp() - 604_800 + i64::from(now.timestamp_subsec_nanos() > 0);
    let mut tx = transaction(pool).await?;
    let candidate: Option<(i64, uuid::Uuid)> = sqlx::query_as(
        "SELECT src.id,src.platformid FROM statisticssources src
         CROSS JOIN LATERAL (SELECT created FROM statisticssamples WHERE sourceid=src.id
             AND created>=GREATEST(src.compacteduntil,$2) AND created<$1 ORDER BY created LIMIT 1) first
         CROSS JOIN LATERAL (SELECT created FROM statisticssamples WHERE sourceid=src.id ORDER BY created DESC LIMIT 1) last
         WHERE first.created/60<last.created/60 ORDER BY first.created,src.id LIMIT 1",
    ).bind(cutoff).bind(retained_from).fetch_optional(&mut *tx).await?;
    let Some((source, platform)) = candidate else {
        tx.commit().await?;
        return Ok(0);
    };
    // Acquire before rereading the range: a waiting capture/compactor must not use
    // the snapshot from before it acquired the platform's serialization lock.
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .fetch_optional(&mut *tx)
        .await?;
    let bounds: Option<(i64, i64)> = sqlx::query_as(
        "SELECT first.created,LEAST($2, (first.created/60+60)*60, last.created/60*60)
         FROM statisticssources src
         CROSS JOIN LATERAL (SELECT created FROM statisticssamples WHERE sourceid=src.id AND created>=GREATEST(src.compacteduntil,$3) AND created<$2 ORDER BY created LIMIT 1) first
         CROSS JOIN LATERAL (SELECT created FROM statisticssamples WHERE sourceid=src.id ORDER BY created DESC LIMIT 1) last
         WHERE src.id=$1 AND first.created/60<last.created/60",
    ).bind(source).bind(cutoff).bind(retained_from).fetch_optional(&mut *tx).await?;
    let mut progress = 0;
    if let Some((from, until)) = bounds {
        let removed: i64 = sqlx::query_scalar(include_str!("compact.sql"))
            .bind(source)
            .bind(from)
            .bind(until)
            .fetch_one(&mut *tx)
            .await?;
        // Even sparse minutes are sealed; don't revisit them on every pass.
        progress = u64::try_from(removed).unwrap_or_default().max(1);
    }
    tx.commit().await?;
    Ok(progress)
}

/// Drop only verified, fully expired daily partitions. Boundary/default partitions
/// use bounded tuple deletion on each physical child (ctid is not globally unique).
pub(crate) async fn retain(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let now = chrono::Utc::now();
    let cutoff = now.timestamp() - 604_800 + i64::from(now.timestamp_subsec_nanos() > 0);
    let day = now.timestamp() / 86_400;
    let mut tx = transaction(pool).await?;
    let mut partition_changed = false;
    for parent in [
        "statisticssamples",
        "statisticsrollups",
        "platformgaugerollups",
        "platformiorollups",
    ] {
        for upcoming in day..=day + 2 {
            partition_changed |=
                sqlx::query_scalar::<_, bool>("SELECT statistics_partition($1,$2)")
                    .bind(parent)
                    .bind(upcoming)
                    .fetch_one(&mut *tx)
                    .await?;
        }
    }
    let partitions: Vec<(String,String,String)> = sqlx::query_as(
        "SELECT parent.relname::text,child.relname::text,pg_get_expr(child.relpartbound,child.oid)
         FROM pg_inherits i JOIN pg_class parent ON parent.oid=i.inhparent
         JOIN pg_class child ON child.oid=i.inhrelid
         WHERE parent.relnamespace='public'::regnamespace AND child.relnamespace='public'::regnamespace
         AND parent.relname IN ('statisticssamples','statisticsrollups','platformgaugerollups','platformiorollups') ORDER BY child.relname"
    ).fetch_all(&mut *tx).await?;
    let mut progress = 0;
    let mut partial = Vec::new();
    for (parent, child, bound) in partitions {
        let default = child == format!("{parent}_default") && bound == "DEFAULT";
        let number = child
            .strip_prefix(&format!("{parent}_d"))
            .and_then(|s| s.parse::<i64>().ok());
        let daily = number
            .filter(|n| *n >= 0 && *n < i64::MAX / 86_400)
            .filter(|n| {
                bound
                    == format!(
                        "FOR VALUES FROM ('{}') TO ('{}')",
                        n * 86_400,
                        (n + 1) * 86_400
                    )
            });
        // Identifiers are used only after matching our literal names/numeric suffix
        // and catalog bounds. Never execute arbitrary catalog text as SQL.
        if let Some(n) = daily {
            if (n + 1) * 86_400 <= cutoff {
                sqlx::query(sqlx::AssertSqlSafe(format!("DROP TABLE public.{child}")))
                    .execute(&mut *tx)
                    .await?;
                progress += 1;
                partition_changed = true;
                continue;
            }
            if parent == "statisticssamples" && (n + 1) * 86_400 <= now.timestamp() - 86_400 {
                let empty: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "SELECT NOT EXISTS(SELECT 1 FROM public.{child})"
                )))
                .fetch_one(&mut *tx)
                .await?;
                if empty {
                    // Vacuum can leave empty indexes allocated. Release an empty
                    // compacted day too, but recheck under locks so a late capture
                    // cannot be lost between the emptiness check and DROP.
                    sqlx::query(sqlx::AssertSqlSafe(format!(
                        "LOCK TABLE ONLY public.statisticssamples, public.{child} IN ACCESS EXCLUSIVE MODE")))
                        .execute(&mut *tx).await?;
                    let still_empty: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                        "SELECT NOT EXISTS(SELECT 1 FROM public.{child})"
                    )))
                    .fetch_one(&mut *tx)
                    .await?;
                    if still_empty {
                        sqlx::query(sqlx::AssertSqlSafe(format!("DROP TABLE public.{child}")))
                            .execute(&mut *tx)
                            .await?;
                        progress += 1;
                        partition_changed = true;
                        continue;
                    }
                }
            }
            if n * 86_400 >= cutoff {
                continue;
            }
        } else if !default {
            continue;
        }
        partial.push(child);
    }
    // Release parent ACCESS EXCLUSIVE locks before row cleanup and analysis.
    // Each completed expiry is independently safe if shutdown cancels later work.
    tx.commit().await?;
    let mut tx = transaction(pool).await?;
    for child in partial {
        progress += sqlx::query(sqlx::AssertSqlSafe(format!(
            "WITH expired AS MATERIALIZED (SELECT ctid FROM public.{child} WHERE created<$1 ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)
             DELETE FROM public.{child} s WHERE s.ctid=ANY(ARRAY(SELECT ctid FROM expired))"
        ))).bind(cutoff).execute(&mut *tx).await?.rows_affected();
    }
    if partition_changed {
        // Autovacuum analyzes children, not their partitioned parents. Daily
        // rotation refreshes parent cardinalities without analyzing every pass.
        sqlx::raw_sql("ANALYZE statisticssamples; ANALYZE statisticsrollups; ANALYZE platformgaugerollups; ANALYZE platformiorollups")
            .execute(&mut *tx)
            .await?;
    }
    // Ownership/source identities are small but must not accumulate after churn.
    progress += sqlx::query("DELETE FROM statisticsowners WHERE id IN (
        SELECT o.id FROM statisticsowners o WHERE NOT EXISTS(SELECT 1 FROM statisticssamples s WHERE s.ownerid=o.id)
        AND NOT EXISTS(SELECT 1 FROM statisticsrollups s WHERE s.ownerid=o.id)
        AND NOT EXISTS(SELECT 1 FROM containers c JOIN statisticssources src ON src.containerid=c.id WHERE src.id=o.sourceid)
        AND NOT EXISTS(SELECT 1 FROM statisticssources src JOIN swarmtaskprojections t ON t.platformid=src.platformid AND t.dockertaskid=src.dockertaskid WHERE src.id=o.sourceid)
        ORDER BY o.id LIMIT 500 FOR UPDATE OF o SKIP LOCKED)")
        .execute(&mut *tx).await?.rows_affected();
    progress += sqlx::query("DELETE FROM statisticssources WHERE id IN (
        SELECT src.id FROM statisticssources src WHERE NOT EXISTS(SELECT 1 FROM statisticssamples s WHERE s.sourceid=src.id)
        AND NOT EXISTS(SELECT 1 FROM statisticsrollups s WHERE s.sourceid=src.id)
        AND NOT EXISTS(SELECT 1 FROM containers c WHERE c.id=src.containerid)
        AND NOT EXISTS(SELECT 1 FROM swarmtaskprojections t WHERE t.platformid=src.platformid AND t.dockertaskid=src.dockertaskid)
        ORDER BY src.id LIMIT 500 FOR UPDATE OF src SKIP LOCKED)")
        .execute(&mut *tx).await?.rows_affected();
    tx.commit().await?;
    Ok(progress)
}

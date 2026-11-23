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

/// Small scheduling hint only: bounds are reread under the Platform lock. A
/// cancelled/erroring operation is revisited on the next source scan.
#[derive(Default)]
pub(crate) struct Discovery {
    after: i64,
    candidates: std::collections::VecDeque<(i64, uuid::Uuid)>,
    end: bool,
    progressed: bool,
}
#[derive(Clone, Copy)]
pub(crate) enum HistoryFamily {
    Container,
    Platform,
    Coarse,
}

impl Discovery {
    pub(super) async fn next(
        &mut self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        family: HistoryFamily,
        cutoff: i64,
        retained_from: i64,
    ) -> Result<(Option<(i64, uuid::Uuid)>, bool), sqlx::Error> {
        if self.candidates.is_empty() {
            let page: Vec<(i64, uuid::Uuid, bool)> = match family {
                HistoryFamily::Container => {
                    sqlx::query_as(include_str!("compaction_candidates.sql"))
                        .bind(cutoff)
                        .bind(retained_from)
                        .bind(self.after)
                        .fetch_all(&mut **tx)
                        .await?
                }
                HistoryFamily::Platform => {
                    sqlx::query_as(concat!(
                        "SELECT id,platformid,COALESCE(from_time<until,false) FROM (",
                        include_str!("platform_compaction_bounds.sql"),
                        ") bounds ORDER BY id"
                    ))
                    .bind(cutoff)
                    .bind(retained_from)
                    .bind(None::<i64>)
                    .bind(self.after)
                    .fetch_all(&mut **tx)
                    .await?
                }
                HistoryFamily::Coarse => {
                    sqlx::query_as(concat!(
                        "SELECT id,platformid,from_time IS NOT NULL FROM (",
                        include_str!("platform_coarse_bounds.sql"),
                        ") bounds ORDER BY id"
                    ))
                    .bind(retained_from)
                    .bind(None::<i64>)
                    .bind(self.after)
                    .fetch_all(&mut **tx)
                    .await?
                }
            };
            self.end = page.len() < 64;
            self.after = page.last().map_or(0, |row| row.0);
            self.candidates.extend(
                page.into_iter()
                    .filter(|row| row.2)
                    .map(|row| (row.0, row.1)),
            );
        }
        let candidate = self.candidates.pop_front();
        let pending = !self.candidates.is_empty() || !self.end;
        if !pending {
            self.after = 0;
        }
        Ok((candidate, pending))
    }

    pub(super) fn committed(&mut self, progress: u64, pending: bool) -> bool {
        self.progressed |= progress > 0;
        // A productive scan may have more source-hours. Carry this hint across
        // worker budgets and empty trailing pages, then require an idle sweep.
        if pending {
            true
        } else {
            std::mem::take(&mut self.progressed)
        }
    }
}

/// Explicit callers retain the one-source-hour contract, including empty pages.
pub(crate) async fn compact(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let discovery = tokio::sync::Mutex::new(Discovery::default());
    loop {
        let (processed, pending) = compact_page(pool, &discovery).await?;
        if processed > 0 || !pending {
            return Ok(processed);
        }
    }
}

/// Keep recent captures at the operator's configured polling resolution. Older
/// minutes retain weighted gauges, extrema and reset-aware physical counters.
pub(crate) async fn compact_page(
    pool: &PgPool,
    discovery: &tokio::sync::Mutex<Discovery>,
) -> Result<(u64, bool), sqlx::Error> {
    let now = chrono::Utc::now();
    let cutoff = now.timestamp().saturating_sub(86_400) / 60 * 60;
    let retained_from = now.timestamp() - 604_800 + i64::from(now.timestamp_subsec_nanos() > 0);
    // One lease-owned scanner: keep its committed progress and cursor together,
    // including when cancellation drops an unfinished transaction.
    let mut discovery = discovery.lock().await;
    let mut tx = transaction(pool).await?;
    let (candidate, pending) = discovery
        .next(&mut tx, HistoryFamily::Container, cutoff, retained_from)
        .await?;
    let Some((source, platform)) = candidate else {
        tx.commit().await?;
        return Ok((0, discovery.committed(0, pending)));
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
         CROSS JOIN LATERAL (SELECT created FROM statisticssamples WHERE sourceid>=src.id AND sourceid<=src.id AND created>=GREATEST(src.compacteduntil,$3) AND created<$2 ORDER BY sourceid,created LIMIT 1) first
         CROSS JOIN LATERAL (SELECT created FROM statisticssamples WHERE sourceid>=src.id AND sourceid<=src.id ORDER BY sourceid DESC,created DESC LIMIT 1) last
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
    Ok((progress, discovery.committed(progress, pending)))
}

/// Drop only verified, fully expired daily partitions. Boundary/default partitions
/// use bounded tuple deletion on each physical child (ctid is not globally unique).
pub(crate) async fn prepare_retention(pool: &PgPool) -> Result<RetentionSweep, sqlx::Error> {
    let now = chrono::Utc::now();
    let cutoff = now.timestamp() - 604_800 + i64::from(now.timestamp_subsec_nanos() > 0);
    let day = now.timestamp() / 86_400;
    let mut progress = 0;
    let mut partial = Vec::new();
    let mut changed_parents = std::collections::BTreeSet::new();
    let mut sparse_days = Vec::new();
    for parent in [
        "statisticssamples",
        "statisticsrollups",
        "platformgaugerollups",
        "platformiorollups",
        "platformiocoarserollups",
    ] {
        // Compaction reads raw history before writing summaries. Never hold a
        // summary parent lock while acquiring another family's partition locks.
        let mut tx = transaction(pool).await?;
        for upcoming in day..=day + 2 {
            if sqlx::query_scalar::<_, bool>("SELECT statistics_partition($1,$2)")
                .bind(parent)
                .bind(upcoming)
                .fetch_one(&mut *tx)
                .await?
            {
                changed_parents.insert(parent.to_owned());
            }
        }
        let partitions: Vec<(String,String,String)> = sqlx::query_as(
        "SELECT parent.relname::text,child.relname::text,pg_get_expr(child.relpartbound,child.oid)
         FROM pg_inherits i JOIN pg_class parent ON parent.oid=i.inhparent
         JOIN pg_class child ON child.oid=i.inhrelid
         WHERE parent.relnamespace='public'::regnamespace AND child.relnamespace='public'::regnamespace
         AND parent.relname=$1 ORDER BY child.relname"
    ).bind(parent).fetch_all(&mut *tx).await?;
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
                    changed_parents.insert(parent);
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
                            changed_parents.insert(parent);
                            continue;
                        }
                    }
                    sparse_days.push(n);
                }
                if n * 86_400 >= cutoff {
                    continue;
                }
            } else if !default {
                continue;
            }
            partial.push(child);
        }
        // Each family's expiry is independently safe if later work is cancelled.
        // Release its parent locks before touching the next family.
        tx.commit().await?;
    }
    for parent in changed_parents {
        let mut tx = transaction(pool).await?;
        // Autovacuum maintains child statistics, but not inherited parent stats.
        // ONLY still samples across partitions without recursively reanalyzing
        // every child. Analyze just the affected, allowlisted parent and commit
        // separately so later cancellation preserves completed analysis.
        sqlx::query(sqlx::AssertSqlSafe(format!("ANALYZE ONLY public.{parent}")))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    progress += super::reclamation::reclaim(pool, &sparse_days).await?;
    Ok(RetentionSweep {
        cutoff,
        partial,
        owners: true,
        sources: true,
        progress,
    })
}

pub(crate) struct RetentionSweep {
    cutoff: i64,
    partial: Vec<String>,
    owners: bool,
    sources: bool,
    progress: u64,
}

impl RetentionSweep {
    pub(crate) async fn batch(&mut self, pool: &PgPool) -> Result<u64, sqlx::Error> {
        if self.partial.is_empty() && !self.owners && !self.sources {
            return Ok(std::mem::take(&mut self.progress));
        }
        let mut tx = transaction(pool).await?;
        let mut progress = self.progress;
        let mut remaining = Vec::new();
        for child in &self.partial {
            let deleted = sqlx::query(sqlx::AssertSqlSafe(format!(
                "WITH expired AS MATERIALIZED (SELECT ctid FROM public.{child} WHERE created<$1 ORDER BY created LIMIT 5000 FOR UPDATE SKIP LOCKED)
                 DELETE FROM public.{child} s WHERE s.ctid=ANY(ARRAY(SELECT ctid FROM expired))"
            ))).bind(self.cutoff).execute(&mut *tx).await?.rows_affected();
            progress += deleted;
            if deleted > 0 {
                remaining.push(child.clone());
            }
        }
        // Ownership/source identities are small but must not accumulate after churn.
        let owners = if self.owners {
            sqlx::query("DELETE FROM statisticsowners WHERE id IN (
        SELECT o.id FROM statisticsowners o WHERE NOT EXISTS(SELECT 1 FROM statisticssamples s WHERE s.ownerid=o.id)
        AND NOT EXISTS(SELECT 1 FROM statisticsrollups s WHERE s.ownerid=o.id)
        AND NOT EXISTS(SELECT 1 FROM containers c JOIN statisticssources src ON src.containerid=c.id WHERE src.id=o.sourceid)
        AND NOT EXISTS(SELECT 1 FROM statisticssources src JOIN swarmtaskprojections t ON t.platformid=src.platformid AND t.dockertaskid=src.dockertaskid WHERE src.id=o.sourceid)
        ORDER BY o.id LIMIT 500 FOR UPDATE OF o SKIP LOCKED)")
        .execute(&mut *tx).await?.rows_affected()
        } else {
            0
        };
        let sources = if self.sources {
            sqlx::query("DELETE FROM statisticssources WHERE id IN (
        SELECT src.id FROM statisticssources src WHERE NOT EXISTS(SELECT 1 FROM statisticssamples s WHERE s.sourceid=src.id)
        AND NOT EXISTS(SELECT 1 FROM statisticsrollups s WHERE s.sourceid=src.id)
        AND NOT EXISTS(SELECT 1 FROM containers c WHERE c.id=src.containerid)
        AND NOT EXISTS(SELECT 1 FROM swarmtaskprojections t WHERE t.platformid=src.platformid AND t.dockertaskid=src.dockertaskid)
        ORDER BY src.id LIMIT 500 FOR UPDATE OF src SKIP LOCKED)")
        .execute(&mut *tx).await?.rows_affected()
        } else {
            0
        };
        tx.commit().await?;
        self.partial = remaining;
        // Further history deletion may expose additional orphan identities.
        self.owners = owners > 0 || !self.partial.is_empty();
        self.sources = sources > 0 || self.owners;
        self.progress = 0;
        Ok(progress + owners + sources)
    }
}

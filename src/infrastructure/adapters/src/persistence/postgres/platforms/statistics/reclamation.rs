//! Release mostly empty raw-day files without changing retained observations.
use sqlx::PgPool;

const MIN_BYTES: i64 = 8 * 1024 * 1024;
const MAX_BYTES: i64 = 128 * 1024 * 1024;
const MAX_ROWS: i64 = 1024;

/// Names/days come from retention's validated daily partition inventory. Each
/// attempt owns its transaction; no summary-family or Platform lock is held.
/// CLUSTER preserves tuple visibility and indexes, unlike truncate/reinsert.
pub(super) async fn reclaim(pool: &PgPool, days: &[i64]) -> Result<u64, sqlx::Error> {
    for &day in days {
        let child = format!("statisticssamples_d{day}");
        let mut tx = super::maintenance::transaction(pool).await?;
        let locked = async {
            sqlx::query("LOCK TABLE ONLY public.statisticssamples IN ACCESS SHARE MODE NOWAIT")
                .execute(&mut *tx)
                .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "LOCK TABLE ONLY public.{child} IN ACCESS EXCLUSIVE MODE NOWAIT"
            )))
            .execute(&mut *tx)
            .await
        }
        .await;
        if let Err(error) = locked {
            tx.rollback().await?;
            if error
                .as_database_error()
                .is_some_and(|e| e.code().as_deref() == Some("55P03"))
            {
                continue;
            }
            return Err(error);
        }
        // Revalidate after locking: an administrator may have changed the
        // partition since inventory. Quote the catalog's index identifier.
        let index: Option<String> = sqlx::query_scalar(
            "SELECT quote_ident(idx.relname) FROM pg_inherits inh
             JOIN pg_class c ON c.oid=inh.inhrelid
             JOIN pg_index i ON i.indrelid=c.oid AND i.indisprimary AND i.indisvalid
             JOIN pg_class idx ON idx.oid=i.indexrelid
             WHERE inh.inhparent='public.statisticssamples'::regclass
             AND c.oid=to_regclass($1) AND pg_get_expr(c.relpartbound,c.oid)=$2
             AND pg_total_relation_size(c.oid) BETWEEN $3 AND $4",
        )
        .bind(format!("public.{child}"))
        .bind(format!(
            "FOR VALUES FROM ('{}') TO ('{}')",
            day * 86400,
            (day + 1) * 86400
        ))
        .bind(MIN_BYTES)
        .bind(MAX_BYTES)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(index) = index else {
            tx.rollback().await?;
            continue;
        };
        let rows: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM (SELECT 1 FROM public.{child} LIMIT {}) samples",
            MAX_ROWS + 1
        )))
        .fetch_one(&mut *tx)
        .await?;
        if rows == 0 || rows > MAX_ROWS {
            tx.rollback().await?;
            continue;
        }
        // One small physical partition, never the partitioned parent. Its
        // existing primary key orders a rewrite of the heap and all indexes.
        // Statement timeout and transaction rollback also cover cancellation.
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "CLUSTER public.{child} USING {index}"
        )))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(1);
    }
    Ok(0)
}

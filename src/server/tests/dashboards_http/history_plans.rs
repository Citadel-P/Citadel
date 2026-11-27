use super::*;
use citadel_dashboards::{
    AttentionTab, DashboardRepository, DashboardViewer, FleetFilter, FleetRead,
};
use sqlx::{PgConnection, postgres::PgPoolOptions};

/// Exercise the statement actually prepared by the repository, including its ACL and tag CTEs.
pub(super) async fn compare(admin: &ActorPrincipal, viewer: &ActorPrincipal, tag: &str) {
    let url = std::env::var("CITADEL_DASHBOARD_DATABASE_URL").unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let repo = PostgresDashboardRepository::new(pool.clone());
    let mut queries = Vec::new();
    for read in [
        FleetRead::Attention,
        FleetRead::Summary,
        FleetRead::OpenAlerts,
    ] {
        repo.fleet(
            DashboardViewer {
                actor: admin.actor_id,
                administrator: true,
                read_only: false,
            },
            FleetFilter {
                read,
                tags: vec![],
                kinds: vec![],
                state: None,
                search: None,
                tab: AttentionTab::All,
                limit: 5,
                offset: 0,
            },
        )
        .await
        .unwrap();
        let sql:String=sqlx::query_scalar("SELECT statement FROM pg_prepared_statements WHERE statement LIKE 'WITH enabled_actor%' ORDER BY prepare_time DESC LIMIT 1").fetch_one(&pool).await.unwrap();
        queries.push((read, sql));
    }
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(include_str!("../fixtures/dashboard_retained_history.sql"))
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SAVEPOINT with_indexes")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::raw_sql("DROP INDEX ix_backupruns_policy_completed, ix_alertevents_unresolved_resource")
        .execute(&mut *tx)
        .await
        .unwrap();
    let mut baseline = vec![];
    for indexed in [false, true] {
        if indexed {
            sqlx::query("ROLLBACK TO SAVEPOINT with_indexes")
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        let mut index = 0;
        for (read, sql) in &queries {
            for (scope, actor, tags) in [
                ("admin", admin, vec![]),
                ("tag", admin, vec![tag]),
                ("viewer", viewer, vec![]),
            ] {
                let plan = execute(
                    &mut tx,
                    &format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {sql}"),
                    *read,
                    actor,
                    &tags,
                )
                .await;
                let result = execute(&mut tx, sql, *read, actor, &tags).await;
                let root = &plan[0]["Plan"];
                eprintln!(
                    "{read:?} {scope} indexed={indexed} shared_blocks={} execution_ms={}",
                    root["Shared Hit Blocks"].as_i64().unwrap()
                        + root["Shared Read Blocks"].as_i64().unwrap(),
                    plan[0]["Execution Time"]
                );
                if indexed {
                    assert_eq!(result, baseline[index], "{read:?} {scope}");
                    if scope == "admin" {
                        let nodes = plan.to_string();
                        assert!(
                            nodes.contains("ix_alertevents_unresolved_resource"),
                            "{read:?}"
                        );
                        if *read == FleetRead::Attention {
                            assert!(nodes.contains("ix_backupruns_policy_completed"));
                        }
                    }
                } else {
                    baseline.push(result.clone());
                }
                if *read == FleetRead::Attention {
                    assert!(
                        !result["rows"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|r| r["name"] == "payments-daily")
                    );
                    let failed_archive = result["rows"].as_array().unwrap().iter().any(|row| {
                        row["name"] == "Archive" && row["reasons"][0]["code"] == "Backup Failed"
                    });
                    assert_eq!(failed_archive, scope == "admin");
                } else {
                    let count = &result[if *read == FleetRead::Summary {
                        "alerts"
                    } else {
                        "total"
                    }];
                    if scope == "admin" {
                        assert_eq!(count, &json!(3));
                    } else if scope == "viewer" {
                        assert_eq!(count, &json!(0));
                    }
                }
                index += 1;
            }
        }
    }
    tx.rollback().await.unwrap();
}

async fn execute(
    conn: &mut PgConnection,
    sql: &str,
    read: FleetRead,
    actor: &ActorPrincipal,
    tags: &[&str],
) -> Value {
    let mut q = sqlx::query_scalar::<_, Value>(sqlx::AssertSqlSafe(sql))
        .bind(actor.actor_id.value())
        .bind(actor.is_administrator())
        .bind(tags);
    if read == FleetRead::Attention {
        q = q.bind("All");
    }
    if read != FleetRead::Summary {
        q = q.bind(25_i64).bind(0_i64);
    }
    q.fetch_one(conn).await.unwrap()
}

use super::*;
use citadel_builds::{BuildCleanupStore, BuildError};

pub(super) async fn verify(
    pool: &sqlx::PgPool,
    store: &PostgresBuildRepository,
    run: Uuid,
    system: ActorId,
) {
    // A lost admission response still needs an operator resolution path.
    sqlx::query("UPDATE buildexecutorreservations SET admissiongeneration=NULL WHERE runid=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    let reservation = store.retained_reservation(run).await.unwrap().unwrap();
    let before = store.get_run(run).await.unwrap();
    let replacement = Uuid::now_v7();
    assert!(matches!(
        store
            .resolve_reservation(&reservation, system, "checked", replacement)
            .await,
        Err(BuildError::Forbidden)
    ));
    let admin = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(admin.value())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,name) VALUES($1,$2,$3,'Recovery administrator')").bind(Uuid::now_v7()).bind(admin.value()).bind(system.value()).execute(pool).await.unwrap();
    assert!(
        matches!(
            store
                .resolve_reservation(&reservation, admin, "checked", replacement)
                .await,
            Err(BuildError::Forbidden)
        ),
        "a human without admin access must be rejected"
    );
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(admin.value())
        .bind(citadel_identity::ADMIN_ROLE_ID)
        .execute(pool)
        .await
        .unwrap();
    assert!(
        store
            .resolve_reservation(&reservation, admin, "checked", reservation.authority_id)
            .await
            .is_err()
    );
    let mut stale = reservation.clone();
    stale.attempt_id = Uuid::now_v7();
    assert!(
        !store
            .resolve_reservation(&stale, admin, "checked", replacement)
            .await
            .unwrap()
    );
    sqlx::query("UPDATE buildruns SET status='Running' WHERE id=$1")
        .bind(run)
        .execute(pool)
        .await
        .unwrap();
    assert!(store.retained_reservation(run).await.unwrap().is_none());
    assert!(
        !store
            .resolve_reservation(&reservation, admin, "checked", replacement)
            .await
            .unwrap()
    );
    sqlx::query("UPDATE buildruns SET status=$2 WHERE id=$1")
        .bind(run)
        .bind(before.status.to_string())
        .execute(pool)
        .await
        .unwrap();
    for (table, check) in [
        (
            "activityevents",
            "eventtype <> 'BuildRunCleanupResolved'".to_owned(),
        ),
        ("buildrunlogs", format!("buildrunid <> '{run}'")),
    ] {
        let constraint = format!("operator_audit_{}", run.simple());
        let ddl =
            format!("ALTER TABLE {table} ADD CONSTRAINT {constraint} CHECK({check}) NOT VALID");
        sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
            .execute(pool)
            .await
            .unwrap();
        assert!(
            store
                .resolve_reservation(&reservation, admin, "checked", replacement)
                .await
                .is_err()
        );
        assert!(
            store.retained_reservation(run).await.unwrap().is_some(),
            "audit or log failure must retain capacity"
        );
        let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='BuildRunCleanupResolved'").bind(before.build_project_id).fetch_one(pool).await.unwrap();
        assert_eq!(audit, 0, "failed release must not leave a success audit");
        let ddl = format!("ALTER TABLE {table} DROP CONSTRAINT {constraint}");
        sqlx::raw_sql(sqlx::AssertSqlSafe(ddl.as_str()))
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query("UPDATE actors SET isenabled=false WHERE id=$1")
        .bind(admin.value())
        .execute(pool)
        .await
        .unwrap();
    assert!(
        matches!(
            store
                .resolve_reservation(&reservation, admin, "checked", replacement)
                .await,
            Err(BuildError::Forbidden)
        ),
        "a revoked administrator must be rechecked at commit"
    );
    sqlx::query("UPDATE actors SET isenabled=true WHERE id=$1")
        .bind(admin.value())
        .execute(pool)
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        store.resolve_reservation(
            &reservation,
            admin,
            "Verified Docker work stopped and restarted Agent",
            replacement
        ),
        store.resolve_reservation(&reservation, admin, "duplicate request", replacement)
    );
    assert_ne!(
        a.unwrap(),
        b.unwrap(),
        "only one concurrent resolution may commit"
    );
    assert!(store.retained_reservation(run).await.unwrap().is_none());
    let after = store.get_run(run).await.unwrap();
    assert_eq!(before.status, after.status);
    assert_eq!(before.completed_at, after.completed_at);
    assert_eq!(after.error_code.as_deref(), Some("build.cleanup_resolved"));
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM buildexecutorcleanupreceipts WHERE runid=$1")
            .bind(run)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        receipts, 0,
        "operator acknowledgment must never masquerade as an Agent receipt"
    );
    let audit: Vec<(Uuid, String)> = sqlx::query_as("SELECT createdbyactorid,info FROM activityevents WHERE resourceid=$1 AND eventtype='BuildRunCleanupResolved'").bind(before.build_project_id).fetch_all(pool).await.unwrap();
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].0, admin.value());
    let info: serde_json::Value = serde_json::from_str(&audit[0].1).unwrap();
    assert_eq!(info["AttemptId"], reservation.attempt_id.to_string());
    assert_eq!(info["ReplacementAuthorityId"], replacement.to_string());
    assert!(
        store
            .logs(run)
            .await
            .unwrap()
            .iter()
            .any(|log| log.message.contains("Original result:"))
    );
    sqlx::query("DELETE FROM activityevents WHERE createdbyactorid=$1")
        .bind(admin.value())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE actorid=$1")
        .bind(admin.value())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(admin.value())
        .execute(pool)
        .await
        .unwrap();
}

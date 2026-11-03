use super::*;
use serde_json::Value;

pub(super) async fn verify_reconciliation_ownership(
    pool: &sqlx::PgPool,
    repo: &PostgresUpdateRepository,
    journal: &UpdateJournal,
) {
    let id = journal.operation.id;
    let original = snapshot(pool, id).await;
    let mut released = journal.clone();
    released.completion_acknowledged = true;
    released.maintenance_released = true;
    for change in [
        "revision=revision+1",
        "instanceid='00000000-0000-0000-0000-000000000001'",
        "actorid='00000000-0000-0000-0000-000000000001'",
        "idempotencykey='00000000-0000-0000-0000-000000000001'",
        "operation=jsonb_set(operation,'{id}','\"00000000-0000-0000-0000-000000000001\"')",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE instanceupdates SET {change} WHERE id=$1"
        )))
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
        let before = snapshot(pool, id).await;
        let activities = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM activityevents")
            .fetch_one(pool)
            .await
            .unwrap();
        assert!(
            repo.import(journal).await.is_err(),
            "import accepted {change}"
        );
        assert!(
            repo.release(&released, &journal.operation.plan.target)
                .await
                .is_err(),
            "release accepted {change}"
        );
        if let Some(receipt) = original.get("completion").filter(|v| !v.is_null()) {
            let receipt: CompletionReceipt = serde_json::from_value(receipt.clone()).unwrap();
            assert!(
                citadel_adapters::persistence::postgres::updates::check_committed_update_receipt(
                    pool, journal, &receipt,
                )
                .await
                .is_err(),
                "receipt accepted {change}"
            );
        }
        assert_eq!(
            snapshot(pool, id).await,
            before,
            "changed evidence for {change}"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM activityevents")
                .fetch_one(pool)
                .await
                .unwrap(),
            activities
        );
        sqlx::query("UPDATE instanceupdates SET revision=$2,instanceid=$3,actorid=$4,idempotencykey=$5,operation=$6 WHERE id=$1")
            .bind(id).bind(original["revision"].as_i64().unwrap())
            .bind(journal.operation.instance_id).bind(journal.operation.actor_id.value())
            .bind(journal.operation.idempotency_key).bind(&original["operation"])
            .execute(pool).await.unwrap();
    }
    if original["completion"].is_null() {
        sqlx::query("UPDATE instanceupdates SET ownsinstallation=FALSE WHERE id=$1")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
        let before = snapshot(pool, id).await;
        assert!(
            repo.import(journal).await.is_err(),
            "released claim accepted advancement"
        );
        assert_eq!(snapshot(pool, id).await, before);
        sqlx::query("UPDATE instanceupdates SET ownsinstallation=TRUE WHERE id=$1")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn snapshot(pool: &sqlx::PgPool, id: Uuid) -> Value {
    sqlx::query_scalar("SELECT to_jsonb(u) FROM instanceupdates u WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

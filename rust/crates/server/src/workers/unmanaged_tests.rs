use super::*;
use citadel_alerts::{AlertError, AlertEventView};
use futures_util::future::BoxFuture;
#[derive(Default)]
struct Alerts(std::sync::Mutex<Vec<AlertObservation>>);
impl AlertEventSink for Alerts {
    fn observe<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEventView>, AlertError>> {
        Box::pin(async move {
            self.0.lock().unwrap().push(observation.clone());
            Ok(None)
        })
    }
}
#[test]
fn duplicate_creation_extends_grace_and_platforms_remain_independent() {
    let now = Instant::now();
    let mut pending = HashMap::new();
    let mut request = Request {
        platform: Uuid::now_v7(),
        container: " ABC ".into(),
        node: None,
    };
    schedule(&mut pending, &mut request, now);
    request.container = "abc".into();
    schedule(&mut pending, &mut request, now + Duration::from_secs(20));
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[&request], now + Duration::from_secs(50));
    request.platform = Uuid::now_v7();
    schedule(&mut pending, &mut request, now);
    assert_eq!(pending.len(), 2);
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn grace_recheck_suppresses_adopted_system_swarm_and_deleted_containers() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$2,$2,'Local',0,0,0,0,0,'{\"$type\":\"Docker\"}','Online')")
        .bind(platform).bind(platform.to_string()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,state,created,updated,ports,issystem) VALUES($1,$2,'abc','image','unmanaged','Running',0,0,'{}',false)")
        .bind(id).bind(platform).execute(&pool).await.unwrap();
    let request = Request {
        platform,
        container: "abc".into(),
        node: None,
    };
    let alerts = Alerts::default();
    observe(&pool, &alerts, &request).await.unwrap();
    assert_eq!(alerts.0.lock().unwrap().len(), 1);
    assert_eq!(alerts.0.lock().unwrap()[0].resource_id, platform);
    assert_eq!(alerts.0.lock().unwrap()[0].resource_type, "Platform");
    let rule = Uuid::now_v7();
    sqlx::query("INSERT INTO alertrules(id,name,createdbyactorid,limitedto,quiethours,severity,status,type) VALUES($1,'Unmanaged fixture',$2,$3,'[]','Critical','Enabled','UnmanagedContainerCreated')")
        .bind(rule).bind(citadel_identity::SYSTEM_ACTOR_ID).bind(serde_json::json!([{"resourceId": platform, "resourceType": "Platform"}])).execute(&pool).await.unwrap();
    let persisted = citadel_adapters::alert_store::PostgresAlertStore::new(pool.clone());
    observe(&pool, &persisted, &request).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, Uuid>("SELECT resourceid FROM alertevents WHERE alertruleid=$1")
            .bind(rule)
            .fetch_one(&pool)
            .await
            .unwrap(),
        platform
    );
    for column in ["issystem", "isswarmtask"] {
        let sql = format!("UPDATE containers SET {column}=true WHERE id=$1");
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        observe(&pool, &alerts, &request).await.unwrap();
        assert_eq!(alerts.0.lock().unwrap().len(), 1);
        let sql = format!("UPDATE containers SET {column}=false WHERE id=$1");
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$2,$3,'{}','Healthy',$4)")
        .bind(deployment).bind(deployment.to_string()).bind(platform).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    sqlx::query("UPDATE containers SET deploymentid=$2 WHERE id=$1")
        .bind(id)
        .bind(deployment)
        .execute(&pool)
        .await
        .unwrap();
    observe(&pool, &alerts, &request).await.unwrap();
    sqlx::query("DELETE FROM containers WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    observe(&pool, &alerts, &request).await.unwrap();
    assert_eq!(alerts.0.lock().unwrap().len(), 1);
    pool.close().await;
}

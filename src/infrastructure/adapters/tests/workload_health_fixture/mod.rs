use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;
#[allow(dead_code)]
pub async fn pool() -> PgPool {
    let url = std::env::var("CITADEL_WORKLOAD_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap()
}
#[allow(dead_code)]
pub async fn platform(pool: &PgPool, swarm: bool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$1::text,'Local',1,0,1024,$1::text,0,$2,'Online',0)")
        .bind(id).bind(json!({"$type":if swarm {"DockerSwarm"} else {"Docker"}})).execute(pool).await.unwrap();
    id
}
#[allow(dead_code)]
pub async fn container(
    pool: &PgPool,
    platform: Uuid,
    docker_id: &str,
    node: Option<&str>,
    owner: Option<Uuid>,
    stack: Option<Uuid>,
    service: Option<&str>,
    result: &str,
) -> Uuid {
    let id = Uuid::now_v7();
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,dockernodeid,deploymentid,stackid,composeservicename,health,isswarmtask) VALUES($1,$2,$3,'image',$3,1,$4,'Running','[]',$5,$6,$7,$8,$9,$10)")
        .bind(id).bind(platform).bind(docker_id).bind(now/1000).bind(node).bind(owner).bind(stack).bind(service)
        .bind(json!({"configuration":"Enabled","result":result,"observedAt":now})).bind(node.is_some()).execute(pool).await.unwrap();
    id
}

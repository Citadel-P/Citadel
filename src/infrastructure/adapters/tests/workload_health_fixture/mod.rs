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
pub struct Container<'a> {
    pub platform: Uuid,
    pub docker_id: &'a str,
    pub node: Option<&'a str>,
    pub owner: Option<Uuid>,
    pub stack: Option<Uuid>,
    pub service: Option<&'a str>,
    pub result: &'a str,
}
#[allow(dead_code)]
pub async fn container(pool: &PgPool, container: Container<'_>) -> Uuid {
    let Container {
        platform,
        docker_id,
        node,
        owner,
        stack,
        service,
        result,
    } = container;
    let id = Uuid::now_v7();
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,dockernodeid,deploymentid,stackid,composeservicename,health,isswarmtask) VALUES($1,$2,$3,'image',$3,1,$4,'Running','[]',$5,$6,$7,$8,$9,$10)")
        .bind(id).bind(platform).bind(docker_id).bind(now/1000).bind(node).bind(owner).bind(stack).bind(service)
        .bind(json!({"configuration":"Enabled","result":result,"observedAt":now})).bind(node.is_some()).execute(pool).await.unwrap();
    id
}

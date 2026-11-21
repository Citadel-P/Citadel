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

/// Exercise the same ambiguous task identity and conflicting ownership through both readers.
#[allow(dead_code)]
pub async fn check_swarm_member_scope<F, Fut>(pool: &PgPool, platform: Uuid, read: F)
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = citadel_primitives::health::WorkloadHealth>,
{
    // A new desired-running task replaces the old slot but has no known node yet.
    sqlx::query("INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockercontainerid,dockernodeid,dockerserviceid,image,name,nodehostname,observedat,ports,servicename,slot,state,versionindex,dockercreatedat) VALUES($1,'replacement','running','same-id','','service','nginx','','',now(),'[]','web',1,'running',3,now()+interval '1 minute')")
        .bind(platform).execute(pool).await.unwrap();
    let health = read().await;
    assert_eq!(health.members.len(), 1);
    assert_eq!(health.members[0].task_id.as_deref(), Some("replacement"));
    assert_eq!(health.members[0].container_id, None);
    assert!(!health.members[0].available);
    sqlx::query("INSERT INTO swarmnoderuntimeprojectionstates(platformid,dockernodeid,isstale) VALUES($1,'replacement-worker',false)")
        .bind(platform).execute(pool).await.unwrap();
    let worker = container(
        pool,
        Container {
            platform,
            docker_id: "same-id",
            node: Some("replacement-worker"),
            owner: None,
            stack: None,
            service: None,
            result: "Healthy",
        },
    )
    .await;
    sqlx::query("UPDATE swarmtaskprojections SET dockernodeid='replacement-worker' WHERE platformid=$1 AND dockertaskid='replacement'")
        .bind(platform).execute(pool).await.unwrap();
    let health = read().await;
    assert_eq!(health.members[0].container_id, Some(worker));
    assert!(health.members[0].available);
    sqlx::query("UPDATE swarmnoderuntimeprojectionstates SET isstale=true WHERE platformid=$1 AND dockernodeid='replacement-worker'")
        .bind(platform).execute(pool).await.unwrap();
    assert!(!read().await.members[0].available);
    let previous: String = sqlx::query_scalar(
        "SELECT ownership FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid='service'",
    )
    .bind(platform)
    .fetch_one(pool)
    .await
    .unwrap();
    let set_ownership = "UPDATE swarmserviceprojections SET ownership=$2 WHERE platformid=$1 AND dockerserviceid='service'";
    sqlx::query(set_ownership)
        .bind(platform)
        .bind("OwnershipConflict")
        .execute(pool)
        .await
        .unwrap();
    assert!(read().await.members.is_empty());
    sqlx::query(set_ownership)
        .bind(platform)
        .bind(previous)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM swarmtaskprojections WHERE platformid=$1 AND dockertaskid='replacement'",
    )
    .bind(platform)
    .execute(pool)
    .await
    .unwrap();
}

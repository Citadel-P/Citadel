use super::*;
use citadel_platforms::{
    RuntimeCapabilityError,
    containers::{ContainerMutationRuntime, ContainerTarget},
};
use futures_util::future::BoxFuture;
#[derive(Default)]
struct Runtime(std::sync::Mutex<Vec<ContainerTarget>>);
impl ContainerMutationRuntime for Runtime {
    fn mutate<'a>(
        &'a self,
        target: &'a ContainerTarget,
        action: ContainerAction,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            assert_eq!(
                action,
                ContainerAction::Delete(DeleteContainerOptions::default())
            );
            self.0.lock().unwrap().push(target.clone());
            Ok(())
        })
    }
    fn observe<'a>(
        &'a self,
        _: &'a ContainerTarget,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, RuntimeCapabilityError>> {
        Box::pin(async { Ok(None) })
    }
}

// Port: ContainerSyncJobTests pruning enabled/disabled, terminal tasks only;
// DockerDaemonEventJobTests terminal task event; also verifies node identity.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn pruning_obeys_setting_and_only_deletes_terminal_tasks_without_volumes_or_force() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status,prunehistoricalswarmtaskcontainers) VALUES($1,$2,$2,'Agent',0,0,0,0,0,'{\"$type\":\"DockerSwarm\"}','Online',false)")
        .bind(platform).bind(platform.to_string()).execute(&pool).await.unwrap();
    let mut ids = Vec::new();
    for (state, task) in [
        ("Exited", true),
        ("Dead", true),
        ("Running", true),
        ("Exited", false),
    ] {
        let id = uuid::Uuid::now_v7();
        ids.push(id);
        sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,state,created,updated,ports,issystem,isswarmtask,dockernodeid) VALUES($1,$2,$3,'image',$3,$4,0,0,'{}',false,$5,'node-a')")
            .bind(id).bind(platform).bind(id.to_string()).bind(state).bind(task).execute(&pool).await.unwrap();
    }
    let runtime = Arc::new(Runtime::default());
    let service = Arc::new(ContainerMutationService::new(
        Arc::new(
            citadel_adapters::container_mutation_store::PostgresContainerRepository::new(
                pool.clone(),
            ),
        ),
        runtime.clone(),
    ));
    let token = CancellationToken::new();
    let events = crate::workers::listener(&pool, "citadel_swarm_prune")
        .await
        .unwrap();
    let worker = tokio::spawn(run(token.clone(), pool.clone(), service, events));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(runtime.0.lock().unwrap().is_empty());
    sqlx::query("UPDATE platforms SET prunehistoricalswarmtaskcontainers=true WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("SELECT pg_notify('citadel_swarm_prune',$1)")
        .bind(platform.to_string())
        .execute(&pool)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(12), async {
        while runtime.0.lock().unwrap().len() < 2 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("enabled terminal tasks must be pruned");
    token.cancel();
    worker.await.unwrap().unwrap();
    {
        let targets = runtime.0.lock().unwrap();
        assert_eq!(targets.len(), 2);
        assert!(
            targets
                .iter()
                .all(|t| ids[..2].contains(&t.id) && t.node_id.as_deref() == Some("node-a"))
        );
    }
    pool.close().await;
}

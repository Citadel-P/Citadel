use citadel_adapters::persistence::postgres::{
    backups::source_planner::PostgresBackupSourcePlanner, stacks::PostgresStackRepository,
};
use citadel_backups::{BackupSourcePlanner, policies::read_models::BackupPreviewKind};
use citadel_database::MigrationRunner;
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_primitives::ActorId;
use citadel_stacks::{
    CreateStack, StackRepository, StackSource, StackSpec, StackSpecCommon, StackUpdateBehavior,
};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_EXECUTION_DATABASE_URL"]
async fn swarm_backup_preview_resolves_legacy_namespace_and_apply_records_reservations() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///var/run/docker.sock','Local',1,0,1048576,$2,0,'{\"$type\":\"DockerSwarm\"}','Online',0)")
        .bind(platform).bind(format!("backup-test-{platform}")).execute(&pool).await.unwrap();
    let store = PostgresStackRepository::new(pool.clone());
    let stack = store.create(actor, true, &CreateStack {
        name: format!("backup-stack-{platform}"), platform_id: platform, description: None, stack_source: StackSource::WebEditor,
        spec: StackSpec::WebEditor { compose_file: "services:\n  web:\n    image: nginx:alpine\n    volumes:\n      - data:/data\nvolumes:\n  data: {}\n".into(), update_behavior: StackUpdateBehavior::Disabled, common: StackSpecCommon { project_name: Some("actual".into()), ..Default::default() } },
        drift_policy: None, tag_ids: vec![], duplicate_source: None,
    }).await.unwrap();
    use citadel_backups::coverage::CoverageReader;
    let workloads =
        citadel_adapters::persistence::postgres::backups::coverage::PostgresVolumeCoverageReader(
            pool.clone(),
        )
        .snapshot(actor, true, &[platform])
        .await
        .unwrap();
    assert!(
        workloads
            .workloads
            .iter()
            .any(|w| w.kind == BackupPreviewKind::Stack && w.id == stack.id)
    );
    let planner = PostgresBackupSourcePlanner::new(pool.clone());
    let cancel = CancellationToken::new();
    let preview = planner
        .preview(BackupPreviewKind::Stack, stack.id, actor, true, &cancel)
        .await
        .unwrap();
    assert!(preview.volumes.is_empty());
    assert!(
        preview
            .warnings
            .iter()
            .any(|w| w.contains("No applied Swarm Stack"))
    );

    // A normally deployed legacy stack has owned inventory, but no reservation.
    sqlx::query("INSERT INTO swarmserviceprojections(platformid,dockerserviceid,name,image,mode,desiredtaskcount,runningtaskcount,ports,networkids,secretids,configids,labels,updatestate,versionindex,observedat,stackid,dockerstacknamespace,ownership) VALUES($1,'service','actual_web','nginx:alpine','Replicated',1,1,'[]','[]','[]','[]','{}','',1,now(),$2,'actual','CitadelStack')")
        .bind(platform).bind(stack.id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO swarmtaskprojections(platformid,dockertaskid,desiredstate,dockernodeid,dockerserviceid,image,name,nodehostname,observedat,ports,servicename,state,statusmessage,versionindex) VALUES($1,'task','running','node-a','service','nginx:alpine','actual_web.1','worker-a',now(),'[]','actual_web','running','',1)")
        .bind(platform).execute(&pool).await.unwrap();
    let preview = planner
        .preview(BackupPreviewKind::Stack, stack.id, actor, true, &cancel)
        .await
        .unwrap();
    assert_eq!(preview.volumes.len(), 1);
    assert_eq!(preview.volumes[0].name, "actual_data");
    assert_eq!(preview.volumes[0].docker_node_id.as_deref(), Some("node-a"));

    let claim = store
        .claim_apply(actor, true, stack.id, None, None)
        .await
        .unwrap();
    let reservation: String = sqlx::query_scalar(
        "SELECT namespace FROM stackswarmnamespacereservations WHERE stackid=$1 AND platformid=$2",
    )
    .bind(stack.id)
    .bind(platform)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(reservation, claim.project_name);
    let preview = planner
        .preview(BackupPreviewKind::Stack, stack.id, actor, true, &cancel)
        .await
        .unwrap();
    assert_eq!(preview.volumes[0].name, "actual_data");

    sqlx::query(
        "UPDATE stackswarmnamespacereservations SET namespace='different' WHERE stackid=$1",
    )
    .bind(stack.id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        planner
            .preview(BackupPreviewKind::Stack, stack.id, actor, true, &cancel)
            .await,
        Err(citadel_backups::BackupError::Conflict(_))
    ));
}

use super::*;
use citadel_adapters::connectors::agent::client::AgentClient;
use citadel_adapters::connectors::agent::client::AgentRequestSigner;
use citadel_adapters::connectors::routing::platforms::runtime::PlatformRuntimeRouter;
use citadel_adapters::connectors::routing::resource_copies::CopyStorageRuntime;
use citadel_adapters::connectors::routing::resource_copies::verify_destination;
use citadel_adapters::persistence::postgres::deployments::PostgresDeploymentRepository;
use citadel_adapters::persistence::postgres::resource_copies::PostgresCopyRepository;
use citadel_adapters::persistence::postgres::stacks::PostgresStackRepository;
use citadel_deployments::DeploymentRepository;
use citadel_docker::client::DockerClient;
use citadel_resource_copies::*;
use citadel_stacks::StackRepository;

// A real second Docker daemon: restores must work without the original volume
// existing there, and no destination data may appear on Core's daemon.
pub(super) async fn verify(
    pool: &sqlx::PgPool,
    backups: &PostgresBackupPersistence,
    run: &BackupRun,
) {
    let suffix = Uuid::now_v7().simple().to_string();
    let daemon = format!("citadel-cross-copy-{suffix}");
    let agent_name = format!("{daemon}-agent");
    let socket = format!("{daemon}-socket");
    let directory = std::env::temp_dir().join(&daemon);
    let outcome = AssertUnwindSafe(async {
        let network = std::env::var("CITADEL_TEST_AGENT_NETWORK").unwrap();
        let image = std::env::var("CITADEL_TEST_AGENT_IMAGE").unwrap();
        docker(&["run","-d","--privileged","--name",&daemon,"--network",&network,"-e","DOCKER_TLS_CERTDIR=","-v",&format!("{socket}:/var/run"),"docker:27.5.1-dind","--host=unix:///var/run/docker.sock","--bip=10.224.0.1/24"],None).await;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            let ready = super::run(ProcessRequest::new("docker").args(["exec", &daemon, "docker", "info"]), &CancellationToken::new()).await.unwrap();
            if ready.succeeded() {break;}
            assert!(tokio::time::Instant::now() < deadline, "Destination daemon unavailable");
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        std::fs::create_dir(&directory).unwrap();
        let archive = directory.join("helper.tar");
        docker(&["save","-o",archive.to_str().unwrap(),&image,IMAGE],None).await;
        docker(&["cp",archive.to_str().unwrap(),&format!("{daemon}:/root/helper.tar")],None).await;
        docker(&["exec",&daemon,"docker","load","-i","/root/helper.tar"],None).await;
        let mut key=[0;32];getrandom::fill(&mut key).unwrap();
        let signer=AgentRequestSigner::from_bytes(&key);key.fill(0);
        docker(&["run","-d","--name",&agent_name,"--network",&network,"-v",&format!("{socket}:/var/run"),"-e",&format!("CITADEL_CORE_PUBLIC_KEY={}",signer.public_key_base64()),"-e","CITADEL_AGENT_TLS_MODE=Disabled",&image],None).await;
        let address=format!("http://{agent_name}:9000");
        let deadline=tokio::time::Instant::now()+Duration::from_secs(30);
        let agent=loop {
            if let Ok(client)=AgentClient::connect(&address,signer.clone(),Duration::from_secs(10),true).await
                && client.handshake(&CancellationToken::new()).await.is_ok() { break client; }
            assert!(tokio::time::Instant::now()<deadline,"Destination Agent unavailable");
            tokio::time::sleep(Duration::from_millis(250)).await;
        };
        let actor=super::resource_copy::administrator(pool).await;
        let target=Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Agent',0,0,0,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)").bind(target).bind(&address).execute(pool).await.unwrap();
        let source_platform=run.items[0].platform_id;
        let volume=&run.items[0].volume_name;
        let deployment=PostgresDeploymentRepository::new(pool.clone()).create(actor,true,&citadel_deployments::CreateDeployment {name:format!("cross-source-{suffix}"),platform_id:source_platform,description:None,tag_ids:vec![],duplicate_source:None,spec:serde_json::from_value(json!({"image":{"$type":"Local","imageId":"fixture"},"volumes":[format!("{volume}:/data")]})).unwrap()}).await.unwrap();
        let stack=PostgresStackRepository::new(pool.clone()).create(actor,true,&citadel_stacks::CreateStack {name:format!("cross-stack-{suffix}"),platform_id:source_platform,description:None,tag_ids:vec![],duplicate_source:None,stack_source:citadel_stacks::StackSource::WebEditor,drift_policy:None,spec:serde_json::from_value(json!({"$type":"WebEditor","composeFile":format!("services:\n  a:\n    image: alpine\n    volumes: [data:/data]\nvolumes:\n  data:\n    external: true\n    name: {volume}\n")})).unwrap()}).await.unwrap();
        let store=PostgresCopyRepository::new(pool.clone());
        // Invalid Core socket proves preflight and ownership checks use destination routing.
        let router=Arc::new(PlatformRuntimeRouter::new(pool.clone(),DockerClient::new("/no-source-daemon.sock",Duration::from_secs(2)).unwrap(),Some(agent.clone()),Default::default()));
        let storage=CopyStorageRuntime {image:IMAGE.into(),runtime:router.clone()};
        let executor=DockerResticBackupExecutor::new("/no-source-daemon",IMAGE,Arc::new(Password),256*1024,pool.clone()).with_agent(Some(agent));
        let cancel=CancellationToken::new();
        for (kind,id) in [(CopyKind::Deployment,deployment.id),(CopyKind::Stack,stack.id)] {
            let source=store.source(actor,kind,id).await.unwrap();
            let request=CopyRequest {destination_platform_id:Some(target),destination_node_id:None,data_mode:CopyDataMode::ExistingBackup,consistency_acknowledged:false,kind,source_id:id,name:format!("cross-target-{}",Uuid::now_v7().simple()),selections:vec![CopySelection{volume:volume.clone(),backup_run_id:run.id,backup_run_item_id:run.items[0].id}]};
            let snapshots=store.snapshots(actor,&source).await.unwrap();
            let mut plan=planning::plan(Uuid::now_v7(),source,request,snapshots).unwrap();
            store.prepare_destination(actor,&mut plan).await.unwrap();
            assert!(plan.preview.blockers.is_empty());
            storage.check(&plan,None,&cancel).await.unwrap();
            store.save_review(actor,&plan).await.unwrap();
            let started=store.start(actor,plan.preview.id).await.unwrap();
            let claim=backups.claim_restore(Utc::now()-chrono::Duration::hours(1)).await.unwrap().unwrap();
            assert_eq!(claim.run.id,started.restores[0].id);
            assert_eq!(claim.run.target_platform_id,target);
            let result=executor.restore(&claim,&cancel).await;
            assert_eq!(result.status,BackupRestoreStatus::Succeeded,"{:?}",result.error_message);
            backups.finish_restore(&claim,&result).await.unwrap();
            let progress=store.get(actor,plan.preview.id).await.unwrap();
            storage.check(&plan,Some(&progress.restores),&cancel).await.unwrap();
            let completed=store.advance(actor,plan.preview.id,Some(Ok(()))).await.unwrap();
            assert_eq!(completed.status,CopyStatus::Succeeded,"{:?}",completed.message);
            verify_destination(pool,&router,completed.destination_id.unwrap(),target,None,&cancel).await.unwrap();
            let target_volume=&plan.preview.volumes[0].target;
            assert_eq!(docker(&["exec",&daemon,"docker","run","--rm","-v",&format!("{target_volume}:/data:ro"),"--entrypoint","sh",IMAGE,"-ec","test -L /data/link; test $(stat -c %u:%g /data/payload.txt) = 1000:1001; cat /data/payload.txt"],None).await,PAYLOAD);
            let on_core=super::run(ProcessRequest::new("docker").args(["volume","inspect",target_volume]),&cancel).await.unwrap();
            assert!(!on_core.succeeded(),"Destination volume was created on Core's daemon");
            storage.cleanup(&plan,&progress.restores,&cancel).await.unwrap();
        }
    }).catch_unwind().await;
    // Always remove only this fixture's daemon and its owned storage.
    for args in [
        vec!["rm", "-f", "-v", &agent_name],
        vec!["rm", "-f", "-v", &daemon],
        vec!["volume", "rm", &socket],
    ] {
        let _ = super::run(
            ProcessRequest::new("docker").args(args),
            &CancellationToken::new(),
        )
        .await;
    }
    let _ = std::fs::remove_dir_all(directory);
    if let Err(error) = outcome {
        std::panic::resume_unwind(error);
    }
}

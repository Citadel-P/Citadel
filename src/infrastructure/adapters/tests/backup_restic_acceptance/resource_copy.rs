use super::*;
use citadel_adapters::connectors::routing::{
    platforms::runtime::PlatformRuntimeRouter,
    resource_copies::{CopyStorageRuntime, verify_destination},
};
use citadel_adapters::persistence::postgres::{
    deployments::PostgresDeploymentRepository, resource_copies::PostgresCopyRepository,
    stacks::PostgresStackRepository,
};
use citadel_deployments::DeploymentRepository;
use citadel_resource_copies::*;
use citadel_stacks::StackRepository;

pub(super) async fn verify_copy_lifecycle(
    pool: &sqlx::PgPool,
    backups: &PostgresBackupPersistence,
    executor: &DockerResticBackupExecutor,
    run: &BackupRun,
    volume: &str,
    router: Arc<PlatformRuntimeRouter>,
) {
    let actor = administrator(pool).await;
    let platform = run.items[0].platform_id;
    let storage = CopyStorageRuntime {
        image: "restic/restic:0.18.1".into(),
        runtime: router.clone(),
    };
    let copies = PostgresCopyRepository::new(pool.clone());
    let deployment=PostgresDeploymentRepository::new(pool.clone()).create(actor,true,&citadel_deployments::CreateDeployment {name:format!("copy-source-{}",Uuid::now_v7().simple()),platform_id:platform,description:None,spec:serde_json::from_value(json!({"image":{"$type":"Local","imageId":"fixture"},"volumes":[format!("{volume}:/data")]})).unwrap(),tag_ids:vec![],duplicate_source:None}).await.unwrap();
    let stack=PostgresStackRepository::new(pool.clone()).create(actor,true,&citadel_stacks::CreateStack {name:format!("copy-stack-{}",Uuid::now_v7().simple()),platform_id:platform,description:None,stack_source:citadel_stacks::StackSource::WebEditor,spec:serde_json::from_value(json!({"$type":"WebEditor","composeFile":format!("services:\n  a:\n    image: alpine\n    volumes: [data:/data]\n  b:\n    image: alpine\n    volumes: [data:/shared:ro]\nvolumes:\n  data:\n    name: {volume}\n    external: true\n")})).unwrap(),drift_policy:None,tag_ids:vec![],duplicate_source:None}).await.unwrap();
    let cancel = CancellationToken::new();
    let mut targets = Vec::new();
    let outcome=AssertUnwindSafe(async {
        for (kind,source_id) in [(CopyKind::Deployment,deployment.id),(CopyKind::Stack,stack.id)] {
          for mode in [CopyDataMode::ExistingBackup, CopyDataMode::CurrentData] {
            let source=copies.source(actor,kind,source_id).await.unwrap();
            let request=CopyRequest { destination_platform_id: None, destination_node_id: None, data_mode: mode, consistency_acknowledged: mode == CopyDataMode::CurrentData,kind,source_id,name:format!("copied-{}",Uuid::now_v7().simple()),selections:if mode == CopyDataMode::ExistingBackup { vec![CopySelection {volume:volume.into(),backup_run_id:run.id,backup_run_item_id:run.items[0].id}] } else {vec![]}};
            let plan=planning::plan(Uuid::now_v7(),source,request,copies.snapshots(actor,&copies.source(actor,kind,source_id).await.unwrap()).await.unwrap()).unwrap();
            assert!(plan.preview.blockers.is_empty(),"{:?}",plan.preview.blockers);
            storage.check(&plan,None,&cancel).await.unwrap();
            copies.save_review(actor,&plan).await.unwrap();
            // Runtime inventory refreshes must not invalidate a reviewed copy.
            sqlx::query("UPDATE platforms SET platformdescriptor=(platformdescriptor::jsonb || '{\"containerCount\":42,\"containersRunning\":17}'::jsonb)::json WHERE id=$1").bind(platform).execute(pool).await.unwrap();
            assert_eq!(copies.source(actor,kind,source_id).await.unwrap().fingerprint, plan.source.fingerprint);
            let (left,right)=tokio::join!(copies.start(actor,plan.preview.id),copies.start(actor,plan.preview.id));
            let started=match (left,right) { (Ok(value),_) | (_,Ok(value))=>value, (Err(left),Err(right))=>panic!("{left}; {right}") };
            // Reopen/retry must point at exactly the same durable child.
            let repeated=copies.start(actor,plan.preview.id).await.unwrap();
            assert_eq!(started.restores[0].id,repeated.restores[0].id);
            assert_eq!(started.restores.len(),1,"shared Stack data must restore once");
            assert!(started.destination_id.is_none());
            targets.push(started.restores[0].target.clone());
            if mode == CopyDataMode::CurrentData {
                let (claimed, job) = copies.claim_current(actor, plan.preview.id).await.unwrap().unwrap();
                assert!(copies.claim_current(actor, plan.preview.id).await.unwrap().is_none());
                let result = storage.copy_current(&claimed, &job, &cancel).await;
                assert!(result.is_ok(), "{result:?}");
                copies.finish_current(actor, plan.preview.id, job.id, result.map_err(|e|e.to_string())).await.unwrap();
                assert!(backups.claim_restore(Utc::now()-chrono::Duration::hours(1)).await.unwrap().is_none());
            } else {
            assert!(!backups.acquire_repository_operation(run.backup_repository_id,Uuid::now_v7(),"Prune",Utc::now()+chrono::Duration::minutes(1)).await.unwrap());
            let claim=backups.claim_restore(Utc::now()-chrono::Duration::hours(1)).await.unwrap().unwrap();
            assert_eq!(claim.run.id,started.restores[0].id);
            let restored=executor.restore(&claim,&cancel).await;
            assert_eq!(restored.status,BackupRestoreStatus::Succeeded,"{:?}",restored.error_message);
            backups.finish_restore(&claim,&restored).await.unwrap();
            }
            assert!(copies.advance(actor,plan.preview.id,None).await.unwrap().destination_id.is_none(),"storage must be verified before finalization");
            let progress=copies.get(actor,plan.preview.id).await.unwrap();
            storage.check(&plan,Some(&progress.restores),&cancel).await.unwrap();
            let completed=copies.advance(actor,plan.preview.id,Some(Ok(()))).await.unwrap();
            assert_eq!(completed.status,CopyStatus::Succeeded,"{:?}",completed.message);
            let destination=completed.destination_id.unwrap();
            assert_eq!(copies.advance(actor,plan.preview.id,Some(Ok(()))).await.unwrap().destination_id,Some(destination));
            verify_destination(pool,&router,destination,platform,None,&cancel).await.unwrap();
            assert!(verify_destination(pool,&router,destination,Uuid::now_v7(),None,&cancel).await.is_err(), "moving a copy must not silently create empty storage");
            let name=&progress.restores[0].target;
            docker(&["run","--rm","--volume",&format!("{name}:/fixture:ro"),"--entrypoint","sh",IMAGE,"-ec","test $(stat -c %u:%g:%a /fixture) = 1000:1001:751"],None).await;
            assert_eq!(docker(&["run","--rm","--volume",&format!("{name}:/fixture"),"--entrypoint","sh",IMAGE,"-ec","test -L /fixture/link; test $(stat -c %u:%g /fixture/payload.txt) = 1000:1001; cat /fixture/payload.txt; printf copied > /fixture/copy-sentinel"],None).await,PAYLOAD);
            docker(&["run","--rm","--volume",&format!("{volume}:/fixture:ro"),"--entrypoint","sh",IMAGE,"-ec","test ! -e /fixture/copy-sentinel; test -L /fixture/link; test $(stat -c %u /fixture/payload.txt) = 1000"],None).await;
            // A removed/recreated destination must never be silently accepted on Apply.
            docker(&["volume","rm",name],None).await;
            docker(&["volume","create",name],None).await;
            assert!(verify_destination(pool,&router,destination,platform,None,&cancel).await.is_err());
            assert!(storage.cleanup(&plan,&progress.restores,&cancel).await.is_err(), "cleanup must reject a replacement volume");
            docker(&["volume","inspect",name],None).await;
          }
        }
        // A stale reviewed source cannot enqueue any restore; cancellation of a
        // queued parent produces no runnable resource or replay after restart.
        let source=copies.source(actor,CopyKind::Deployment,deployment.id).await.unwrap();
        let request=CopyRequest { destination_platform_id: None, destination_node_id: None, data_mode: citadel_resource_copies::CopyDataMode::ExistingBackup, consistency_acknowledged: false,kind:CopyKind::Deployment,source_id:deployment.id,name:format!("cancelled-{}",Uuid::now_v7().simple()),selections:vec![CopySelection{volume:volume.into(),backup_run_id:run.id,backup_run_item_id:run.items[0].id}]};
        let plan=planning::plan(Uuid::now_v7(),source.clone(),request,copies.snapshots(actor,&source).await.unwrap()).unwrap();
        copies.save_review(actor,&plan).await.unwrap();
        sqlx::query("UPDATE deployments SET description='changed after review',rowversion=rowversion+1 WHERE id=$1").bind(deployment.id).execute(pool).await.unwrap();
        assert!(matches!(copies.start(actor,plan.preview.id).await,Err(CopyError::Conflict(_))));
        let source=copies.source(actor,CopyKind::Deployment,deployment.id).await.unwrap();
        let plan=planning::plan(Uuid::now_v7(),source.clone(),plan.request,copies.snapshots(actor,&source).await.unwrap()).unwrap();
        copies.save_review(actor,&plan).await.unwrap();
        copies.start(actor,plan.preview.id).await.unwrap();
        copies.cancel(actor,plan.preview.id).await.unwrap();
        let completed=PostgresCopyRepository::new(pool.clone()).advance(actor,plan.preview.id,None).await.unwrap();
        assert_eq!(completed.status,CopyStatus::Cancelled);
        assert!(completed.destination_id.is_none());
        copies.cleanup(actor,plan.preview.id).await.unwrap();
        storage.cleanup(&plan,&completed.restores,&cancel).await.unwrap();
        copies.finish_cleanup(actor,plan.preview.id,Ok(())).await.unwrap();
        assert_eq!(copies.get(actor,plan.preview.id).await.unwrap().status,CopyStatus::Cleaned);

        // Failed children never create a partially restored workload, including
        // after the repository is reconstructed as it would be after restart.
        let mut request=plan.request.clone();
        request.name=format!("failed-{}",Uuid::now_v7().simple());
        let failed=planning::plan(Uuid::now_v7(),plan.source.clone(),request,copies.snapshots(actor,&plan.source).await.unwrap()).unwrap();
        copies.save_review(actor,&failed).await.unwrap();
        let started=copies.start(actor,failed.preview.id).await.unwrap();
        sqlx::query("UPDATE backuprestoreruns SET status='Failed',errormessage='Fixture restore failed',completedat=CURRENT_TIMESTAMP WHERE id=$1").bind(started.restores[0].id).execute(pool).await.unwrap();
        let stopped=PostgresCopyRepository::new(pool.clone()).advance(actor,failed.preview.id,None).await.unwrap();
        assert_eq!(stopped.status,CopyStatus::Failed);
        assert!(stopped.destination_id.is_none());
        assert_eq!(stopped.restores[0].message.as_deref(),Some("Fixture restore failed"));
        assert!(copies.get(ActorId::new(Uuid::now_v7()),failed.preview.id).await.is_err());

        // Revocation before completion cancels queued children and does not
        // depend on a frontend subscription or the initiating actor reconnecting.
        let mut request=plan.request.clone();
        request.name=format!("revoked-{}",Uuid::now_v7().simple());
        let revoked=planning::plan(Uuid::now_v7(),plan.source.clone(),request,copies.snapshots(actor,&plan.source).await.unwrap()).unwrap();
        copies.save_review(actor,&revoked).await.unwrap();
        copies.start(actor,revoked.preview.id).await.unwrap();
        sqlx::query("UPDATE actors SET isenabled=false WHERE id=$1").bind(actor.value()).execute(pool).await.unwrap();
        let stopped=copies.advance(actor,revoked.preview.id,None).await.unwrap();
        assert_eq!(stopped.status,CopyStatus::Failed);
        assert!(stopped.destination_id.is_none());
        assert!(stopped.restores.iter().all(|r|r.status=="Cancelled"));
    }).catch_unwind().await;
    for target in targets {
        let _ = run_process_remove(&target).await;
    }
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
async fn run_process_remove(target: &str) {
    let _ = run(
        ProcessRequest::new("docker").args(["volume", "rm", target]),
        &CancellationToken::new(),
    )
    .await;
}

pub(super) async fn administrator(pool: &sqlx::PgPool) -> ActorId {
    let actor = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,type,isenabled) VALUES($1,'User',true)")
        .bind(actor.value())
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,createdbyactorid,name) VALUES($1,$2,$3,$4)")
        .bind(Uuid::now_v7())
        .bind(actor.value())
        .bind(SYSTEM_ACTOR_ID)
        .bind(format!("copy-admin-{}", actor.value()))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor.value())
        .bind(citadel_identity::ADMIN_ROLE_ID)
        .execute(pool)
        .await
        .unwrap();
    actor
}

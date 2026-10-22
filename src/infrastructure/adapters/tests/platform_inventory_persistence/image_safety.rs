use super::*;
use citadel_adapters::persistence::postgres::{
    deployments::PostgresDeploymentRepository, platforms::images::PostgresImageMutationStore,
};
use citadel_deployments::{
    CreateDeployment, DeploymentError, DeploymentRepository, DeploymentSpec,
};
use citadel_platforms::{
    RuntimeErrorKind,
    image_mutations::ImageMutationStore,
    image_usage::{ImageDeletionReview, ImageLocation, ImageUsageStore},
};

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn image_safety_local_dependencies_reviews_authorization_and_tag_moves() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}' WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    let store = PostgresImageMutationStore(pool.clone());
    let image = RuntimeImageSummary {
        id: format!("sha256:{}", "a".repeat(64)),
        repo_tags: vec!["nginx:latest".into()],
        ..Default::default()
    };
    store
        .persist_pull(platform, Some(Uuid::from_u128(0x100)), &image)
        .await
        .unwrap();
    let image_id: Uuid = sqlx::query_scalar("SELECT id FROM images WHERE platformid=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    let ids = vec![image.id.clone()];
    let snapshot = store
        .deletion_snapshots(platform, &ids)
        .await
        .unwrap()
        .remove(0);
    let stale = ImageDeletionReview {
        docker_image_id: image.id.clone(),
        row_version: snapshot.row_version - 1,
        tags: snapshot.tags,
    };
    assert_eq!(
        store
            .claim_deletion(platform, &ids, false, &[stale])
            .await
            .err()
            .unwrap()
            .kind,
        RuntimeErrorKind::Conflict
    );
    let deployments = PostgresDeploymentRepository::new(pool.clone());
    let input = CreateDeployment {
        name: format!("image-dependency-{platform}"),
        platform_id: platform,
        description: None,
        spec: serde_json::from_value::<DeploymentSpec>(
            json!({"image":{"$type":"Local","imageId":image_id.to_string().to_uppercase()}}),
        )
        .unwrap(),
        tag_ids: vec![],
        duplicate_source: None,
    };
    // Pause deletion admission while an actual configuration write is waiting.
    let mut deleting = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM images WHERE id=$1 FOR UPDATE")
        .bind(image_id)
        .fetch_one(&mut *deleting)
        .await
        .unwrap();
    let mut saving = deployments.create(ActorId::new(actor), true, &input);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut saving)
            .await
            .is_err()
    );
    sqlx::query("UPDATE images SET controlstate='Processing',controlstartedat=1 WHERE id=$1")
        .bind(image_id)
        .execute(&mut *deleting)
        .await
        .unwrap();
    deleting.commit().await.unwrap();
    assert!(matches!(saving.await, Err(DeploymentError::Conflict(_))));
    sqlx::query("UPDATE images SET controlstate='Idle',controlstartedat=NULL WHERE id=$1")
        .bind(image_id)
        .execute(&pool)
        .await
        .unwrap();
    // Deletion wins admission: the shared create/config path must reject its image.
    let claim = store
        .claim_deletion(platform, &ids, false, &[])
        .await
        .unwrap();
    let spellings = [
        image_id.to_string().to_uppercase(),
        image_id.simple().to_string(),
        image_id.braced().to_string(),
        image_id.urn().to_string(),
    ];
    for spelling in &spellings {
        assert_eq!(Uuid::parse_str(spelling).unwrap(), image_id);
        let mut alternate = input.clone();
        alternate.spec.image = citadel_deployments::DeploymentImageInfo::Local {
            image_id: spelling.clone(),
        };
        assert!(
            matches!(
                deployments
                    .create(ActorId::new(actor), true, &alternate)
                    .await,
                Err(DeploymentError::Conflict(_))
            ),
            "a claimed image must reject {spelling}"
        );
    }
    store
        .complete_deletion(platform, &claim, None)
        .await
        .unwrap();
    let deployment = deployments
        .create(ActorId::new(actor), true, &input)
        .await
        .unwrap();
    // Configuration wins admission: deletion cannot claim the saved dependency.
    assert_eq!(
        store
            .claim_deletion(platform, &ids, false, &[])
            .await
            .err()
            .unwrap()
            .kind,
        RuntimeErrorKind::Conflict
    );
    let location = ImageLocation {
        docker_image_id: image.id.clone(),
        docker_node_id: None,
    };
    // Already persisted, noncanonical references must receive the same protection.
    for spelling in &spellings {
        sqlx::query("UPDATE deployments SET spec=jsonb_set(spec::jsonb,'{Image,ImageId}',to_jsonb($2::text)) WHERE id=$1")
            .bind(deployment.id).bind(spelling).execute(&pool).await.unwrap();
        assert!(store.deletion_snapshots(platform, &ids).await.unwrap()[0].has_local_dependency);
        assert_eq!(
            store
                .claim_deletion(platform, &ids, false, &[])
                .await
                .err()
                .unwrap()
                .kind,
            RuntimeErrorKind::Conflict
        );
        assert!(
            store
                .usage(ActorId::new(actor), true, platform, &location, 0)
                .await
                .unwrap()
                .entries
                .iter()
                .any(|entry| entry.workload_id == Some(deployment.id))
        );
    }
    let admin_usage = store
        .usage(ActorId::new(actor), true, platform, &location, 0)
        .await
        .unwrap();
    assert!(
        admin_usage
            .entries
            .iter()
            .any(|entry| entry.workload_id == Some(deployment.id))
    );
    let inventory = PostgresPlatformReader::new(pool.clone())
        .list_images(platform)
        .await
        .unwrap();
    let summaries = store
        .summaries(ActorId::new(actor), true, platform, &inventory)
        .await
        .unwrap();
    assert_eq!(summaries[0].deployments, 1);
    assert_eq!(summaries[0].containers, 0);
    let hidden_summaries = store
        .summaries(ActorId::new(Uuid::now_v7()), false, platform, &inventory)
        .await
        .unwrap();
    assert_eq!(hidden_summaries[0].deployments, 0);
    let hidden = store
        .usage(ActorId::new(Uuid::now_v7()), false, platform, &location, 0)
        .await
        .unwrap();
    assert!(
        hidden.entries.is_empty(),
        "hidden dependencies must not leak identifiers or counts"
    );
    assert!(store.deletion_snapshots(platform, &ids).await.unwrap()[0].has_local_dependency);
    let private_registry = Uuid::now_v7();
    sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{}',$2,$1::text,'registry.example','Enabled')").bind(private_registry).bind(actor).execute(&pool).await.unwrap();
    let mut external = input.clone();
    external.name = format!("hub-{platform}");
    external.spec.image = citadel_deployments::DeploymentImageInfo::External {
        registry_id: Uuid::from_u128(0x100),
        image_tag: "nginx".into(),
        resolved_digest: None,
    };
    let hub = deployments
        .create(ActorId::new(actor), true, &external)
        .await
        .unwrap();
    external.name = format!("private-{platform}");
    external.spec.image = citadel_deployments::DeploymentImageInfo::External {
        registry_id: private_registry,
        image_tag: "nginx".into(),
        resolved_digest: None,
    };
    let private = deployments
        .create(ActorId::new(actor), true, &external)
        .await
        .unwrap();
    let usage = store
        .usage(ActorId::new(actor), true, platform, &location, 0)
        .await
        .unwrap();
    assert!(
        usage
            .entries
            .iter()
            .any(|entry| entry.workload_id == Some(hub.id))
    );
    assert!(
        !usage
            .entries
            .iter()
            .any(|entry| entry.workload_id == Some(private.id)),
        "same spelling in another registry is not the same reference"
    );
    sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,platformid,ports,name,state,updated,deploymentid) VALUES($1,0,'hidden-stopped',$2,$3,'{}','private-container','Exited',0,$4)").bind(Uuid::now_v7()).bind(&image.id).bind(platform).bind(deployment.id).execute(&pool).await.unwrap();
    let usage = store
        .usage(ActorId::new(actor), true, platform, &location, 0)
        .await
        .unwrap();
    assert!(
        usage.entries.iter().any(
            |entry| entry.container_id.as_deref() == Some("hidden-stopped")
                && entry.container_state.as_deref() == Some("Exited")
        )
    );
    let hidden = store
        .usage(ActorId::new(Uuid::now_v7()), false, platform, &location, 0)
        .await
        .unwrap();
    assert!(hidden.entries.is_empty());
    sqlx::query("DELETE FROM containers WHERE platformid=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    // A configuration transaction that owns the SHARE lock commits before deletion
    // can inspect dependencies; the latter must see that new committed reference.
    sqlx::query("DELETE FROM deployments WHERE id=$1")
        .bind(deployment.id)
        .execute(&pool)
        .await
        .unwrap();
    let mut saving = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM images WHERE id=$1 FOR SHARE")
        .bind(image_id)
        .fetch_one(&mut *saving)
        .await
        .unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,controlstate,createdbyactorid) VALUES($1,$2,$3,$4,'Created','Idle',$5)")
        .bind(deployment.id).bind(&input.name).bind(platform).bind(input.spec.to_storage_value().unwrap()).bind(actor).execute(&mut *saving).await.unwrap();
    let mut deleting = store.claim_deletion(platform, &ids, false, &[]);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut deleting)
            .await
            .is_err()
    );
    saving.commit().await.unwrap();
    assert_eq!(
        deleting.await.err().unwrap().kind,
        RuntimeErrorKind::Conflict
    );
    // Pulling a new tag preserves the old image UUID and its Local-image reference.
    let newer = RuntimeImageSummary {
        id: format!("sha256:{}", "b".repeat(64)),
        repo_tags: image.repo_tags.clone(),
        ..Default::default()
    };
    store
        .persist_pull(platform, Some(Uuid::from_u128(0x100)), &newer)
        .await
        .unwrap();
    let old: (Uuid, serde_json::Value) =
        sqlx::query_as("SELECT id,tags FROM images WHERE platformid=$1 AND dockerimageid=$2")
            .bind(platform)
            .bind(&image.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(old, (image_id, json!([])));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM images WHERE platformid=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn image_usage_fills_matching_pages_and_resumes_bounded_scans() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    let store = PostgresImageMutationStore(pool.clone());
    let image = RuntimeImageSummary {
        id: format!("sha256:{}", "c".repeat(64)),
        repo_tags: vec!["nginx:latest".into()],
        ..Default::default()
    };
    store
        .persist_pull(platform, Some(Uuid::from_u128(0x100)), &image)
        .await
        .unwrap();
    let location = ImageLocation {
        docker_image_id: image.id.clone(),
        docker_node_id: None,
    };
    sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,platformid,ports,name,state,updated) SELECT gen_random_uuid(),0,'container-'||n,$1,$2,'{}','container-'||n,'Exited',0 FROM generate_series(1,60) n")
        .bind(&image.id).bind(platform).execute(&pool).await.unwrap();
    for batch_size in [60, 500] {
        let mut tx = pool.begin().await.unwrap();
        for _ in 0..batch_size {
            let stack = Uuid::now_v7();
            let release = Uuid::now_v7();
            sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,$1::text,$2,'WebEditor','{}','{}')")
                .bind(stack).bind(actor).execute(&mut *tx).await.unwrap();
            sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,$4,$5,'Healthy','1')")
                .bind(release).bind(actor).bind(platform).bind(json!({"composeFile":"services:\n  app:\n    image: redis:latest\n"})).bind(stack).execute(&mut *tx).await.unwrap();
            sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
                .bind(release)
                .bind(stack)
                .execute(&mut *tx)
                .await
                .unwrap();
        }
        tx.commit().await.unwrap();
        let mut page = store
            .usage(ActorId::new(actor), true, platform, &location, 0)
            .await
            .unwrap();
        if batch_size == 500 {
            assert!(page.entries.is_empty());
            assert_eq!(
                page.next_offset,
                Some(500),
                "bounded scans must expose their continuation"
            );
            page = store
                .usage(
                    ActorId::new(actor),
                    true,
                    platform,
                    &location,
                    page.next_offset.unwrap(),
                )
                .await
                .unwrap();
        }
        assert_eq!(
            page.entries.len(),
            50,
            "unrelated Compose candidates must not consume the result page"
        );
        let next = store
            .usage(
                ActorId::new(actor),
                true,
                platform,
                &location,
                page.next_offset.unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(next.entries.len(), 10);
        assert!(next.next_offset.is_none());
        let ids: std::collections::HashSet<_> = page
            .entries
            .iter()
            .chain(&next.entries)
            .map(|entry| entry.container_id.as_ref().unwrap())
            .collect();
        assert_eq!(
            ids.len(),
            60,
            "continuation must neither skip nor repeat matching references"
        );
    }
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn image_usage_reports_owner_status_separately_from_container_state() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let actor = Uuid::now_v7();
    seed_platform(&pool, platform, actor, Uuid::now_v7()).await;
    let store = PostgresImageMutationStore(pool.clone());
    let image = RuntimeImageSummary {
        id: format!("sha256:{}", "f".repeat(64)),
        repo_tags: vec!["nginx:latest".into()],
        ..Default::default()
    };
    store
        .persist_pull(platform, Some(Uuid::from_u128(0x100)), &image)
        .await
        .unwrap();
    let deployment = Uuid::now_v7();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    let service = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,'usage-deployment',$2,$3,'Healthy',$4)")
        .bind(deployment).bind(platform).bind(json!({"Image":{"$type":"Local","ImageId":image.id}})).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO containers(id,created,dockercontainerid,dockerimageid,platformid,ports,name,state,updated,deploymentid) VALUES($1,0,'usage-container',$2,$3,'{}','usage-container','Exited',0,$4)")
        .bind(Uuid::now_v7()).bind(&image.id).bind(platform).bind(deployment).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,'usage-stack',$2,'WebEditor','{}','{}')")
        .bind(stack).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,createdbyactorid,platformid,spec,stackid,status,version) VALUES($1,$2,$3,$4,$5,'Stopped','1')")
        .bind(release).bind(actor).bind(platform).bind(json!({"composeFile":"services:\n  app:\n    image: nginx:latest\n"})).bind(stack).execute(&pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,desiredspechash,spec,health,synchronizationstate,updatedat) VALUES($1,'usage-service','usage-service',$2,$3,'test',$4,'Degraded','Unknown',now())")
        .bind(service).bind(platform).bind(actor).bind(json!({"image":{"imageTag":"nginx:latest","registryId":Uuid::from_u128(0x100)}})).execute(&pool).await.unwrap();
    let location = ImageLocation {
        docker_image_id: image.id,
        docker_node_id: None,
    };
    let usage = store
        .usage(ActorId::new(actor), true, platform, &location, 0)
        .await
        .unwrap();
    for (id, status) in [
        (deployment, "Healthy"),
        (stack, "Stopped"),
        (service, "Degraded"),
    ] {
        let entries: Vec<_> = usage
            .entries
            .iter()
            .filter(|entry| entry.workload_id == Some(id))
            .collect();
        assert!(!entries.is_empty());
        assert!(
            entries
                .iter()
                .all(|entry| entry.workload_status.as_deref() == Some(status))
        );
    }
    let container = usage
        .entries
        .iter()
        .find(|entry| entry.container_id.as_deref() == Some("usage-container"))
        .unwrap();
    assert_eq!(container.container_state.as_deref(), Some("Exited"));
    assert_eq!(container.workload_status.as_deref(), Some("Healthy"));
    assert!(
        store
            .usage(ActorId::new(Uuid::now_v7()), false, platform, &location, 0)
            .await
            .unwrap()
            .entries
            .is_empty()
    );
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn image_update_observations_fence_old_checks_and_tag_moves() {
    use citadel_adapters::persistence::postgres::platforms::image_updates::PostgresImageUpdateStore;
    use citadel_platforms::image_update_store::ImageUpdateStore;
    use citadel_platforms::image_updates::{
        ImageTagUpdate, ImageUpdateRegistry, ImageUpdateStatus,
    };
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let image_id = Uuid::now_v7();
    let image = format!("sha256:{}", "a".repeat(64));
    sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags) VALUES($1,$2,'nginx',$3,now(),'[\"nginx:17\"]')").bind(image_id).bind(&image).bind(platform).execute(&pool).await.unwrap();
    let registry = ImageUpdateRegistry {
        id: Uuid::from_u128(0x100),
        host: "docker.io".into(),
        revision: "one".into(),
    };
    let store = PostgresImageUpdateStore(pool.clone());
    let now = Utc::now();
    let mut update = ImageTagUpdate {
        reference: "nginx:17".into(),
        registry_id: Some(registry.id),
        status: ImageUpdateStatus::UpdateAvailable,
        reason: None,
        last_attempt_at: Some(now),
        last_success_at: Some(now),
        local_config_digest: Some(image.clone()),
        remote_config_digest: Some(format!("sha256:{}", "b".repeat(64))),
        remote_manifest_digest: None,
        is_stale: false,
    };
    assert!(
        store
            .save(platform, &image, &registry, &update, false)
            .await
            .unwrap()
    );
    update.status = ImageUpdateStatus::UpToDate;
    update.last_attempt_at = Some(now - chrono::Duration::seconds(10));
    assert!(
        !store
            .save(platform, &image, &registry, &update, true)
            .await
            .unwrap()
    );
    let association: Option<Uuid> = sqlx::query_scalar("SELECT registryid FROM images WHERE id=$1")
        .bind(image_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(association, None);
    let images = vec![image.clone()];
    assert!(
        store.load(platform, &images, &[]).await.unwrap().is_empty(),
        "registry access filters results in SQL"
    );
    let visible = store.load(platform, &images, &[registry.id]).await.unwrap();
    assert_eq!(visible[0].update.status, ImageUpdateStatus::UpdateAvailable);
    // A no-op pull must invalidate the old comparison without losing its fence.
    let pulled = citadel_platforms::RuntimeImageSummary {
        id: image.clone(),
        repo_tags: vec!["nginx:17".into()],
        ..Default::default()
    };
    citadel_adapters::persistence::postgres::platforms::images::PostgresImageMutationStore(
        pool.clone(),
    )
    .persist_pull(platform, None, &pulled)
    .await
    .unwrap();
    let visible = store.load(platform, &images, &[registry.id]).await.unwrap();
    assert_eq!(visible[0].update.status, ImageUpdateStatus::NotChecked);
    assert!(visible[0].update.last_attempt_at.is_none());
    assert!(visible[0].update.remote_config_digest.is_none());
    update.last_attempt_at = Some(now);
    assert!(
        !store
            .save(platform, &image, &registry, &update, true)
            .await
            .unwrap(),
        "a pre-pull check cannot resurrect old metadata"
    );
    let association: Option<Uuid> = sqlx::query_scalar("SELECT registryid FROM images WHERE id=$1")
        .bind(image_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(association, None);
    update.last_attempt_at = Some(Utc::now());
    assert!(
        store
            .save(platform, &image, &registry, &update, true)
            .await
            .unwrap()
    );
    let association: Option<Uuid> = sqlx::query_scalar("SELECT registryid FROM images WHERE id=$1")
        .bind(image_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(association, Some(registry.id));
    // An image losing its tag must neither expose its old status nor accept an
    // in-flight check completion for that tag.
    sqlx::query("UPDATE images SET tags='[]',registryid=NULL WHERE id=$1")
        .bind(image_id)
        .execute(&pool)
        .await
        .unwrap();
    update.last_attempt_at = Some(now + chrono::Duration::seconds(1));
    assert!(
        !store
            .save(platform, &image, &registry, &update, true)
            .await
            .unwrap()
    );
    assert!(
        store
            .load(platform, &images, &[registry.id])
            .await
            .unwrap()
            .is_empty()
    );
    let association: Option<Uuid> = sqlx::query_scalar("SELECT registryid FROM images WHERE id=$1")
        .bind(image_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(association, None);
    sqlx::query("DELETE FROM images WHERE id=$1")
        .bind(image_id)
        .execute(&pool)
        .await
        .unwrap();
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM imageupdateobservations WHERE imageid=$1")
            .bind(image_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn image_update_registry_association_requires_success_and_preserves_concurrent_choices() {
    use citadel_adapters::persistence::postgres::platforms::image_updates::PostgresImageUpdateStore;
    use citadel_platforms::image_update_store::ImageUpdateStore;
    use citadel_platforms::image_updates::{
        ImageTagUpdate, ImageUpdateRegistry, ImageUpdateStatus,
    };
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let registry = ImageUpdateRegistry {
        id: Uuid::from_u128(0x100),
        host: "docker.io".into(),
        revision: "one".into(),
    };
    let other = ImageUpdateRegistry {
        id: Uuid::now_v7(),
        ..registry.clone()
    };
    sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) SELECT $1,configuration,createdbyactorid,$1::text,registryhost,status FROM registries WHERE id=$2")
        .bind(other.id).bind(registry.id).execute(&pool).await.unwrap();
    let store = PostgresImageUpdateStore(pool.clone());
    for (status, remember, expected) in [
        (ImageUpdateStatus::UpToDate, false, false),
        (ImageUpdateStatus::CheckFailed, true, false),
        (ImageUpdateStatus::Unknown, true, false),
        (ImageUpdateStatus::NotChecked, true, false),
        (ImageUpdateStatus::NotApplicable, true, false),
        (ImageUpdateStatus::UpToDate, true, true),
        (ImageUpdateStatus::UpdateAvailable, true, true),
    ] {
        let id = Uuid::now_v7();
        let image = format!("sha256:{id}");
        sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags) VALUES($1,$2,'nginx',$3,now(),'[\"nginx:17\"]')")
            .bind(id).bind(&image).bind(platform).execute(&pool).await.unwrap();
        let now = Utc::now();
        let update = ImageTagUpdate {
            reference: "nginx:17".into(),
            registry_id: Some(registry.id),
            status,
            reason: None,
            last_attempt_at: Some(now),
            last_success_at: Some(now),
            local_config_digest: None,
            remote_config_digest: None,
            remote_manifest_digest: None,
            is_stale: false,
        };
        assert!(
            store
                .save(platform, &image, &registry, &update, remember)
                .await
                .unwrap()
        );
        let association: Option<Uuid> =
            sqlx::query_scalar("SELECT registryid FROM images WHERE id=$1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(association, expected.then_some(registry.id), "{status:?}");

        // Competing successful checks can both save observations, but only one
        // may associate an image; an established association must remain intact.
        let success = ImageTagUpdate {
            status: ImageUpdateStatus::UpToDate,
            last_attempt_at: Some(Utc::now()),
            ..update
        };
        let other_success = ImageTagUpdate {
            registry_id: Some(other.id),
            ..success.clone()
        };
        let (first, second) = tokio::join!(
            store.save(platform, &image, &registry, &success, true),
            store.save(platform, &image, &other, &other_success, true),
        );
        assert!(first.unwrap());
        assert!(second.unwrap());
        let (association, version): (Option<Uuid>, i64) =
            sqlx::query_as("SELECT registryid,rowversion FROM images WHERE id=$1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(version, 1, "only the first association changes the image");
        if expected {
            assert_eq!(association, Some(registry.id));
        } else {
            assert!(association == Some(registry.id) || association == Some(other.id));
        }
    }
}

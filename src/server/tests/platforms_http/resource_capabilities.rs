use super::*;
use citadel_primitives::ResourceType;
use citadel_server::realtime_groups::{Group, GroupReadPort};

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn docker_realtime_rows_preserve_platform_capabilities() {
    let f = fixture().await;
    let actor = super::lookup::subject(&f).await;
    let writer = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        actor.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,2,$3,$4,0)")
        .bind(Uuid::now_v7()).bind(writer.actor_id.value()).bind(f.platform_id).bind(ResourceType::Platform as i32)
        .execute(&f.pool).await.unwrap();
    let reader = super::realtime_groups::reader(&f);
    let groups = [
        Group::parse(&format!("containers:{}", f.platform_id)).unwrap(),
        Group::parse(&format!("docker-daemon:{}", f.platform_id)).unwrap(),
    ];
    let hub = citadel_server::realtime::RealtimeHub::new(
        8,
        Arc::new(citadel_server::metrics::Metrics::default()),
    );
    let mut changes = hub.subscribe();
    let docker_id: String =
        sqlx::query_scalar("SELECT dockercontainerid FROM containers WHERE platformid=$1 LIMIT 1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    hub.publish_runtime_change(f.platform_id, "container", "start", &docker_id);
    let change = changes.recv().await.unwrap();
    for (principal, writable, executable) in [
        (&actor, false, false),
        (&writer, true, false),
        (&f.administrator, true, true),
    ] {
        for group in &groups {
            for event in [None, Some(change.as_ref())] {
                let snapshot = reader.read(principal, group, event).await.unwrap();
                let mut checked = 0;
                for batch in &snapshot.rows {
                    if matches!(
                        batch.target,
                        "ContainerEventReceived" | "ImageEventReceived"
                    ) {
                        for row in &batch.rows {
                            assert_docker_capabilities(row, writable, executable);
                            checked += 1;
                        }
                    }
                }
                for event in &snapshot.events {
                    if matches!(event.target, "ContainersInfoUpdated" | "ContainersChanged") {
                        for row in event.arguments[0]["containers"].as_array().unwrap() {
                            assert_docker_capabilities(row, writable, executable);
                            checked += 1;
                        }
                    }
                }
                assert!(checked > 0);
            }
        }
    }
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(&f.docker_socket).unwrap();
}

fn assert_docker_capabilities(row: &Value, writable: bool, executable: bool) {
    assert_eq!(row["capabilities"]["canRead"], true);
    assert_eq!(row["capabilities"]["canWrite"], writable);
    assert_eq!(row["capabilities"]["canExecute"], executable);
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn git_and_backup_realtime_rows_include_actor_capabilities() {
    let f = fixture().await;
    let actor = super::lookup::subject(&f).await;
    let writer = super::lookup::subject(&f).await;
    let denied = super::lookup::subject(&f).await;
    let role = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO roles(id,name,roletype) VALUES($1,'realtime-resource-reader','Custom')",
    )
    .bind(role)
    .execute(&f.pool)
    .await
    .unwrap();
    for principal in [&actor, &writer] {
        sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
            .bind(principal.actor_id.value())
            .bind(role)
            .execute(&f.pool)
            .await
            .unwrap();
    }
    for kind in [
        ResourceType::GitRepository,
        ResourceType::BackupRepository,
        ResourceType::BackupPolicy,
    ] {
        sqlx::query("INSERT INTO permissions(id,roleid,resourcetype,permissionlevel,specificpermissions) VALUES($1,$2,$3,1,0)")
            .bind(Uuid::now_v7()).bind(role).bind(kind as i32).execute(&f.pool).await.unwrap();
    }
    let secret = Uuid::now_v7();
    let repository = Uuid::now_v7();
    let policy = Uuid::now_v7();
    let git = Uuid::now_v7();
    sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,'realtime-backup-password','InternalEncrypted')")
        .bind(secret).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO backuprepositories(id,name,normalizedname,passwordsecretid,spec,status,type,createdbyactorid) VALUES($1,'realtime-backups','realtime-backups',$2,$3,'Ready','FileSystem',$4)")
        .bind(repository).bind(secret)
        .bind(json!({"$type":"FileSystem","location":"Core","platformId":null,"path":"/fixture/backups"}))
        .bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO backuppolicies(id,name,normalizedname,backuprepositoryid,createdbyactorid,runasactorid,source) VALUES($1,'realtime-policy','realtime-policy',$2,$3,$3,'{\"$type\":\"CitadelSystem\"}')")
        .bind(policy).bind(repository).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO gitrepositories(id,name,url,defaultbranch,status,createdbyactorid) VALUES($1,'realtime-git','https://example.test/repo','main','Unknown',$2)")
        .bind(git).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();

    let reader = super::realtime_groups::reader(&f);
    for (kind, id, list, detail) in [
        (
            ResourceType::GitRepository,
            git,
            "git-repositories",
            "git-repo",
        ),
        (
            ResourceType::BackupRepository,
            repository,
            "backup-repositories",
            "backup-repository",
        ),
        (
            ResourceType::BackupPolicy,
            policy,
            "backup-policies",
            "backup-policy",
        ),
    ] {
        super::lookup::grant(&f, actor.actor_id.value(), kind, id, 0).await;
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,2,$3,$4,0)")
            .bind(Uuid::now_v7()).bind(writer.actor_id.value()).bind(id).bind(kind as i32)
            .execute(&f.pool).await.unwrap();
        let hub = citadel_server::realtime::RealtimeHub::new(
            8,
            Arc::new(citadel_server::metrics::Metrics::default()),
        );
        let mut changes = hub.subscribe();
        hub.publish_resource_change(
            match kind {
                ResourceType::GitRepository => "GitRepository",
                ResourceType::BackupRepository => "BackupRepository",
                _ => "BackupPolicy",
            },
            id,
            "updated",
        );
        let change = changes.recv().await.unwrap();
        for group_name in [list.to_string(), format!("{detail}:{id}")] {
            let group = Group::parse(&group_name).unwrap();
            for (principal, writable, executable) in [
                (&actor, false, false),
                (&writer, true, false),
                (&f.administrator, true, true),
            ] {
                for event in [None, Some(change.as_ref())] {
                    let snapshot = reader.read(principal, &group, event).await.unwrap();
                    let row = snapshot
                        .rows
                        .iter()
                        .flat_map(|batch| &batch.rows)
                        .find(|row| row["id"] == json!(id))
                        .unwrap();
                    assert_eq!(
                        row["capabilities"],
                        json!({"canRead":true,"canWrite":writable,"canExecute":executable}),
                        "{group_name}"
                    );
                }
            }
            assert!(
                reader.read(&denied, &group, None).await.is_err(),
                "{group_name}"
            );
        }
    }
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(&f.docker_socket).unwrap();
}

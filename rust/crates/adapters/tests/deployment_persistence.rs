use citadel_adapters::postgres::deployments::PostgresDeploymentRepository;
use citadel_database::MigrationRunner;
use citadel_deployments::{
    CreateDeployment, DeploymentError, DeploymentFilter, DeploymentImageInfo, DeploymentRepository,
    DeploymentSpec, DuplicateSource, FieldPatch, UpdateBehavior, UpdateDeploymentMetadata,
};
use citadel_domain::ActorId;
use citadel_identity::SYSTEM_ACTOR_ID;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE6_DATABASE_URL"]
async fn deployment_crud_duplicate_acl_and_delete_are_transactional() {
    let database_url = std::env::var("CITADEL_PHASE6_DATABASE_URL")
        .expect("CITADEL_PHASE6_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let store = PostgresDeploymentRepository::new(pool.clone());
    let actor = ActorId::new(SYSTEM_ACTOR_ID);
    let suffix = Uuid::now_v7().simple().to_string();
    let platform_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO platforms(
               id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,
               platformdescriptor,status,volumecount)
           VALUES($1,'unix:///var/run/docker-swarm.sock','Local',1,0,1048576,$2,0,
                  '{"$type":"Docker"}'::json,'Online',0)"#,
    )
    .bind(platform_id)
    .bind(format!("phase6-{suffix}"))
    .execute(&pool)
    .await
    .unwrap();
    let other_platform_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO platforms(
               id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,
               platformdescriptor,status,volumecount)
           VALUES($1,'unix:///var/run/other-docker.sock','Local',1,0,1048576,$2,0,
                  '{"$type":"Docker"}'::json,'Online',0)"#,
    )
    .bind(other_platform_id)
    .bind(format!("phase6-other-{suffix}"))
    .execute(&pool)
    .await
    .unwrap();
    let swarm_platform_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO platforms(
               id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,
               platformdescriptor,status,volumecount)
           VALUES($1,'unix:///var/run/docker.sock','Local',1,0,1048576,$2,0,
                  '{"$type":"DockerSwarm"}'::json,'Online',0)"#,
    )
    .bind(swarm_platform_id)
    .bind(format!("phase6-swarm-{suffix}"))
    .execute(&pool)
    .await
    .unwrap();
    let unsupported_name = format!("unsupported-{suffix}");
    let unsupported = store
        .create(
            actor,
            true,
            &CreateDeployment {
                name: unsupported_name.clone(),
                platform_id: swarm_platform_id,
                description: None,
                spec: local_spec(None),
                tag_ids: Vec::new(),
                duplicate_source: None,
            },
        )
        .await;
    assert!(matches!(unsupported, Err(DeploymentError::NotFound)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deployments WHERE name=$1")
            .bind(&unsupported_name)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let tag_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tags(id,name,normalizedname,color,createdbyactorid,createdat,updatedat) VALUES($1,$2,lower($2),'#112233',$3,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP)",
    )
    .bind(tag_id)
    .bind(format!("phase6-tag-{suffix}"))
    .bind(SYSTEM_ACTOR_ID)
    .execute(&pool)
    .await
    .unwrap();

    let created = store
        .create(
            actor,
            true,
            &CreateDeployment {
                name: format!("deployment-{suffix}"),
                platform_id,
                description: Some("created".to_owned()),
                spec: local_spec(Some("/srv/data:/data")),
                tag_ids: vec![tag_id],
                duplicate_source: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(created.status, "Created");
    assert_eq!(created.tags.len(), 1);
    for tag_filter in [created.tags[0].name.clone(), tag_id.to_string()] {
        let tagged = store
            .list_authorized(
                actor,
                true,
                &DeploymentFilter {
                    tags: vec![tag_filter],
                    platform_id: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].id, created.id);
    }
    assert!(
        store
            .list_authorized(
                actor,
                true,
                &DeploymentFilter {
                    tags: vec!["missing-tag".to_owned()],
                    platform_id: None,
                },
            )
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        created
            .auto_update_state
            .as_ref()
            .unwrap()
            .last_checked_at
            .to_rfc3339(),
        "0001-01-01T00:00:00+00:00"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT autoupdatestate_lastcheckedat::text FROM deployments WHERE id=$1"
        )
        .bind(created.id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "-infinity"
    );

    let image_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO images(id,containers,createdat,dockerimageid,name,platformid,size,tags) VALUES($1,0,CURRENT_TIMESTAMP,$2,$3,$4,1024,'[\"nginx:latest\"]'::json)",
    )
    .bind(image_id)
    .bind(format!("sha256:{suffix}"))
    .bind(format!("nginx-{suffix}"))
    .bind(platform_id)
    .execute(&pool)
    .await
    .unwrap();
    let expected_image_name = format!("nginx-{suffix}");
    let configured_local = store
        .create(
            actor,
            true,
            &CreateDeployment {
                name: format!("configured-local-{suffix}"),
                platform_id,
                description: None,
                spec: DeploymentSpec {
                    image: DeploymentImageInfo::Local {
                        image_id: image_id.to_string(),
                    },
                    ..local_spec(None)
                },
                tag_ids: Vec::new(),
                duplicate_source: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(configured_local.image_id, Some(image_id));
    assert_eq!(
        configured_local.image_name.as_deref(),
        Some(expected_image_name.as_str())
    );
    let unresolved_local = store
        .create(
            actor,
            true,
            &CreateDeployment {
                name: format!("unresolved-local-{suffix}"),
                platform_id,
                description: None,
                spec: local_spec(None),
                tag_ids: Vec::new(),
                duplicate_source: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(unresolved_local.image_id, None);
    assert_eq!(unresolved_local.image_name, None);
    let on_other_platform = store
        .create(
            actor,
            true,
            &CreateDeployment {
                name: format!("other-platform-{suffix}"),
                platform_id: other_platform_id,
                description: None,
                spec: local_spec(None),
                tag_ids: Vec::new(),
                duplicate_source: None,
            },
        )
        .await
        .unwrap();
    let platform_filtered = store
        .list_authorized(
            actor,
            true,
            &DeploymentFilter {
                tags: Vec::new(),
                platform_id: Some(other_platform_id),
            },
        )
        .await
        .unwrap();
    assert_eq!(platform_filtered.len(), 1);
    assert_eq!(platform_filtered[0].id, on_other_platform.id);

    sqlx::query(
        "INSERT INTO resourcebindings(id,kind,name,resourceid,scope,value) VALUES($1,'Variable','TOKEN',$2,'Deployment','secret-reference')",
    )
    .bind(Uuid::now_v7())
    .bind(created.id)
    .execute(&pool)
    .await
    .unwrap();

    let draft = store
        .duplicate_draft(actor, true, created.id)
        .await
        .unwrap();
    assert_eq!(draft.warnings[0].code, "HOST_BIND_MOUNT");
    let duplicate = store
        .create(
            actor,
            true,
            &CreateDeployment {
                name: draft.draft.name,
                platform_id,
                description: draft.draft.description,
                spec: draft.draft.spec,
                tag_ids: draft.draft.tag_ids,
                duplicate_source: Some(DuplicateSource {
                    resource_type: "Deployment".to_owned(),
                    resource_id: created.id,
                    resource_name: "untrusted-name".to_owned(),
                }),
            },
        )
        .await
        .unwrap();
    let copied_bindings: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resourcebindings WHERE scope='Deployment' AND resourceid=$1",
    )
    .bind(duplicate.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(copied_bindings, 1);
    assert!(
        store
            .duplicate_draft(actor, true, created.id)
            .await
            .unwrap()
            .draft
            .name
            .ends_with("-copy-2")
    );

    let next_spec = DeploymentSpec {
        ports: Some(vec!["8080:80".to_owned()]),
        ..duplicate.spec.clone()
    };
    let config_updated = store
        .update_config(actor, true, duplicate.id, duplicate.row_version, &next_spec)
        .await
        .unwrap();
    assert_eq!(config_updated.spec.ports, next_spec.ports);
    assert!(matches!(
        store
            .update_config(
                actor,
                true,
                duplicate.id,
                duplicate.row_version,
                &duplicate.spec,
            )
            .await,
        Err(DeploymentError::Conflict(_))
    ));

    let renamed = store
        .rename(actor, true, duplicate.id, &format!("renamed-{suffix}"))
        .await
        .unwrap();
    assert!(renamed.name.starts_with("renamed-"));
    let updated = store
        .update_metadata(
            actor,
            true,
            duplicate.id,
            &UpdateDeploymentMetadata {
                description: FieldPatch::Clear,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.description, None);

    let reader = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
        .bind(reader)
        .execute(&pool)
        .await
        .unwrap();
    let inaccessible_name = format!("inaccessible-{suffix}");
    let inaccessible = store
        .create(
            ActorId::new(reader),
            false,
            &CreateDeployment {
                name: inaccessible_name.clone(),
                platform_id,
                description: None,
                spec: local_spec(None),
                tag_ids: Vec::new(),
                duplicate_source: None,
            },
        )
        .await;
    assert!(matches!(inaccessible, Err(DeploymentError::NotFound)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deployments WHERE name=$1")
            .bind(&inaccessible_name)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    sqlx::query(
        "INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,1,0)",
    )
    .bind(Uuid::now_v7())
    .bind(reader)
    .bind(created.id)
    .execute(&pool)
    .await
    .unwrap();
    let authorized = store
        .list_authorized(ActorId::new(reader), false, &DeploymentFilter::default())
        .await
        .unwrap();
    assert_eq!(authorized.len(), 1);
    assert_eq!(authorized[0].id, created.id);
    assert!(!authorized[0].effective_permission.allows(<citadel_deployments::permissions::WriteDeployment as citadel_domain::PermissionPolicy>::REQUIREMENT));

    let team_member = Uuid::now_v7();
    let team_actor = Uuid::now_v7();
    let team_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User'),($2,TRUE,'Team')")
        .bind(team_member)
        .bind(team_actor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams(id,actorid,name) VALUES($1,$2,$3)")
        .bind(team_id)
        .bind(team_actor)
        .bind(format!("phase6-team-{suffix}"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorteammemberships(memberactorid,teamid) VALUES($1,$2)")
        .bind(team_member)
        .bind(team_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,1,0)",
    )
    .bind(Uuid::now_v7())
    .bind(team_actor)
    .bind(unresolved_local.id)
    .execute(&pool)
    .await
    .unwrap();
    let team_authorized = store
        .list_authorized(
            ActorId::new(team_member),
            false,
            &DeploymentFilter::default(),
        )
        .await
        .unwrap();
    assert_eq!(team_authorized.len(), 1);
    assert_eq!(team_authorized[0].id, unresolved_local.id);

    let processing_claims = store
        .claim_delete(actor, true, &[duplicate.id])
        .await
        .unwrap();
    assert!(matches!(
        store
            .rename(actor, true, duplicate.id, "blocked-rename")
            .await,
        Err(DeploymentError::Conflict(_))
    ));
    assert!(matches!(
        store
            .update_metadata(
                actor,
                true,
                duplicate.id,
                &UpdateDeploymentMetadata {
                    description: FieldPatch::Set("blocked".to_owned()),
                },
            )
            .await,
        Err(DeploymentError::Conflict(_))
    ));
    assert!(matches!(
        store
            .update_config(
                actor,
                true,
                duplicate.id,
                config_updated.row_version,
                &config_updated.spec,
            )
            .await,
        Err(DeploymentError::Conflict(_))
    ));
    store.release_delete(&processing_claims).await.unwrap();

    let claim_ids = [unresolved_local.id];
    let (first_claim, second_claim) = tokio::join!(
        store.claim_delete(actor, true, &claim_ids),
        store.claim_delete(actor, true, &claim_ids)
    );
    let concurrent_claims = match (first_claim, second_claim) {
        (Ok(claims), Err(DeploymentError::Conflict(_)))
        | (Err(DeploymentError::Conflict(_)), Ok(claims)) => claims,
        result => panic!("expected one deletion claim and one conflict, got {result:?}"),
    };
    store.release_delete(&concurrent_claims).await.unwrap();

    let concurrent_name = format!("concurrent-{suffix}");
    let first_input = CreateDeployment {
        name: concurrent_name.clone(),
        platform_id,
        description: None,
        spec: local_spec(None),
        tag_ids: Vec::new(),
        duplicate_source: None,
    };
    let second_input = first_input.clone();
    let (first_create, second_create) = tokio::join!(
        store.create(actor, true, &first_input),
        store.create(actor, true, &second_input)
    );
    let concurrently_created = match (first_create, second_create) {
        (Ok(created), Err(DeploymentError::Conflict(_)))
        | (Err(DeploymentError::Conflict(_)), Ok(created)) => created,
        result => panic!("expected one create and one name conflict, got {result:?}"),
    };

    let ids = vec![
        created.id,
        duplicate.id,
        configured_local.id,
        unresolved_local.id,
        on_other_platform.id,
        concurrently_created.id,
    ];
    sqlx::query("UPDATE resourceaccesses SET permissionlevel=4 WHERE actorid=$1")
        .bind(reader)
        .execute(&pool)
        .await
        .unwrap();
    let unauthorized_batch = store.claim_delete(ActorId::new(reader), false, &ids).await;
    assert!(matches!(
        unauthorized_batch,
        Err(DeploymentError::Forbidden)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM deployments WHERE id=ANY($1::uuid[]) AND controlstate='Processing'"
        )
        .bind(&ids)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let claims = store.claim_delete(actor, true, &ids).await.unwrap();
    assert_eq!(claims.len(), ids.len());
    store.release_delete(&claims).await.unwrap();
    let claims = store.claim_delete(actor, true, &ids).await.unwrap();
    store.complete_delete(actor, &claims).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deployments WHERE id=ANY($1::uuid[])")
            .bind(&ids)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM resourcebindings WHERE resourceid=ANY($1::uuid[])"
        )
        .bind(&ids)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM activityevents WHERE resourceid=ANY($1::uuid[]) AND eventtype='DeploymentDeleted'")
            .bind(&ids)
            .fetch_one(&pool)
            .await
            .unwrap(),
        ids.len() as i64
    );

    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(other_platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(swarm_platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tags WHERE id=$1")
        .bind(tag_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=$1")
        .bind(reader)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actorteammemberships WHERE teamid=$1")
        .bind(team_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM teams WHERE id=$1")
        .bind(team_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM actors WHERE id=ANY($1::uuid[])")
        .bind(vec![team_member, team_actor])
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

fn local_spec(volume: Option<&str>) -> DeploymentSpec {
    DeploymentSpec {
        image: DeploymentImageInfo::Local {
            image_id: "sha256:test".to_owned(),
        },
        update_behavior: UpdateBehavior::Disabled,
        life_cycle_spec: None,
        resource_spec: None,
        labels: None,
        ports: None,
        volumes: volume.map(|value| vec![value.to_owned()]),
        networks: None,
        command: None,
        environment_variables: None,
    }
}

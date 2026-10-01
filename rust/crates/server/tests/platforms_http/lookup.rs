use super::*;
use citadel_primitives::{ResourceType, SpecificPermission};

pub(super) async fn subject(f: &Fixture) -> ActorPrincipal {
    let actor = Uuid::now_v7();
    let user = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(actor)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO users(id,actorid,name,createdbyactorid,email) VALUES($1,$2,$3,$2,$4)")
        .bind(user)
        .bind(actor)
        .bind(format!("lookup-{user}"))
        .bind(format!("{user}@lookup.test"))
        .execute(&f.pool)
        .await
        .unwrap();
    ActorPrincipal {
        subject_id: user,
        actor_id: ActorId::new(actor),
        name: "lookup reader".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    }
}

pub(super) async fn grant(f: &Fixture, actor: Uuid, kind: ResourceType, id: Uuid, specific: i32) {
    use citadel_adapters::persistence::postgres::identity::{
        teams::repository::PostgresTeamRepository, users::repository::PostgresUserRepository,
    };
    use citadel_identity::{ResourceAccessInput, TeamRepository, UserRepository};
    let access = ResourceAccessInput {
        resource_type: kind,
        resource_id: id,
        permission_level: citadel_primitives::PermissionLevel::Read,
        specific_permissions: SpecificPermission::ALL
            .into_iter()
            .filter(|p| specific & (*p as i32) != 0)
            .collect(),
    };
    let user: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE actorid=$1")
        .bind(actor)
        .fetch_optional(&f.pool)
        .await
        .unwrap();
    if let Some(user) = user {
        PostgresUserRepository::new(f.pool.clone())
            .add_resource_access(user, &access, f.administrator.actor_id, Utc::now(), true)
            .await
            .unwrap();
    } else {
        let team: Uuid = sqlx::query_scalar("SELECT id FROM teams WHERE actorid=$1")
            .bind(actor)
            .fetch_one(&f.pool)
            .await
            .unwrap();
        PostgresTeamRepository::new(f.pool.clone())
            .add_resource_access(team, &access, f.administrator.actor_id, Utc::now(), true)
            .await
            .unwrap();
    }
}

async fn lookup(f: &Fixture, principal: &ActorPrincipal, query: &str) -> Value {
    let response = send(
        f,
        &format!("/api/v1/lookup?{query}"),
        Some(principal.clone()),
    )
    .await;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "{}: {}",
        query,
        String::from_utf8_lossy(&bytes)
    );
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    for row in value.as_array().expect("lookup returns a bare array") {
        assert_eq!(
            row.as_object().unwrap().len(),
            3,
            "only id/name/group are exposed"
        );
    }
    value
}

fn ids(rows: &Value) -> Vec<Uuid> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|row| Uuid::parse_str(row["id"].as_str().unwrap()).unwrap())
        .collect()
}

async fn close(f: Fixture) {
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn lookup_validation_admin_precedence_and_all_add_targets_match_dotnet() {
    let f = fixture().await;
    let reader = subject(&f).await;
    assert_eq!(
        send(&f, "/api/v1/lookup?TargetResourceType=Platform", None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    for query in [
        "",
        "TargetResourceType=unknown",
        "TargetResourceType=Platform&SourceResourceType=invalid",
        "TargetResourceType=Platform&PlatformId=invalid",
        "TargetResourceType=Image",
        "TargetResourceType=Registry&SourceResourceType=Volume",
        "TargetResourceType=Image&SourceResourceType=Deployment",
    ] {
        assert_eq!(
            send(&f, &format!("/api/v1/lookup?{query}"), Some(reader.clone()))
                .await
                .status(),
            StatusCode::BAD_REQUEST,
            "{query}"
        );
    }
    let missing_id = Uuid::now_v7();
    for query in [
        format!("TargetResourceType=Platform&SourceResourceId={missing_id}"),
        format!(
            "TargetResourceType=Platform&SourceResourceId={missing_id}&SourceResourceType=invalid"
        ),
    ] {
        assert_eq!(
            send(&f, &format!("/api/v1/lookup?{query}"), Some(reader.clone()))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for target in [
        "User",
        "Team",
        "Role",
        "ServiceAccount",
        "OidcProvider",
        "License",
    ] {
        for source in ["", "&SourceResourceType=Deployment"] {
            assert_eq!(
                send(
                    &f,
                    &format!("/api/v1/lookup?TargetResourceType={target}{source}"),
                    Some(reader.clone())
                )
                .await
                .status(),
                StatusCode::FORBIDDEN
            );
        }
    }
    assert_eq!(send(&f,&format!("/api/v1/lookup?TargetResourceType=Registry&SourceResourceType=Deployment&SourceResourceId={missing_id}"),Some(reader.clone())).await.status(),StatusCode::NOT_FOUND);
    assert_eq!(
        send(
            &f,
            &format!(
                "/api/v1/lookup?TargetResourceType=Image&PlatformId={}",
                f.platform_id
            ),
            Some(reader.clone())
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    for target in [
        "Platform",
        "Deployment",
        "Stack",
        "Registry",
        "GitRepository",
        "AutomationAction",
        "Alert",
        "User",
        "UserActor",
        "RunAsActor",
        "Team",
        "Role",
        "ResourceBinding",
        "OidcProvider",
        "License",
        "BackupRepository",
        "BackupPolicy",
        "Build",
        "BuildAgentPool",
        "SwarmService",
        "ServiceAccount",
    ] {
        lookup(
            &f,
            &f.administrator,
            &format!("TargetResourceType={target}"),
        )
        .await;
    }
    let own = lookup(&f, &reader, "targetResourceType=useractor").await;
    assert_eq!(ids(&own), vec![reader.actor_id.value()]);
    let license = lookup(&f, &f.administrator, "TargetResourceType=License").await;
    let instance_id: Uuid =
        sqlx::query_scalar("SELECT instanceid FROM citadelinstanceidentity WHERE id=1")
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(ids(&license), vec![instance_id]);
    assert!(
        lookup(&f, &reader, "TargetResourceType=BackupRepository")
            .await
            .as_array()
            .unwrap()
            .is_empty()
    );
    close(f).await;
}

async fn registry(f: &Fixture) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO registries(id,name,registryhost,status,configuration,createdbyactorid) VALUES($1,$2,$2,'Active','{\"password\":\"must-not-leak\"}',$3)").bind(id).bind(format!("lookup-registry-{id}")).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    id
}
async fn deployment(f: &Fixture, registry_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$2,$3,$4,'Created',$5)").bind(id).bind(format!("lookup-deployment-{id}")).bind(f.platform_id).bind(json!({"image":{"$type":"External","registryId":registry_id,"image":"nginx"}})).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    id
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn lookup_preserves_linked_targets_without_granting_unrelated_resource_access() {
    let f = fixture().await;
    let reader = subject(&f).await;
    let linked = registry(&f).await;
    let visible = registry(&f).await;
    let hidden = registry(&f).await;
    let d = deployment(&f, linked).await;
    let hidden_deployment = deployment(&f, hidden).await;
    let query =
        format!("TargetResourceType=Registry&SourceResourceType=Deployment&SourceResourceId={d}");
    assert_eq!(
        send(&f, &format!("/api/v1/lookup?{query}"), Some(reader.clone()))
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    grant(&f, reader.actor_id.value(), ResourceType::Deployment, d, 0).await;
    assert_eq!(ids(&lookup(&f, &reader, &query).await), vec![linked]);
    grant(
        &f,
        reader.actor_id.value(),
        ResourceType::Registry,
        visible,
        0,
    )
    .await;
    grant(
        &f,
        reader.actor_id.value(),
        ResourceType::Registry,
        linked,
        0,
    )
    .await;
    let rows = lookup(&f, &reader, &query).await;
    assert_eq!(ids(&rows).len(), 2);
    assert!(ids(&rows).contains(&visible));
    assert!(!ids(&rows).contains(&hidden));
    assert_eq!(
        ids(&lookup(
            &f,
            &reader,
            &format!(
                "TargetResourceType=Platform&SourceResourceType=Deployment&SourceResourceId={d}"
            )
        )
        .await),
        vec![f.platform_id]
    );
    let images = lookup(
        &f,
        &reader,
        &format!("TargetResourceType=Image&SourceResourceType=Deployment&SourceResourceId={d}"),
    )
    .await;
    assert_eq!(images.as_array().unwrap().len(), 1);
    assert!(
        lookup(&f, &reader, "TargetResourceType=Platform")
            .await
            .as_array()
            .unwrap()
            .is_empty(),
        "linked name lookup must not grant Platform access"
    );
    grant(
        &f,
        reader.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let deployments = lookup(
        &f,
        &reader,
        &format!(
            "TargetResourceType=Deployment&SourceResourceType=Platform&SourceResourceId={}",
            f.platform_id
        ),
    )
    .await;
    assert_eq!(ids(&deployments).len(), 2);
    assert!(ids(&deployments).contains(&hidden_deployment));
    for (target, name) in [("Network", "frontend"), ("Volume", "data")] {
        let rows = lookup(
            &f,
            &reader,
            &format!(
                "TargetResourceType={target}&SourceResourceType=Platform&SourceResourceId={}",
                f.platform_id
            ),
        )
        .await;
        assert_eq!(rows[0]["id"], Uuid::nil().to_string());
        assert_eq!(rows[0]["name"], name);
    }
    close(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn lookup_stack_links_and_effective_bindings_match_dotnet_without_secret_values() {
    let f = fixture().await;
    let reader = subject(&f).await;
    let registry = registry(&f).await;
    let git = Uuid::now_v7();
    sqlx::query("INSERT INTO gitrepositories(id,name,url,defaultbranch,status,createdbyactorid) VALUES($1,$2,'https://example.test/repo','main','Unknown',$3)").bind(git).bind(format!("git-{git}")).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,stacksource,driftpolicy,stackupdatestate) VALUES($1,$2,$3,'Git','{}','{}')").bind(stack).bind(format!("stack-{stack}")).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES($1,$2,$3,$4,$5,'Created','1')").bind(release).bind(stack).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).bind(json!({"registryId":registry,"gitRepoId":git})).execute(&f.pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&f.pool)
        .await
        .unwrap();
    grant(&f, reader.actor_id.value(), ResourceType::Stack, stack, 0).await;
    for (target, id) in [
        ("Registry", registry),
        ("GitRepository", git),
        ("Platform", f.platform_id),
    ] {
        assert_eq!(
            ids(&lookup(
                &f,
                &reader,
                &format!(
                    "TargetResourceType={target}&SourceResourceType=Stack&SourceResourceId={stack}"
                )
            )
            .await),
            vec![id]
        );
    }
    let binding_name = format!("binding_{stack}");
    let global = Uuid::now_v7();
    let local = Uuid::now_v7();
    for (id, scope, resource_id) in [(global, "Global", None), (local, "Stack", Some(stack))] {
        sqlx::query("INSERT INTO resourcebindings(id,name,scope,resourceid,kind,value) VALUES($1,$2,$3,$4,'Variable','never-send-this-value')").bind(id).bind(&binding_name).bind(scope).bind(resource_id).execute(&f.pool).await.unwrap();
    }
    let rows = lookup(
        &f,
        &reader,
        &format!(
            "TargetResourceType=ResourceBinding&SourceResourceType=Stack&SourceResourceId={stack}"
        ),
    )
    .await;
    assert!(ids(&rows).contains(&local));
    assert!(!ids(&rows).contains(&global));
    assert!(!rows.to_string().contains("never-send-this-value"));
    assert!(
        ids(&lookup(
            &f,
            &reader,
            "TargetResourceType=ResourceBinding&SourceResourceType=Stack"
        )
        .await)
        .contains(&global)
    );
    close(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn run_as_lookup_requires_use_permission_and_license_and_returns_actor_ids() {
    let mut f = fixture().await;
    let reader = subject(&f).await;
    let account = Uuid::now_v7();
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,type,isenabled) VALUES($1,'ServiceAccount',true)")
        .bind(actor)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO serviceaccounts(id,actorid,name,createdat,updatedat,createdbyactorid) VALUES($1,$2,$3,now(),now(),$4)").bind(account).bind(actor).bind(format!("service-{account}")).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    grant(
        &f,
        reader.actor_id.value(),
        ResourceType::ServiceAccount,
        account,
        0,
    )
    .await;
    assert_eq!(
        ids(&lookup(&f, &reader, "TargetResourceType=RunAsActor").await),
        vec![reader.actor_id.value()]
    );
    sqlx::query(
        "UPDATE resourceaccesses SET specificpermissions=$1 WHERE actorid=$2 AND resourceid=$3",
    )
    .bind(SpecificPermission::Use as i32)
    .bind(reader.actor_id.value())
    .bind(account)
    .execute(&f.pool)
    .await
    .unwrap();
    let rows = lookup(&f, &reader, "TargetResourceType=RunAsActor").await;
    assert_eq!(rows[0]["id"], actor.to_string());
    assert_eq!(rows[0]["group"], "Service Accounts");
    f.lookup_state.entitlements = Arc::new(StaticEntitlementService::new(false));
    f.app = citadel_server::api::routes::lookup::router(f.lookup_state.clone());
    assert_eq!(
        ids(&lookup(&f, &reader, "TargetResourceType=RunAsActor").await),
        vec![reader.actor_id.value()]
    );
    f.lookup_state.entitlements = Arc::new(StaticEntitlementService::new(true));
    f.app = citadel_server::api::routes::lookup::router(f.lookup_state.clone());
    sqlx::query("UPDATE actors SET isenabled=false WHERE id=$1")
        .bind(actor)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        ids(&lookup(&f, &reader, "TargetResourceType=RunAsActor").await),
        vec![reader.actor_id.value()]
    );
    close(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn lookup_honors_team_roles_and_locks_platform_orchestration_type() {
    let f = fixture().await;
    let reader = subject(&f).await;
    let team = Uuid::now_v7();
    let actor = Uuid::now_v7();
    let role = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,type,isenabled) VALUES($1,'Team',true)")
        .bind(actor)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams(id,actorid,name) VALUES($1,$2,$3)")
        .bind(team)
        .bind(actor)
        .bind(format!("team-{team}"))
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorteammemberships(teamid,memberactorid) VALUES($1,$2)")
        .bind(team)
        .bind(reader.actor_id.value())
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO roles(id,name,roletype) VALUES($1,$2,'Custom')")
        .bind(role)
        .bind(format!("role-{role}"))
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(actor)
        .bind(role)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO permissions(id,roleid,resourcetype,permissionlevel,specificpermissions) VALUES($1,$2,$3,1,0)")
        .bind(Uuid::now_v7()).bind(role).bind(ResourceType::Registry as i32).execute(&f.pool).await.unwrap();
    let registry = registry(&f).await;
    assert!(
        ids(&lookup(
            &f,
            &reader,
            "TargetResourceType=Registry&SourceResourceType=Image"
        )
        .await)
        .contains(&registry)
    );
    sqlx::query("UPDATE actors SET isenabled=false WHERE id=$1")
        .bind(actor)
        .execute(&f.pool)
        .await
        .unwrap();
    assert!(ids(&lookup(&f, &reader, "TargetResourceType=Registry").await).is_empty());
    sqlx::query("UPDATE actors SET isenabled=true WHERE id=$1")
        .bind(actor)
        .execute(&f.pool)
        .await
        .unwrap();

    let d = deployment(&f, registry).await;
    grant(&f, actor, ResourceType::Deployment, d, 0).await;
    let standalone = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,name,connectortype,platformdescriptor,status,cpucount,imagecount,memtotal,networkcount,volumecount) VALUES($1,$2,$3,'Local','{\"$type\":\"Docker\"}','Online',0,0,0,0,0)")
        .bind(standalone).bind(format!("unix:///lookup/{standalone}.sock")).bind(format!("standalone-{standalone}")).execute(&f.pool).await.unwrap();
    for id in [standalone, f.platform_id] {
        grant(&f, actor, ResourceType::Platform, id, 0).await;
    }
    assert_eq!(
        ids(&lookup(
            &f,
            &reader,
            "TargetResourceType=Platform&SourceResourceType=Deployment"
        )
        .await),
        vec![standalone]
    );
    assert_eq!(
        ids(&lookup(
            &f,
            &reader,
            &format!(
                "TargetResourceType=Platform&SourceResourceType=Deployment&SourceResourceId={d}"
            )
        )
        .await),
        vec![f.platform_id]
    );
    assert_eq!(
        ids(&lookup(
            &f,
            &reader,
            "TargetResourceType=Platform&SourceResourceType=Stack"
        )
        .await)
        .len(),
        2
    );
    for target in ["Team", "Role"] {
        let query = format!(
            "TargetResourceType={target}&SourceResourceType=User&SourceResourceId={}",
            reader.subject_id
        );
        assert_eq!(
            send(&f, &format!("/api/v1/lookup?{query}"), Some(reader.clone()))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        let rows = lookup(&f, &f.administrator, &query).await;
        assert!(ids(&rows).contains(&if target == "Team" { team } else { role }));
    }
    close(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn lookup_deployment_and_swarm_service_bindings_are_scoped_and_read_authorized() {
    let f = fixture().await;
    let reader = subject(&f).await;
    let registry = registry(&f).await;
    let deployment = deployment(&f, registry).await;
    let service = Uuid::now_v7();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,desiredspechash,health,spec,synchronizationstate,updatedat) VALUES($1,$2,$2,$3,$4,'hash','Unknown','{}','Unknown',now())")
        .bind(service).bind(format!("service-{service}")).bind(f.platform_id).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    let binding_name = format!("binding_{service}");
    let global = Uuid::now_v7();
    sqlx::query("INSERT INTO resourcebindings(id,name,scope,kind,value) VALUES($1,$2,'Global','Variable','private')")
        .bind(global).bind(&binding_name).execute(&f.pool).await.unwrap();
    for (source, kind, id) in [
        ("Deployment", ResourceType::Deployment, deployment),
        ("SwarmService", ResourceType::SwarmService, service),
    ] {
        let local = Uuid::now_v7();
        sqlx::query("INSERT INTO resourcebindings(id,name,scope,resourceid,kind,value) VALUES($1,$2,$3,$4,'Variable','private-local')")
            .bind(local).bind(binding_name.to_uppercase()).bind(source).bind(id).execute(&f.pool).await.unwrap();
        let query = format!(
            "TargetResourceType=ResourceBinding&SourceResourceType={source}&SourceResourceId={id}"
        );
        assert_eq!(
            send(&f, &format!("/api/v1/lookup?{query}"), Some(reader.clone()))
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
        grant(&f, reader.actor_id.value(), kind, id, 0).await;
        let rows = lookup(&f, &reader, &query).await;
        let matching: Vec<_> = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["name"]
                    .as_str()
                    .unwrap()
                    .eq_ignore_ascii_case(&binding_name)
            })
            .collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0]["id"], local.to_string());
        assert!(!ids(&rows).contains(&global));
        assert!(!rows.to_string().contains("private"));
    }
    close(f).await;
}

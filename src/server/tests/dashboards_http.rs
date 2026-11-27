use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use citadel_adapters::persistence::postgres::{
    dashboards::PostgresDashboardRepository,
    identity::authentication::store::StaticEntitlementService,
};
use citadel_identity::{ActorPrincipal, AuthenticatedPrincipalType};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType};
use citadel_server::api::routes::dashboards::{DashboardsHttpState, router};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

async fn actor(pool: &sqlx::PgPool, admin: bool) -> ActorPrincipal {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,type,isenabled) VALUES($1,'User',true)")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    ActorPrincipal {
        actor_id: ActorId::new(id),
        subject_id: id,
        name: format!("reader-{id}"),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: if admin { vec!["Admin".into()] } else { vec![] },
    }
}

async fn send(
    app: &Router,
    who: &ActorPrincipal,
    method: Method,
    path: &str,
    body: Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(format!("/api/v1/{path}"))
                .header("content-type", "application/json")
                .extension(who.clone())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn call(
    app: &Router,
    who: &ActorPrincipal,
    method: Method,
    path: &str,
    body: Value,
    status: StatusCode,
) -> Value {
    let response = send(app, who, method, path, body).await;
    let actual = response.status();
    let bytes = to_bytes(response.into_body(), 200_000).await.unwrap();
    assert_eq!(
        actual,
        status,
        "{path}: {}",
        String::from_utf8_lossy(&bytes)
    );
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

#[tokio::test]
#[ignore = "requires a disposable CITADEL_DASHBOARD_DATABASE_URL"]
async fn dashboard_visibility_revisions_and_defaults_follow_the_viewer() {
    let db = std::env::var("CITADEL_DASHBOARD_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&db)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&db).await.unwrap();
    let repository = Arc::new(PostgresDashboardRepository::new(pool.clone()));
    let app = |team| {
        router(DashboardsHttpState {
            dashboards: repository.clone(),
            entitlements: Arc::new(StaticEntitlementService::new(team)),
        })
    };
    let (community, team) = (app(false), app(true));
    let owner = actor(&pool, false).await;
    let admin = actor(&pool, true).await;
    let other = actor(&pool, false).await;
    for (actor, role) in [(&other, "Viewer"), (&owner, "Operator")] {
        let grant = sqlx::query(
            "INSERT INTO actorroles(actorid, roleid) SELECT $1,id FROM roles WHERE name=$2",
        )
        .bind(actor.actor_id.value())
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(grant.rows_affected(), 1);
    }
    let examples = call(
        &community,
        &other,
        Method::GET,
        "dashboards?search=Overview",
        Value::Null,
        StatusCode::OK,
    )
    .await;
    let example = &examples["dashboards"][0];
    assert_eq!(example["name"], "Overview");
    assert_eq!(example["visibility"], "InstanceShared");
    assert_eq!(example["canEdit"], false);
    assert_eq!(example["canDelete"], false);
    let example_id = example["id"].as_str().unwrap();
    let example_path = format!("dashboards/{example_id}");
    let saved_example = call(
        &community,
        &admin,
        Method::GET,
        &example_path,
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(saved_example["summary"]["canEdit"], true);
    assert_eq!(saved_example["summary"]["canDelete"], true);
    let mut example_configuration = saved_example["configuration"].clone();
    let kinds: Vec<_> = example_configuration["widgets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|widget| widget["settings"]["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        [
            "FleetSummary",
            "NeedsAttention",
            "ResourceState",
            "RecentActivity"
        ]
    );
    example_configuration["rangeMinutes"] = json!(360);
    let updated_example = call(
        &community,
        &admin,
        Method::PUT,
        &example_path,
        json!({"revision":1,"name":"Edited Overview","configuration":example_configuration}),
        StatusCode::OK,
    )
    .await;
    assert_eq!(updated_example["summary"]["id"], json!(example_id));
    assert_eq!(updated_example["summary"]["name"], "Edited Overview");
    assert_eq!(updated_example["summary"]["revision"], 2);
    call(
        &community,
        &admin,
        Method::PUT,
        "instance/dashboard-default",
        json!({"dashboardId":example_id}),
        StatusCode::NO_CONTENT,
    )
    .await;
    call(
        &community,
        &admin,
        Method::DELETE,
        &format!("{example_path}?revision=2"),
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        citadel_database::MigrationRunner::migrate(&db)
            .await
            .unwrap()
            .applied,
        0
    );
    assert_eq!(
        call(
            &community,
            &other,
            Method::GET,
            "profile/dashboard-default",
            Value::Null,
            StatusCode::OK
        )
        .await,
        json!({"dashboardId":null})
    );

    let private = Uuid::now_v7();
    let shared = Uuid::now_v7();
    let restricted = Uuid::now_v7();
    let config = json!({"schemaVersion":2,"description":"**On call**","rangeMinutes":60,"tags":[],"widgets":[]});
    let board = |id, visibility| json!({"id":id,"name":format!("System {visibility}"),"visibility":visibility,"configuration":config});
    let private_path = format!("dashboards/{private}");
    let restricted_path = format!("dashboards/{restricted}");
    let saved = call(
        &community,
        &owner,
        Method::POST,
        "dashboards",
        board(private, "Private"),
        StatusCode::OK,
    )
    .await;
    assert_eq!(saved["configuration"]["description"], "**On call**");
    assert_eq!(saved["summary"]["revision"], 1);
    let container_pin = json!({"kind":"Container","id":Uuid::now_v7(),"platformId":Uuid::now_v7()});
    for settings in [
        json!({"kind":"Watchlist","pins":[container_pin]}),
        json!({"kind":"Metrics","source":container_pin,"metric":"Cpu","presentation":"Area","color":"Blue","legend":false}),
        json!({"kind":"ResourceSummary"}),
        json!({"kind":"Platforms","limit":5}),
        json!({"kind":"Workloads","resourceKind":"Deployment","limit":5}),
    ] {
        let mut retired = config.clone();
        retired["widgets"] = json!([{"id":Uuid::now_v7(),"title":"Old widget","layout":{"x":0,"y":0,"w":6,"h":4},"settings":settings}]);
        call(
            &community,
            &owner,
            Method::PUT,
            &private_path,
            json!({"revision":1,"name":"System Private","configuration":retired}),
            StatusCode::BAD_REQUEST,
        )
        .await;
    }
    assert_eq!(
        call(
            &community,
            &owner,
            Method::GET,
            &private_path,
            Value::Null,
            StatusCode::OK
        )
        .await,
        saved
    );
    assert_eq!(
        call(
            &community,
            &owner,
            Method::POST,
            "dashboards",
            board(private, "Private"),
            StatusCode::OK
        )
        .await,
        saved
    );
    for viewer in [&other, &admin] {
        call(
            &community,
            viewer,
            Method::GET,
            &private_path,
            Value::Null,
            StatusCode::NOT_FOUND,
        )
        .await;
    }
    call(
        &community,
        &other,
        Method::POST,
        "dashboards",
        board(shared, "InstanceShared"),
        StatusCode::FORBIDDEN,
    )
    .await;
    call(
        &community,
        &admin,
        Method::POST,
        "dashboards",
        board(shared, "InstanceShared"),
        StatusCode::OK,
    )
    .await;
    let denied = call(
        &community,
        &admin,
        Method::POST,
        "dashboards",
        board(restricted, "Restricted"),
        StatusCode::FORBIDDEN,
    )
    .await;
    assert_eq!(denied["capability"], "CustomAccessControl");
    call(
        &team,
        &admin,
        Method::POST,
        "dashboards",
        board(restricted, "Restricted"),
        StatusCode::OK,
    )
    .await;
    let picker = call(
        &community,
        &other,
        Method::GET,
        "dashboards",
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(picker["dashboards"].as_array().unwrap().len(), 1);
    assert_eq!(picker["dashboards"][0]["id"], json!(shared));
    assert!(picker["dashboards"][0].get("configuration").is_none());
    // A private name is reserved instance-wide even for someone who cannot read the board.
    let mut duplicate = board(Uuid::now_v7(), "Private");
    duplicate["name"] = json!("  system private  ");
    let rejected = call(
        &community,
        &other,
        Method::POST,
        "dashboards",
        duplicate,
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_eq!(
        rejected["detail"],
        "That dashboard name is already in use. Choose a different name."
    );
    call(
        &community,
        &other,
        Method::GET,
        &restricted_path,
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourceid,resourcetype,permissionlevel,specificpermissions) VALUES ($1,$2,$3,$4,$5,0)")
        .bind(Uuid::now_v7()).bind(other.actor_id.value()).bind(restricted)
        .bind(ResourceType::Dashboard as i32).bind(PermissionLevel::Write as i32).execute(&pool).await.unwrap();
    call(
        &community,
        &owner,
        Method::GET,
        &restricted_path,
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    let rejected = call(
        &community,
        &other,
        Method::PUT,
        &restricted_path,
        json!({"revision":1,"name":"SYSTEM INSTANCESHARED","configuration":config}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_eq!(
        rejected["detail"],
        "That dashboard name is already in use. Choose a different name."
    );
    let mut patch = json!({"name":"Updated","revision":1,"configuration":config});
    patch["visibility"] = json!("InstanceShared");
    call(
        &community,
        &other,
        Method::PUT,
        &restricted_path,
        patch.clone(),
        StatusCode::BAD_REQUEST,
    )
    .await;
    patch.as_object_mut().unwrap().remove("visibility");
    let updated = call(
        &community,
        &other,
        Method::PUT,
        &restricted_path,
        patch.clone(),
        StatusCode::OK,
    )
    .await;
    assert_eq!(updated["summary"]["revision"], 2);
    call(
        &community,
        &admin,
        Method::PUT,
        &restricted_path,
        patch,
        StatusCode::CONFLICT,
    )
    .await;
    let concurrent = |name| json!({"name":name,"revision":2,"configuration":config});
    let (first, second) = tokio::join!(
        send(
            &community,
            &admin,
            Method::PUT,
            &restricted_path,
            concurrent("Admin change")
        ),
        send(
            &community,
            &other,
            Method::PUT,
            &restricted_path,
            concurrent("Writer change")
        ),
    );
    let mut statuses = [first.status(), second.status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    // An older client may still read a newer definition, but saving must preserve it.
    sqlx::query("UPDATE dashboards SET configuration=jsonb_set(configuration,'{schemaVersion}','3') WHERE id=$1")
        .bind(restricted).execute(&pool).await.unwrap();
    call(
        &community,
        &other,
        Method::PUT,
        &restricted_path,
        json!({"name":"Old editor","revision":3,"configuration":config}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    let newer = call(
        &community,
        &other,
        Method::GET,
        &restricted_path,
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(newer["configuration"]["schemaVersion"], 3);
    assert_eq!(newer["summary"]["revision"], 3);
    call(
        &community,
        &other,
        Method::DELETE,
        &format!("{restricted_path}?revision=2"),
        Value::Null,
        StatusCode::FORBIDDEN,
    )
    .await;
    call(
        &community,
        &admin,
        Method::PUT,
        "instance/dashboard-default",
        json!({"dashboardId":shared}),
        StatusCode::NO_CONTENT,
    )
    .await;
    call(
        &community,
        &owner,
        Method::PUT,
        "instance/dashboard-default",
        json!({"dashboardId":private}),
        StatusCode::FORBIDDEN,
    )
    .await;
    call(
        &community,
        &owner,
        Method::PUT,
        "profile/dashboard-default",
        json!({"dashboardId":private}),
        StatusCode::NO_CONTENT,
    )
    .await;
    let default = |id| json!({"dashboardId":id});
    assert_eq!(
        call(
            &community,
            &owner,
            Method::GET,
            "profile/dashboard-default",
            Value::Null,
            StatusCode::OK
        )
        .await,
        default(private)
    );
    call(
        &community,
        &owner,
        Method::DELETE,
        &format!("{private_path}?revision=1"),
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    assert_eq!(
        call(
            &community,
            &owner,
            Method::GET,
            "profile/dashboard-default",
            Value::Null,
            StatusCode::OK
        )
        .await,
        default(shared)
    );
    call(
        &community,
        &other,
        Method::PUT,
        "profile/dashboard-default",
        default(restricted),
        StatusCode::NO_CONTENT,
    )
    .await;
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid=$1 AND resourceid=$2")
        .bind(other.actor_id.value())
        .bind(restricted)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        call(
            &community,
            &other,
            Method::GET,
            "profile/dashboard-default",
            Value::Null,
            StatusCode::OK
        )
        .await,
        default(shared)
    );
}

#[tokio::test]
#[ignore = "requires a disposable CITADEL_DASHBOARD_DATABASE_URL"]
async fn dashboard_sharing_enforces_visibility_revision_and_entitlement() {
    let db = std::env::var("CITADEL_DASHBOARD_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&db)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&db).await.unwrap();
    let app = |team| {
        router(DashboardsHttpState {
            dashboards: Arc::new(PostgresDashboardRepository::new(pool.clone())),
            entitlements: Arc::new(StaticEntitlementService::new(team)),
        })
    };
    let (team, community) = (app(true), app(false));
    let admin = actor(&pool, true).await;
    let editor = actor(&pool, false).await;
    let viewer = actor(&pool, false).await;
    let id = Uuid::now_v7();
    let path = format!("dashboards/{id}");
    let sharing = format!("{path}/visibility");
    let settings = json!({"schemaVersion":2,"rangeMinutes":60,"tags":[],"widgets":[]});
    let board = |id, mode| json!({"id":id,"name":format!("{mode} operations"),"visibility":mode,"configuration":settings});
    let change = |revision, mode| json!({"revision":revision,"visibility":mode});
    let share = |app, actor, revision, mode, status| {
        call(
            app,
            actor,
            Method::PUT,
            &sharing,
            change(revision, mode),
            status,
        )
    };
    let default_for_viewer = || {
        call(
            &community,
            &viewer,
            Method::GET,
            "profile/dashboard-default",
            Value::Null,
            StatusCode::OK,
        )
    };
    call(
        &team,
        &admin,
        Method::POST,
        "dashboards",
        board(id, "InstanceShared"),
        StatusCode::OK,
    )
    .await;
    let original_default = default_for_viewer().await;
    call(
        &team,
        &viewer,
        Method::PUT,
        "profile/dashboard-default",
        json!({"dashboardId":id}),
        StatusCode::NO_CONTENT,
    )
    .await;
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourceid,resourcetype,permissionlevel,specificpermissions) VALUES ($1,$2,$3,$4,$5,0)")
        .bind(Uuid::now_v7()).bind(editor.actor_id.value()).bind(id).bind(ResourceType::Dashboard as i32)
        .bind(PermissionLevel::Write as i32).execute(&pool).await.unwrap();
    share(&team, &editor, 1, "Restricted", StatusCode::FORBIDDEN).await;
    let denied = share(&community, &admin, 1, "Restricted", StatusCode::FORBIDDEN).await;
    assert_eq!(denied["capability"], "CustomAccessControl");
    let saved = share(&team, &admin, 1, "Restricted", StatusCode::OK).await;
    assert_eq!(saved["revision"], 2);
    assert_eq!(saved["visibility"], "Restricted");
    let noop = share(&community, &admin, 2, "Restricted", StatusCode::OK).await;
    assert_eq!(noop, saved);
    call(
        &community,
        &viewer,
        Method::GET,
        &path,
        Value::Null,
        StatusCode::NOT_FOUND,
    )
    .await;
    let fallback = default_for_viewer().await;
    assert_eq!(fallback, original_default);
    let retained = call(
        &community,
        &editor,
        Method::GET,
        &path,
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(retained["summary"]["canEdit"], true);
    assert_eq!(retained["configuration"], settings);
    share(&team, &admin, 1, "InstanceShared", StatusCode::CONFLICT).await;
    share(&team, &admin, 2, "Private", StatusCode::BAD_REQUEST).await;
    share(&community, &admin, 2, "InstanceShared", StatusCode::OK).await;
    let restored = default_for_viewer().await;
    assert_eq!(restored["dashboardId"], json!(id));
    for owner in [&admin, &editor] {
        let private = Uuid::now_v7();
        call(
            &community,
            owner,
            Method::POST,
            "dashboards",
            board(private, "Private"),
            StatusCode::OK,
        )
        .await;
        let status = if owner.actor_id == admin.actor_id {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::NOT_FOUND
        };
        call(
            &team,
            &admin,
            Method::PUT,
            &format!("dashboards/{private}/visibility"),
            change(1, "InstanceShared"),
            status,
        )
        .await;
        let own = call(
            &community,
            owner,
            Method::PUT,
            &format!("dashboards/{private}"),
            json!({"revision":1,"name":"My private notes","configuration":settings}),
            StatusCode::OK,
        )
        .await;
        assert_eq!(own["summary"]["visibility"], "Private");
        assert_eq!(own["summary"]["name"], "My private notes");
        assert_eq!(own["summary"]["revision"], 2);
        call(
            &community,
            owner,
            Method::DELETE,
            &format!("dashboards/{private}?revision=2"),
            Value::Null,
            StatusCode::NO_CONTENT,
        )
        .await;
    }
    call(
        &community,
        &admin,
        Method::DELETE,
        &format!("{path}?revision=3"),
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
}

#[tokio::test]
#[ignore = "requires a disposable CITADEL_DASHBOARD_DATABASE_URL"]
async fn widget_sources_enforce_access_and_upcoming_orders_enabled_schedules() {
    use citadel_adapters::persistence::postgres::instance::PostgresSchedules;
    use citadel_instance::{ScheduleCatalogPolicy, SchedulePurpose, ScheduleReader};
    let db = std::env::var("CITADEL_DASHBOARD_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&db)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&db).await.unwrap();
    let app = router(DashboardsHttpState {
        dashboards: Arc::new(PostgresDashboardRepository::new(pool.clone())),
        entitlements: Arc::new(StaticEntitlementService::new(false)),
    });
    let viewer = actor(&pool, false).await;
    let admin = actor(&pool, true).await;
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,platformdescriptor,status,volumecount) VALUES($1,'Dashboard source','','Local',1,0,1024,0,'{}','Online',0)").bind(id).execute(&pool).await.unwrap();
    let path = format!("dashboards/sources?kind=Platform&ids={id}");
    assert_eq!(
        call(
            &app,
            &viewer,
            Method::GET,
            &path,
            Value::Null,
            StatusCode::OK
        )
        .await,
        json!([])
    );
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourceid,resourcetype,permissionlevel,specificpermissions) VALUES ($1,$2,$3,$4,$5,0)").bind(Uuid::now_v7()).bind(viewer.actor_id.value()).bind(id).bind(ResourceType::Platform as i32).bind(PermissionLevel::Read as i32).execute(&pool).await.unwrap();
    sqlx::query("UPDATE platforms SET status='Offline' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    for read in ["Attention", "Summary", "States", "Members"] {
        let result = call(
            &app,
            &viewer,
            Method::GET,
            &format!("dashboards/fleet?read={read}"),
            Value::Null,
            StatusCode::OK,
        )
        .await;
        if read == "Attention" {
            assert_eq!(result["problems"], 1);
            assert_eq!(result["rows"][0]["id"], json!(id));
            assert_eq!(result["rows"][0]["reasons"][0]["code"], "Offline");
        } else if read == "Members" {
            assert_eq!(result["total"], 1);
        } else {
            assert_eq!(
                result["states"],
                json!([{"kind":"Platform","state":"Offline","count":1}])
            );
        }
    }
    for query in ["tab=Updates", "tags=unassigned"] {
        let result = call(
            &app,
            &viewer,
            Method::GET,
            &format!("dashboards/fleet?read=Attention&{query}"),
            Value::Null,
            StatusCode::OK,
        )
        .await;
        assert_eq!(result["rows"], json!([]));
    }
    sqlx::query("UPDATE platforms SET status='Online' WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let tagged = call(
        &app,
        &viewer,
        Method::GET,
        &format!("{path}&tags=missing"),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(tagged[0]["id"], json!(id));
    assert_eq!(tagged[0]["matchesTags"], false);
    for kind in ["Platform", "Deployment", "Stack", "SwarmService"] {
        let rows = call(
            &app,
            &admin,
            Method::GET,
            &format!("dashboards/sources?kind={kind}&tags=missing"),
            Value::Null,
            StatusCode::OK,
        )
        .await;
        assert_eq!(rows, json!([]));
    }
    let stack = Uuid::now_v7();
    let release = Uuid::now_v7();
    sqlx::query("INSERT INTO stacks(id,name,createdbyactorid,driftpolicy,stacksource,stackupdatestate) VALUES ($1,'Source budget',$2,'{}','Inline','{}')")
        .bind(stack).bind(admin.actor_id.value()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version) VALUES ($1,$2,$3,$4,'{}','Running','1')")
        .bind(release).bind(stack).bind(id).bind(admin.actor_id.value()).execute(&pool).await.unwrap();
    sqlx::query("UPDATE stacks SET currentstackreleaseid=$1 WHERE id=$2")
        .bind(release)
        .bind(stack)
        .execute(&pool)
        .await
        .unwrap();
    let physical: i64 = sqlx::query_scalar(
        "INSERT INTO statisticssources(platformid,containerid) VALUES ($1,$2) RETURNING id",
    )
    .bind(id)
    .bind(Uuid::now_v7())
    .fetch_one(&pool)
    .await
    .unwrap();
    call(
        &app,
        &admin,
        Method::GET,
        "dashboards/sources?kind=Container",
        Value::Null,
        StatusCode::BAD_REQUEST,
    )
    .await;
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,createdbyactorid,name,platformid,spec,status) VALUES ($1,$2,'Pinned deployment',$3,'{}','Healthy')")
        .bind(deployment).bind(admin.actor_id.value()).bind(id).execute(&pool).await.unwrap();
    let source_path = format!("dashboards/sources?kind=Deployment&ids={deployment}");
    for container in [Uuid::now_v7(), Uuid::now_v7()] {
        sqlx::query("INSERT INTO containers(id,name,platformid,deploymentid,dockercontainerid,dockerimageid,state,created,updated,ports) VALUES($1,'Fixture workload',$2,$3,$1::uuid::text,'image','Running',0,0,'[]')")
            .bind(container).bind(id).bind(deployment).execute(&pool).await.unwrap();
        let source = call(
            &app,
            &admin,
            Method::GET,
            &source_path,
            Value::Null,
            StatusCode::OK,
        )
        .await;
        assert_eq!(source[0]["id"], json!(deployment));
        assert_eq!(source[0]["containerProjectionId"], json!(container));
        let states = call(
            &app,
            &viewer,
            Method::GET,
            "dashboards/fleet?read=States&kinds=Container",
            Value::Null,
            StatusCode::OK,
        )
        .await;
        assert_eq!(
            states["states"],
            json!([{"kind":"Container","state":"Running","count":1}])
        );
        sqlx::query("DELETE FROM containers WHERE id=$1")
            .bind(container)
            .execute(&pool)
            .await
            .unwrap();
    }
    let stack_path = format!("dashboards/sources?kind=Stack&ids={stack}");
    for (first, last, exceeds) in [(1_i32, 8_i32, false), (9, 9, true)] {
        sqlx::query("INSERT INTO statisticsowners(sourceid,stackid,kind,servicename,replicakey) SELECT $1,$2,'Workload','service-'||n,'replica' FROM generate_series($3::integer,$4::integer)n")
            .bind(physical).bind(stack).bind(first).bind(last).execute(&pool).await.unwrap();
        let rows = call(
            &app,
            &admin,
            Method::GET,
            &stack_path,
            Value::Null,
            StatusCode::OK,
        )
        .await;
        assert_eq!(rows[0]["exceedsChartSeriesLimit"], exceeds);
    }
    let now = "2026-11-24T09:00:00Z".parse().unwrap();
    for (name, cron, enabled) in [
        ("Later", "0 12 * * *", true),
        ("Earlier", "0 10 * * *", true),
        ("Disabled", "1 9 * * *", false),
    ] {
        sqlx::query("INSERT INTO scripts(id,name,alertonfailure,code,createdbyactorid,runasactorid,enabled,scheduleenabled,schedulecron,scheduletimezone,timeoutseconds) VALUES($1,$2,false,'',$3,$3,$4,true,$5,'UTC',30)").bind(Uuid::now_v7()).bind(name).bind(viewer.actor_id.value()).bind(enabled).bind(cron).execute(&pool).await.unwrap();
    }
    let store = PostgresSchedules(pool);
    let fixed = SchedulePurpose::ALL
        .into_iter()
        .filter(|p| p.fixed_interval().is_some())
        .map(|p| (p, now))
        .collect();
    let policy = ScheduleCatalogPolicy {
        blockers: Default::default(),
        max_script_timeout: 300,
        attention_only: false,
    };
    let rows = store.upcoming(&[], 1, now, fixed, &policy).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Earlier");
    assert!(
        store
            .upcoming(&["missing".into()], 1, now, Default::default(), &policy)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
#[ignore = "requires a disposable CITADEL_DASHBOARD_DATABASE_URL"]
async fn fleet_ranks_before_limiting_and_counts_only_visible_owners() {
    let url = std::env::var("CITADEL_DASHBOARD_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let admin = actor(&pool, true).await;
    let reader = actor(&pool, false).await;
    let platform = Uuid::now_v7();
    let app = router(DashboardsHttpState {
        dashboards: Arc::new(PostgresDashboardRepository::new(pool.clone())),
        entitlements: Arc::new(StaticEntitlementService::new(false)),
    });
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,platformdescriptor,status,volumecount) VALUES($1,'Scale fixture',$1::uuid::text,'Local',1,0,1024,0,'{}','Online',0)").bind(platform).execute(&pool).await.unwrap();
    let problem = Uuid::now_v7();
    sqlx::query(r#"INSERT INTO deployments(id,createdbyactorid,name,platformid,spec,status,autoupdatestate_status,autoupdatestate_currentdigest,autoupdatestate_remotedigest) VALUES($1,$2,'zz-last',$3,'{"Image":{"ResolvedDigest":"sha256:applied"}}','Failed','UpdateAvailable','sha256:applied','sha256:new')"#).bind(problem).bind(admin.actor_id.value()).bind(platform).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourceid,resourcetype,permissionlevel,specificpermissions) VALUES (gen_random_uuid(),$1,$2,$3,$4,0)").bind(reader.actor_id.value()).bind(problem).bind(ResourceType::Deployment as i32).bind(PermissionLevel::Read as i32).execute(&pool).await.unwrap();
    let member_query = "dashboards/fleet?read=Members&kinds=Deployment&state=Failed";
    let member = call(
        &app,
        &reader,
        Method::GET,
        member_query,
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(member["total"], 1);
    assert_eq!(member["rows"][0]["id"], json!(problem));
    assert!(member["rows"][0].get("platformName").is_none());
    let platform_search = format!("{member_query}&search=Scale%20Fixture");
    assert_eq!(
        call(
            &app,
            &reader,
            Method::GET,
            &platform_search,
            Value::Null,
            StatusCode::OK
        )
        .await["total"],
        0,
    );
    let visible_platform = call(
        &app,
        &admin,
        Method::GET,
        &platform_search,
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(visible_platform["rows"][0]["platformName"], "Scale fixture");

    let running = Uuid::now_v7();
    let stopped = Uuid::now_v7();
    sqlx::query(r#"INSERT INTO containers(id,name,platformid,dockercontainerid,dockerimageid,state,created,updated,ports)
        VALUES($1,'Web container',$3,$1::uuid::text,'sha256:public-web','Running',1700000000,1700000000,'{"80/tcp":[{"hostIP":"::","hostPort":"8080"},{"hostIP":"0.0.0.0","hostPort":"8080"}],"443/tcp":[],"53/udp":[{"HostPort":"5353"}]}'),
              ($2,'Legacy container',$3,$2::uuid::text,'sha256:public-old','Exited',1700000000,1700000000,'[{"PrivatePort":80,"PublicPort":8181,"Type":"tcp"},{"PrivatePort":443,"Type":"tcp"}]')"#)
        .bind(running).bind(stopped).bind(platform).execute(&pool).await.unwrap();
    let containers = "dashboards/fleet?read=Members&kinds=Container&search=SHA256%3APUBLIC&limit=1";
    let first = call(
        &app,
        &admin,
        Method::GET,
        containers,
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(first["total"], 2);
    assert_eq!(first["rows"][0]["id"], json!(stopped));
    assert_eq!(first["rows"][0]["publishedPorts"], json!(["8181/tcp"]));
    let second = call(
        &app,
        &admin,
        Method::GET,
        &format!("{containers}&state=Running"),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(second["total"], 1);
    assert_eq!(second["rows"][0]["id"], json!(running));
    assert_eq!(second["rows"][0]["image"], "sha256:public-web");
    assert!(second["rows"][0]["createdAt"].is_string());
    assert_eq!(
        second["rows"][0]["publishedPorts"],
        json!(["5353/udp", "8080/tcp"])
    );
    let forbidden_rows = call(
        &app,
        &reader,
        Method::GET,
        containers,
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(forbidden_rows["total"], 0);
    let literal = call(
        &app,
        &admin,
        Method::GET,
        &format!("{member_query}&search=%25"),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(literal["total"], 0);
    let mut previous = 1;
    for size in [10, 1_000, 10_000] {
        sqlx::query("INSERT INTO deployments(id,createdbyactorid,name,platformid,spec,status) SELECT gen_random_uuid(),$1,'app-'||i,$2,'{}','Healthy' FROM generate_series($3::int+1,$4::int) i").bind(admin.actor_id.value()).bind(platform).bind(previous).bind(size).execute(&pool).await.unwrap();
        previous = size;
        for viewer in [&admin, &reader] {
            for tab in ["All", "Updates"] {
                let result = call(
                    &app,
                    viewer,
                    Method::GET,
                    &format!("dashboards/fleet?read=Attention&kinds=Deployment&tab={tab}&limit=1"),
                    Value::Null,
                    StatusCode::OK,
                )
                .await;
                assert!(
                    serde_json::to_vec(&result).unwrap().len() < 1024,
                    "One preview owner should have a bounded payload at fleet size {size}"
                );
                assert_eq!(result["rows"][0]["id"], json!(problem));
                assert_eq!(result["problems"], 1);
                assert_eq!(result["updates"], 1);
                assert_eq!(result["total"], 1);
                assert_eq!(result["rows"][0]["reasons"].as_array().unwrap().len(), 2);
            }
            let states = call(
                &app,
                viewer,
                Method::GET,
                "dashboards/fleet?read=States&kinds=Deployment",
                Value::Null,
                StatusCode::OK,
            )
            .await;
            let total: i64 = states["states"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["count"].as_i64().unwrap())
                .sum();
            assert_eq!(
                total,
                if viewer.is_administrator() {
                    i64::from(size)
                } else {
                    1
                }
            );
        }
    }
    sqlx::query("UPDATE deployments SET spec='{}',status='Stopped' WHERE id=$1")
        .bind(problem)
        .execute(&pool)
        .await
        .unwrap();
    let result = call(
        &app,
        &reader,
        Method::GET,
        "dashboards/fleet?read=Attention",
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(result["total"], 0);
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('fixture.actor',$1,true),set_config('fixture.platform',$2,true),set_config('fixture.deployment',$3,true)").bind(admin.actor_id.value().to_string()).bind(platform.to_string()).bind(problem.to_string()).execute(&mut *tx).await.unwrap();
    sqlx::raw_sql(include_str!("fixtures/dashboard_attention.sql"))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let get =
        |path: &'static str| call(&app, &admin, Method::GET, path, Value::Null, StatusCode::OK);
    let all = get("dashboards/fleet?read=Attention").await;
    assert_eq!(
        (&all["total"], &all["problems"], &all["updates"]),
        (&json!(14), &json!(4), &json!(12))
    );
    let rows = all["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0]["kind"], "Platform");
    assert_eq!(rows[1]["id"], json!(problem));
    let summary = get("dashboards/fleet?read=Summary").await;
    let alerts = get("dashboards/fleet?read=OpenAlerts").await;
    assert_eq!(summary["alerts"], 3);
    assert_eq!(alerts["total"], summary["alerts"]);
    let alert_rows = alerts["rows"].as_array().unwrap();
    assert_eq!(alert_rows.len(), 3);
    for alert in alert_rows {
        let event = &alert["reasons"][0];
        assert_eq!(event["target"], alert["id"]);
        assert!(event["code"].as_str().is_some_and(|code| !code.is_empty()));
        assert!(event["observedAt"].is_string());
    }
    let last = get("dashboards/fleet?read=Attention&tab=Updates&limit=25").await;
    assert!(
        last["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "web-stack" && r["reasons"][0]["target"].is_string())
    );
    let tags: Vec<String> = sqlx::query_scalar(
        "SELECT t.name FROM tags t JOIN resourcetags rt ON rt.tagid=t.id WHERE rt.resourceid=$1",
    )
    .bind(problem)
    .fetch_all(&pool)
    .await
    .unwrap();
    let scoped = call(
        &app,
        &reader,
        Method::GET,
        &format!(
            "dashboards/fleet?read=Attention&tags={}&tags={}",
            tags[0], tags[1]
        ),
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(scoped["total"], 1);
    assert_eq!(scoped["rows"][0]["reasons"].as_array().unwrap().len(), 2);
    sqlx::query("INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,snapshotavailability,sourcesnapshot,status,trigger,triggeredbyactorid,completedat) SELECT gen_random_uuid(),id,backuprepositoryid,name,'Local','Available','{}','Succeeded','Scheduled',runasactorid,now() FROM backuppolicies WHERE normalizedname='r4-payments-daily'").execute(&pool).await.unwrap();
    assert_eq!(
        get("dashboards/fleet?read=Attention&kinds=BackupPolicy").await["total"],
        0
    );
}

#[path = "dashboards_http/activity.rs"]
mod activity;

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
            backup_coverage: None,
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
    let private = Uuid::now_v7();
    let shared = Uuid::now_v7();
    let restricted = Uuid::now_v7();
    let config = json!({"schemaVersion":1,"rangeMinutes":60,"tags":[],"widgets":[]});
    let board = |id, visibility| json!({"id":id,"name":"System","visibility":visibility,"configuration":config});
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
    assert_eq!(saved["summary"]["revision"], 1);
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
    sqlx::query("UPDATE dashboards SET configuration=jsonb_set(configuration,'{schemaVersion}','2') WHERE id=$1")
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
    assert_eq!(newer["configuration"]["schemaVersion"], 2);
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
            backup_coverage: None,
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
    let settings = json!({"schemaVersion":1,"rangeMinutes":60,"tags":[],"widgets":[]});
    let board =
        |id, mode| json!({"id":id,"name":"Operations","visibility":mode,"configuration":settings});
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
        backup_coverage: None,
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
    for kind in [
        "Platform",
        "Deployment",
        "Container",
        "Stack",
        "SwarmService",
    ] {
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
    for kind in ["Stack", "SwarmService", "Deployment", "Container"] {
        let rows = call(
            &app,
            &admin,
            Method::GET,
            &format!("dashboards/workloads?kind={kind}&tags=missing"),
            Value::Null,
            StatusCode::OK,
        )
        .await;
        assert_eq!(rows, json!([]));
    }
    let rows = call(
        &app,
        &admin,
        Method::GET,
        "dashboards/workloads?kind=Stack",
        Value::Null,
        StatusCode::OK,
    )
    .await;
    let visible = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == json!(stack))
        .unwrap();
    assert_eq!(visible["name"], "Source budget");
    assert_eq!(visible["status"], "Running");
    assert_eq!(visible["processing"], false);
    assert_eq!(visible["canViewLogs"], true);
    assert!(visible.get("spec").is_none());
    assert_eq!(visible["observedProbes"], json!([]));
    let rows = call(
        &app,
        &viewer,
        Method::GET,
        "dashboards/workloads?kind=Stack",
        Value::Null,
        StatusCode::OK,
    )
    .await;
    assert_eq!(rows, json!([]));
    let container = Uuid::now_v7();
    sqlx::query("INSERT INTO containers(id,name,platformid,dockercontainerid,dockerimageid,state,created,updated,ports,health,controlstate) VALUES($1,'Fixture workload',$2,'fixture-runtime','image','Running',0,0,'[]',$3,'Processing')")
        .bind(container).bind(id).bind(json!({"configuration":"Enabled","result":"Unhealthy","observedAt":chrono::Utc::now().timestamp_millis()})).execute(&pool).await.unwrap();
    let rows = call(
        &app,
        &viewer,
        Method::GET,
        "dashboards/workloads?kind=Container",
        Value::Null,
        StatusCode::OK,
    )
    .await;
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == json!(container))
        .unwrap();
    assert_eq!(row["processing"], true);
    assert_eq!(row["observedProbes"], json!(["Unhealthy"]));
    assert_eq!(row["canViewLogs"], false);
    assert!(row["backupStatus"].is_null());
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

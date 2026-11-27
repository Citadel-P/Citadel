use super::*;
use citadel_adapters::persistence::postgres::activities::store::PostgresActivityStore;
use citadel_identity::IdentityStore;
use citadel_server::api::routes::activities::{self, ActivitiesHttpState};

async fn get(app: &Router, actor: &ActorPrincipal, path: &str) -> Value {
    call(app, actor, Method::GET, path, Value::Null, StatusCode::OK).await
}

#[tokio::test]
#[ignore = "requires a disposable CITADEL_DASHBOARD_DATABASE_URL"]
async fn dashboard_activity_tracks_committed_changes_and_current_audience() {
    let db = std::env::var("CITADEL_DASHBOARD_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&db)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&db).await.unwrap();
    let app = router(DashboardsHttpState {
        dashboards: Arc::new(PostgresDashboardRepository::new(pool.clone())),
        entitlements: Arc::new(StaticEntitlementService::new(true)),
        identity: identity(&pool, []),
    })
    .merge(activities::router(ActivitiesHttpState {
        activities: Arc::new(citadel_activities::ActivityService::new(Arc::new(
            PostgresActivityStore::new(pool.clone()),
        ))),
    }));
    let owner = actor(&pool, false).await;
    let admin = actor(&pool, true).await;
    let viewer = actor(&pool, false).await;
    let recent = "activities/recent?eventType=DashboardCreated&limit=10";
    for (visibility, creator) in [("Private", &owner), ("InstanceShared", &admin)] {
        let id = Uuid::now_v7();
        let name = id.to_string();
        let path = format!("dashboards/{id}");
        let list = format!("activities?ResourceType=Dashboard&ResourceId={id}");
        let widget = Uuid::now_v7();
        let initial_snapshot = json!({"name":name,"configuration":{
            "schemaVersion":2,"rangeMinutes":60,"tags":[],"widgets":[{
                "id":widget,"title":"Runbook","layout":{"x":0,"y":0,"w":6,"h":4},
                "settings":{"kind":"Notes"}
            }]
        }});
        let mut config = initial_snapshot["configuration"].clone();
        config["description"] = json!("private operator notes");
        config["widgets"][0]["settings"]["text"] = json!("private notes body");
        config["widgets"][0]["settings"]["links"] =
            json!([{"label":"Internal link","href":"https://example.test/?token=placeholder"}]);
        let create = json!({"id":id,"name":name,"visibility":visibility,"configuration":config});
        for _ in 0..2 {
            call(
                &app,
                creator,
                Method::POST,
                "dashboards",
                create.clone(),
                StatusCode::OK,
            )
            .await;
        }
        let records = get(&app, creator, &list).await;
        assert_eq!(records["pagedResult"]["totalCount"], 1);
        let created = &records["pagedResult"]["items"][0];
        assert_eq!(created["actorId"], json!(creator.actor_id.value()));
        assert_eq!(
            created["info"],
            json!({"$type":"DashboardCreated","visibility":visibility})
        );
        let event = format!("activities/{}", created["id"].as_str().unwrap());
        assert_eq!(
            get(&app, creator, &event).await["info"]["configuration"],
            initial_snapshot
        );
        let present = |rows: Value| {
            rows.as_array()
                .unwrap()
                .iter()
                .any(|r| r["resourceName"] == name)
        };
        assert!(present(get(&app, creator, recent).await));
        if visibility == "Private" {
            for outsider in [&admin, &viewer] {
                assert_eq!(
                    get(&app, outsider, &list).await["pagedResult"]["totalCount"],
                    0
                );
                assert!(!present(get(&app, outsider, recent).await));
                assert_eq!(
                    send(&app, outsider, Method::GET, &event, Value::Null)
                        .await
                        .status(),
                    StatusCode::NOT_FOUND
                );
            }
        } else {
            assert!(present(get(&app, &viewer, recent).await));
            assert_eq!(
                get(&app, &viewer, &event).await["info"]["configuration"],
                initial_snapshot
            );
            let sharing = format!("{path}/visibility");
            call(
                &app,
                creator,
                Method::PUT,
                &sharing,
                json!({"revision":1,"visibility":"Restricted"}),
                StatusCode::OK,
            )
            .await;
            assert!(!present(get(&app, &viewer, recent).await));
            assert_eq!(
                get(&app, &viewer, &list).await["pagedResult"]["totalCount"],
                0
            );
            assert_eq!(
                send(&app, &viewer, Method::GET, &event, Value::Null)
                    .await
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
        config["description"] = json!("updated private operator notes");
        config["rangeMinutes"] = json!(360);
        config["widgets"][0]["layout"]["y"] = json!(2);
        config["widgets"][0]["settings"]["text"] = json!("updated private notes body");
        let mut updated_snapshot = initial_snapshot.clone();
        updated_snapshot["configuration"]["rangeMinutes"] = json!(360);
        updated_snapshot["configuration"]["widgets"][0]["layout"]["y"] = json!(2);
        let revision = if visibility == "Private" { 1 } else { 2 };
        let update = |r| json!({"revision":r,"name":name,"configuration":config});
        call(
            &app,
            creator,
            Method::PUT,
            &path,
            update(revision),
            StatusCode::OK,
        )
        .await;
        call(
            &app,
            creator,
            Method::PUT,
            &path,
            update(revision),
            StatusCode::CONFLICT,
        )
        .await;
        call(
            &app,
            creator,
            Method::PUT,
            &path,
            update(revision + 1),
            StatusCode::OK,
        )
        .await;
        let records = get(&app, creator, &list).await;
        assert_eq!(records["pagedResult"]["totalCount"], revision + 1);
        let updated = &records["pagedResult"]["items"][0];
        let summary = json!({
            "$type":"DashboardUpdated",
            "changes":["Description","Chart range","Layout","Widgets"]
        });
        assert_eq!(updated["info"], summary);
        let details_path = format!("activities/{}", updated["id"].as_str().unwrap());
        let details = get(&app, creator, &details_path).await;
        assert_eq!(details["info"]["previousConfiguration"], initial_snapshot);
        assert_eq!(details["info"]["configuration"], updated_snapshot);
        assert_eq!(
            send(&app, &viewer, Method::GET, &details_path, Value::Null)
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
        let authorization = PostgresIdentityStore::new(pool.clone());
        if visibility != "Private" {
            for choice in [Some(id), Some(id), None, None] {
                call(
                    &app,
                    creator,
                    Method::PUT,
                    "instance/dashboard-default",
                    json!({"dashboardId":choice}),
                    StatusCode::NO_CONTENT,
                )
                .await;
            }
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM activityevents
                 WHERE createdbyactorid=$1 AND eventtype='InstanceHomeDashboardChanged'",
            )
            .bind(creator.actor_id.value())
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(count, 2);
            sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourceid,resourcetype,permissionlevel,specificpermissions) VALUES ($1,$2,$3,$4,$5,0)")
                .bind(Uuid::now_v7())
                .bind(viewer.actor_id.value())
                .bind(id)
                .bind(ResourceType::Dashboard as i32)
                .bind(PermissionLevel::Read as i32)
                .execute(&pool)
                .await
                .unwrap();
            assert!(
                authorization
                    .resource_permission(viewer.actor_id, ResourceType::Dashboard, id)
                    .await
                    .unwrap()
                    .is_some()
            );
        }
        call(
            &app,
            creator,
            Method::DELETE,
            &format!("{path}?revision={}", revision + 1),
            Value::Null,
            StatusCode::NO_CONTENT,
        )
        .await;
        if visibility != "Private" {
            assert!(
                authorization
                    .resource_permission(viewer.actor_id, ResourceType::Dashboard, id)
                    .await
                    .unwrap()
                    .is_none()
            );
        }
        let history = get(&app, creator, &list).await;
        let deleted = &history["pagedResult"]["items"][0];
        let visibility_at_deletion = if visibility == "Private" {
            visibility
        } else {
            "Restricted"
        };
        assert_eq!(
            deleted["info"],
            json!({"$type":"DashboardDeleted","visibility":visibility_at_deletion})
        );
        let deletion_path = format!("activities/{}", deleted["id"].as_str().unwrap());
        assert_eq!(
            get(&app, creator, &deletion_path).await["info"]["configuration"],
            updated_snapshot
        );
        assert_eq!(
            send(&app, &viewer, Method::GET, &deletion_path, Value::Null)
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(history["pagedResult"]["totalCount"], revision + 2);
        assert_eq!(
            get(&app, &viewer, &list).await["pagedResult"]["totalCount"],
            0
        );
        if visibility == "Private" {
            assert_eq!(
                get(&app, &admin, &list).await["pagedResult"]["totalCount"],
                0
            );
            assert_eq!(
                send(&app, &admin, Method::GET, &deletion_path, Value::Null)
                    .await
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
        call(
            &app,
            &viewer,
            Method::POST,
            "dashboards",
            json!({"visibility":"Private","id":id,"name":"Reused","configuration":config}),
            StatusCode::CONFLICT,
        )
        .await;
    }

    let unsupported = Uuid::now_v7();
    sqlx::query("INSERT INTO dashboards (id, name, visibility, configuration) VALUES ($1, 'Unsupported activity fixture', 'InstanceShared', $2)")
        .bind(unsupported)
        .bind(json!({"schemaVersion":3,"unknownField":"private"}))
        .execute(&pool)
        .await
        .unwrap();
    call(
        &app,
        &admin,
        Method::DELETE,
        &format!("dashboards/{unsupported}?revision=1"),
        Value::Null,
        StatusCode::NO_CONTENT,
    )
    .await;
    let history = get(
        &app,
        &admin,
        &format!("activities?ResourceType=Dashboard&ResourceId={unsupported}"),
    )
    .await;
    let event = history["pagedResult"]["items"][0]["id"].as_str().unwrap();
    assert_eq!(
        get(&app, &admin, &format!("activities/{event}")).await["info"],
        json!({"$type":"DashboardDeleted","visibility":"InstanceShared"})
    );
}

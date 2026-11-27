use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use citadel_activities::{
    ActivityAccess, ActivityError, ActivityQueryStore, ActivityRecord, ActivityService,
    PagedActivityRecords, ValidatedActivityFilter,
};
use citadel_identity::{ActorPrincipal, AuthenticatedPrincipalType};
use citadel_primitives::ActorId;
use citadel_server::api::routes::{activities as activities_http, activities::ActivitiesHttpState};
use futures_util::future::BoxFuture;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Default)]
struct Store {
    scopes: Mutex<Vec<ActivityAccess>>,
    filters: Mutex<Vec<ValidatedActivityFilter>>,
    recent: Mutex<Vec<(Option<citadel_activities::ActivityEventType>, i32)>>,
}
impl ActivityQueryStore for Store {
    fn get_authorized<'a>(
        &'a self,
        access: &'a ActivityAccess,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<ActivityRecord>, ActivityError>> {
        Box::pin(async move {
            self.scopes.lock().unwrap().push(*access);
            if id.is_nil() {
                Err(ActivityError::Storage("sensitive database detail".into()))
            } else {
                Ok(None)
            }
        })
    }
    fn recent_authorized<'a>(
        &'a self,
        access: &'a ActivityAccess,
        event_type: Option<citadel_activities::ActivityEventType>,
        limit: i32,
        _tags: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<citadel_activities::RecentActivity>, ActivityError>> {
        self.scopes.lock().unwrap().push(*access);
        self.recent.lock().unwrap().push((event_type, limit));
        Box::pin(async { Ok(vec![]) })
    }
    fn list_authorized<'a>(
        &'a self,
        access: &'a ActivityAccess,
        filter: ValidatedActivityFilter,
    ) -> BoxFuture<'a, Result<PagedActivityRecords, ActivityError>> {
        Box::pin(async move {
            self.filters.lock().unwrap().push(filter);
            self.scopes.lock().unwrap().push(*access);
            Ok(PagedActivityRecords {
                items: vec![],
                total_count: 0,
                page: filter.page,
                page_size: filter.page_size,
            })
        })
    }
}

#[tokio::test]
async fn dashboard_resource_filter_accepts_activity_page_query() {
    let (app, store, _) = app(AuthenticatedPrincipalType::User);
    let (status, body) = get(
        &app,
        "/api/v1/activities?Page=1&PageSize=50&ResourceType=Dashboard",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["pagedResult"]["pageSize"], 50);
    let filters = store.filters.lock().unwrap();
    assert_eq!(
        filters[0].resource_type,
        Some(citadel_activities::ActivityResourceType::Dashboard)
    );
    assert_eq!(filters[0].page, 1);
}

#[tokio::test]
async fn failed_backup_filter_preserves_event_type_and_failure_status() {
    let (app, store, _) = app(AuthenticatedPrincipalType::User);
    let (status, _) = get(
        &app,
        "/api/v1/activities?EventType=BackupRunCompleted&Status=Failure",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    {
        let filters = store.filters.lock().unwrap();
        assert_eq!(
            filters[0].event_type,
            Some(citadel_activities::ActivityEventType::BackupRunCompleted)
        );
        assert_eq!(
            filters[0].status,
            Some(citadel_activities::ActivityStatus::Failure)
        );
    }
    let (status, _) = get(&app, "/api/v1/activities?Status=Invalid").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(store.filters.lock().unwrap().len(), 1);
}
fn app(principal_type: AuthenticatedPrincipalType) -> (Router, Arc<Store>, ActorId) {
    let store = Arc::new(Store::default());
    let actor_id = ActorId::new(Uuid::now_v7());
    let principal = ActorPrincipal {
        subject_id: Uuid::now_v7(),
        actor_id,
        name: "Reader".into(),
        principal_type,
        credential_id: None,
        roles: vec!["Admin".into()],
    };
    let app = activities_http::router(ActivitiesHttpState {
        activities: Arc::new(ActivityService::new(store.clone())),
    })
    .layer(Extension(principal));
    (app, store, actor_id)
}
async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}
#[tokio::test]
async fn activity_scope_comes_from_the_principal_and_service_accounts_cannot_claim_admin() {
    for (kind, administrator) in [
        (AuthenticatedPrincipalType::User, true),
        (AuthenticatedPrincipalType::ServiceAccount, false),
    ] {
        let (app, store, actor_id) = app(kind);
        let (status, body) = get(&app, "/api/v1/activities?administrator=true&PageSize=12").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["pagedResult"]["pageSize"], 12);
        let scopes = store.scopes.lock().unwrap();
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0].actor_id, actor_id);
        assert_eq!(scopes[0].administrator, administrator);
    }
}
#[tokio::test]
async fn activity_service_errors_preserve_http_status_and_hide_storage_details() {
    let (app, store, _) = app(AuthenticatedPrincipalType::User);
    let (status, body) = get(&app, "/api/v1/activities?PageSize=501").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["type"], "validation_error");
    assert_eq!(body["detail"], "PageSize must be between 1 and 500.");
    assert!(store.scopes.lock().unwrap().is_empty());
    let (status, body) = get(&app, &format!("/api/v1/activities/{}", Uuid::now_v7())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["type"], "not_found");
    let (status, body) = get(&app, &format!("/api/v1/activities/{}", Uuid::nil())).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["type"], "internal_error");
    assert_eq!(body["detail"], "An unexpected error occurred.");
    assert!(!body.to_string().contains("sensitive database detail"));
}

#[tokio::test]
async fn recent_summaries_use_the_viewers_scope_and_bound_the_read() {
    use citadel_activities::ActivityEventType;
    let (app, store, actor_id) = app(AuthenticatedPrincipalType::ServiceAccount);
    for invalid in [
        "limit=0",
        "limit=11",
        "eventType=Invalid",
        "tags=%20",
        "unexpected=x",
    ] {
        assert_eq!(
            get(&app, &format!("/api/v1/activities/recent?{invalid}"))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let (status, data) = get(
        &app,
        "/api/v1/activities/recent?eventType=BackupRunCompleted&limit=3",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(data, serde_json::json!([]));
    assert_eq!(
        *store.recent.lock().unwrap(),
        vec![(Some(ActivityEventType::BackupRunCompleted), 3)]
    );
    let scopes = store.scopes.lock().unwrap();
    assert_eq!(scopes[0].actor_id, actor_id);
    assert!(!scopes[0].administrator);
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn recent_activity_filters_before_limit_for_a_seeded_viewer() {
    use citadel_adapters::persistence::postgres::activities::store::PostgresActivityStore;
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let actor = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,type,isenabled) VALUES($1,'User',true)")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO actorroles(actorid,roleid) SELECT $1,id FROM roles WHERE name='Viewer'",
    )
    .bind(actor)
    .execute(&pool)
    .await
    .unwrap();
    let backup = Uuid::now_v7();
    let platform = Uuid::now_v7();
    let policy = Uuid::now_v7();
    sqlx::query("INSERT INTO secretdefinitions(id,name,providertype) VALUES($1,'test-password','InternalEncrypted')")
        .bind(policy).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO backuprepositories(id,name,normalizedname,createdbyactorid,passwordsecretid,spec,status,type)
        VALUES($1,'repo','REPO',$2,$1,'{}','Ready','Local')")
        .bind(policy).bind(actor).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO backuppolicies(id,name,normalizedname,createdbyactorid,runasactorid,backuprepositoryid,source)
        VALUES($1,'policy','POLICY',$2,$2,$1,'{}')")
        .bind(policy).bind(actor).execute(&pool).await.unwrap();
    for name in ["Production", "Nightly"] {
        let tag = Uuid::now_v7();
        sqlx::query("INSERT INTO tags(id,name,normalizedname,color,createdbyactorid) VALUES($1,$2,lower($2),'#ffffff',$3)")
            .bind(tag).bind(name).bind(actor).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid) VALUES('BackupPolicy',$1,$2,$3)")
            .bind(policy).bind(tag).bind(actor).execute(&pool).await.unwrap();
    }
    for (id, kind, event, name) in [
        (
            backup,
            "BackupPolicy",
            "BackupRunCompleted",
            "Nightly volumes",
        ),
        (platform, "Platform", "PlatformConnected", "Lab"),
        (Uuid::now_v7(), "User", "UserCreated", "Private admin event"),
    ] {
        sqlx::query("INSERT INTO activityevents(id,createdbyactorid,eventtype,resourcetype,resourceid,resourcename,info,status)
          VALUES($1,$2,$3,$4,$5,$6,'{}','Success')")
            .bind(id).bind(actor).bind(event).bind(kind).bind(policy).bind(name)
            .execute(&pool).await.unwrap();
    }
    let app = activities_http::router(ActivitiesHttpState {
        activities: Arc::new(ActivityService::new(Arc::new(PostgresActivityStore::new(
            pool.clone(),
        )))),
    })
    .layer(Extension(ActorPrincipal {
        actor_id: ActorId::new(actor),
        subject_id: actor,
        name: "Viewer".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec!["Viewer".into()],
    }));
    for (query, expected) in [
        ("limit=1", platform),
        ("limit=1&tags=production&TaGs=NIGHTLY", backup),
        ("eventType=BackupRunCompleted&tags=nightly&limit=1", backup),
        ("eventType=BackupRunCompleted&limit=1", backup),
    ] {
        let (status, data) = get(&app, &format!("/api/v1/activities/recent?{query}")).await;
        assert_eq!(status, StatusCode::OK);
        let rows = data.as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["id"], expected.to_string());
        assert_eq!(
            rows[0]["resourceType"],
            if expected == backup {
                "BackupPolicy"
            } else {
                "Platform"
            }
        );
        assert!(rows[0].get("info").is_none());
    }
    let (status, data) = get(
        &app,
        "/api/v1/activities/recent?tags=Production&tags=unknown",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(data, serde_json::json!([]));
    sqlx::query("DELETE FROM resourcetags WHERE resourceid=$1 AND resourcetype='BackupPolicy'")
        .bind(policy)
        .execute(&pool)
        .await
        .unwrap();
    let (status, rows) = get(
        &app,
        "/api/v1/activities/recent?eventType=BackupRunCompleted&tags=Production",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(rows, serde_json::json!([]));
}

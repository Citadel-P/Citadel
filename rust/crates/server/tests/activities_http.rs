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
    fn list_authorized<'a>(
        &'a self,
        access: &'a ActivityAccess,
        filter: ValidatedActivityFilter,
    ) -> BoxFuture<'a, Result<PagedActivityRecords, ActivityError>> {
        Box::pin(async move {
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

use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use citadel_adapters::connectors::registries::browser::RegistryBrowser;
use citadel_primitives::{ActorId, PatchField};
use citadel_registries::*;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

#[derive(Default)]
struct Repository(AtomicUsize, Option<RegistryDetails>);
impl RegistryRepository for Repository {
    fn list_registries<'a>(
        &'a self,
        _: ActorId,
        _: bool,
    ) -> BoxFuture<'a, Result<Vec<RegistryDetails>, RegistryError>> {
        unimplemented!()
    }
    fn get_registry<'a>(
        &'a self,
        _: Uuid,
    ) -> BoxFuture<'a, Result<RegistryDetails, RegistryError>> {
        Box::pin(async move { self.1.clone().ok_or(RegistryError::NotFound) })
    }
    fn create_registry<'a>(
        &'a self,
        _: ActorId,
        input: &'a NewRegistry,
    ) -> BoxFuture<'a, Result<RegistryDetails, RegistryError>> {
        Box::pin(async move {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(RegistryDetails {
                id: Uuid::now_v7(),
                audit: citadel_primitives::AuditMetadata {
                    created_by_actor_id: citadel_primitives::ActorId::new(Uuid::now_v7()),
                    created_at: chrono::Utc::now(),
                },
                name: input.name.clone(),
                status: input.status,
                description: None,
                registry_host: input.registry_host.clone(),
                registry_type: registry_type(&input.configuration)?,
                configuration: input.configuration.clone(),
                tags: vec![],
            })
        })
    }
    fn update_registry<'a>(
        &'a self,
        _: ActorId,
        _: Uuid,
        patch: &'a RegistryPatch,
        _: RegistryMutationKind,
    ) -> BoxFuture<'a, Result<RegistryDetails, RegistryError>> {
        Box::pin(async move {
            let current = self.1.as_ref().unwrap();
            let updated = patch.apply_to(current);
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(RegistryDetails {
                configuration: updated.configuration,
                ..current.clone()
            })
        })
    }
    fn delete_registries<'a>(
        &'a self,
        _: ActorId,
        _: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), RegistryError>> {
        unimplemented!()
    }
}

#[tokio::test]
async fn creation_checks_credentials_before_persistence() {
    let router = Router::new()
        .route(
            "/v2/auth/token",
            post(|Json(body): Json<Value>| async move {
                assert_eq!(body["identifier"], "test-user");
                match body["secret"].as_str().unwrap() {
                    "accepted-token" => (
                        StatusCode::OK,
                        Json(json!({"access_token":"session-token"})),
                    ),
                    "malformed-token" => (StatusCode::OK, Json(json!({"access_token":null}))),
                    _ => (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"message":"rejected-token secret must not be echoed"})),
                    ),
                }
            }),
        )
        .route(
            "/users/test-user/packages",
            get(|headers: HeaderMap| async move {
                if headers["authorization"] == "Bearer accepted-token" {
                    (StatusCode::OK, Json(json!([])))
                } else {
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"message":"rejected-token secret must not be echoed"})),
                    )
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let checker = RegistryBrowser::with_endpoints(&endpoint, &endpoint).unwrap();
    for (kind, token, accepted) in [
        ("DockerHub", "accepted-token", true),
        ("DockerHub", "rejected-token", false),
        ("DockerHub", "malformed-token", false),
        ("GitHub", "accepted-token", true),
        ("GitHub", "rejected-token", false),
    ] {
        let repository = Repository::default();
        let mut input = input(
            json!({"$type":kind,"userName":"test-user","nameSpace":"test-user","pat":token,"ghcrAuthEnabled":true}),
        );
        let result = create_registry(
            &repository,
            &checker,
            ActorId::new(Uuid::now_v7()),
            &mut input,
        )
        .await;
        assert_eq!(repository.0.load(Ordering::SeqCst), usize::from(accepted));
        if accepted {
            assert_eq!(
                result.unwrap().registry_host,
                if kind == "DockerHub" {
                    "docker.io"
                } else {
                    "ghcr.io"
                }
            );
        } else {
            let error = result.unwrap_err();
            assert!(matches!(error, RegistryError::Validation(_)));
            assert!(!error.to_string().contains(token));
        }
    }
    for kind in ["DockerHub", "GitHub"] {
        let actor = ActorId::new(Uuid::now_v7());
        let initial = input(
            json!({"$type":kind,"userName":"test-user","nameSpace":"test-user","pat":"accepted-token","ghcrAuthEnabled":true}),
        );
        let current = Repository::default()
            .create_registry(actor, &initial)
            .await
            .unwrap();
        for (token, accepted) in [("rejected-token", false), ("accepted-token", true)] {
            let repository = Repository(AtomicUsize::new(0), Some(current.clone()));
            let patch = RegistryPatch {
                configuration: PatchField::Value(json!({"pat":token})),
                ..Default::default()
            };
            let result = update_registry(
                &repository,
                &checker,
                actor,
                current.id,
                &patch,
                RegistryMutationKind::Update,
            )
            .await;
            assert_eq!(result.is_ok(), accepted);
            assert_eq!(repository.0.load(Ordering::SeqCst), usize::from(accepted));
            if !accepted {
                assert!(matches!(result.unwrap_err(), RegistryError::Validation(_)));
            }
        }
    }
    // Accepted configuration aliases must not bypass credential verification.
    let repository = Repository::default();
    let mut aliases = input(
        json!({"type":"GitHub","GhcrAuthEnabled":true,"NameSpace":"test-user","PAT":"rejected-token"}),
    );
    assert!(
        create_registry(
            &repository,
            &checker,
            ActorId::new(Uuid::now_v7()),
            &mut aliases
        )
        .await
        .is_err()
    );
    assert_eq!(repository.0.load(Ordering::SeqCst), 0);
    server.abort();
}

#[tokio::test]
async fn unreachable_provider_blocks_private_creation_but_public_github_needs_no_credentials() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let checker = RegistryBrowser::with_endpoints(&endpoint, &endpoint).unwrap();
    let repository = Repository::default();
    let actor = ActorId::new(Uuid::now_v7());
    let mut private =
        input(json!({"$type":"DockerHub","UserName":"test-user","PAT":"accepted-token"}));
    assert!(
        create_registry(&repository, &checker, actor, &mut private)
            .await
            .is_err()
    );
    assert_eq!(repository.0.load(Ordering::SeqCst), 0);
    let mut public = input(json!({"$type":"GitHub","ghcrAuthEnabled":false}));
    assert!(
        create_registry(&repository, &checker, actor, &mut public)
            .await
            .is_ok()
    );
    assert_eq!(repository.0.load(Ordering::SeqCst), 1);
}

fn input(configuration: Value) -> NewRegistry {
    NewRegistry {
        name: "test-registry".into(),
        registry_host: String::new(),
        status: RegistryStatus::Active,
        configuration,
        description: None,
        tag_ids: vec![],
    }
}

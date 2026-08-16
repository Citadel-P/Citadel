use std::sync::Arc;

use chrono::Duration;
use citadel_adapters::crypto::{
    Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
};
use citadel_adapters::identity_store::{PostgresIdentityStore, StaticEntitlementService};
use citadel_adapters::service_account_store::PostgresServiceAccountStore;
use citadel_application::{
    AddServiceAccountResourceAccessRequest, ArchiveServiceAccountsRequest,
    CreateServiceAccountRequest, CreateServiceAccountTokenRequest, IdentityError, IdentityService,
    InitializeCitadelRequest, NoopServiceAccountLastUsedTracker, ServiceAccountResourceAccess,
    ServiceAccountService, SessionMetadata, SystemClock,
};
use citadel_database::MigrationRunner;
use citadel_domain::{ADMIN_ROLE_ID, PermissionLevel, ResourceType, SpecificPermission};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn identity_and_service_account_lifecycle_is_atomic_and_actor_scoped() {
    let database_url = std::env::var("CITADEL_PHASE3_DATABASE_URL")
        .expect("CITADEL_PHASE3_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&database_url)
        .await
        .unwrap();

    let identity_store = Arc::new(PostgresIdentityStore::new(pool.clone()));
    let service_account_store = Arc::new(PostgresServiceAccountStore::new(pool.clone()));
    let entitlement = Arc::new(StaticEntitlementService::new(true));
    let service_tokens = Arc::new(OpaqueServiceAccountTokenCodec);
    let clock = Arc::new(SystemClock);
    let identity = Arc::new(IdentityService::new(
        identity_store,
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(&[7_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        service_tokens.clone(),
        entitlement.clone(),
        clock.clone(),
        Arc::new(NoopServiceAccountLastUsedTracker),
        Duration::minutes(15),
        Duration::days(30),
    ));
    let accounts = Arc::new(ServiceAccountService::new(
        service_account_store,
        service_tokens,
        entitlement,
        clock,
    ));

    assert!(identity.setup_status().await.unwrap().requires_setup);
    let (login, session) = identity
        .initialize(
            InitializeCitadelRequest {
                name: "owner".into(),
                email: "owner@example.test".into(),
                password: "correct-horse-battery-staple".into(),
            },
            metadata(),
        )
        .await
        .unwrap();
    assert!(login.access_token.is_some());
    assert!(!identity.setup_status().await.unwrap().requires_setup);
    let owner = identity
        .authenticate_bearer(&session.access_token)
        .await
        .unwrap();
    assert!(owner.is_administrator());

    let protected_resource = Uuid::now_v7();
    let mut account = accounts
        .create(
            CreateServiceAccountRequest {
                name: "ci-runner".into(),
                description: Some("CI".into()),
                is_enabled: true,
                team_ids: vec![],
                role_ids: vec![],
                resource_accesses: vec![ServiceAccountResourceAccess {
                    id: None,
                    resource_type: ResourceType::Platform,
                    resource_id: protected_resource,
                    resource_name: None,
                    permission_level: PermissionLevel::Read,
                    specific_permissions: vec![SpecificPermission::Inspect],
                }],
            },
            owner.actor_id,
        )
        .await
        .unwrap();
    account = accounts.add_role(account.id, ADMIN_ROLE_ID).await.unwrap();
    assert!(account.roles.iter().any(|role| role.id == ADMIN_ROLE_ID));
    account = accounts
        .remove_role(account.id, ADMIN_ROLE_ID)
        .await
        .unwrap();
    assert!(account.roles.iter().all(|role| role.id != ADMIN_ROLE_ID));

    let removable_resource = Uuid::now_v7();
    account = accounts
        .add_resource_access(
            account.id,
            AddServiceAccountResourceAccessRequest {
                resource_type: ResourceType::Platform,
                resource_id: removable_resource,
                permission_level: PermissionLevel::Read,
                specific_permissions: vec![],
            },
        )
        .await
        .unwrap();
    let access_id = account
        .resource_accesses
        .iter()
        .find(|access| access.resource_id == removable_resource)
        .and_then(|access| access.id)
        .unwrap();
    account = accounts
        .remove_resource_access(account.id, access_id)
        .await
        .unwrap();
    assert!(
        account
            .resource_accesses
            .iter()
            .all(|access| access.resource_id != removable_resource)
    );
    let token = accounts
        .create_token(
            account.id,
            CreateServiceAccountTokenRequest {
                name: "pipeline".into(),
                expires_at_utc: None,
                never_expires: false,
            },
            owner.actor_id,
        )
        .await
        .unwrap();
    assert!(matches!(
        accounts
            .create_token(
                account.id,
                CreateServiceAccountTokenRequest {
                    name: "pipeline".into(),
                    expires_at_utc: None,
                    never_expires: false,
                },
                owner.actor_id,
            )
            .await,
        Err(IdentityError::Conflict(_))
    ));
    assert_eq!(token.token.len(), 83);
    let service_principal = identity.authenticate_bearer(&token.token).await.unwrap();
    assert!(!service_principal.is_human());
    identity
        .authorize_resource(
            &service_principal,
            ResourceType::Platform,
            protected_resource,
            PermissionLevel::Read,
            Some(SpecificPermission::Inspect),
        )
        .await
        .unwrap();
    assert!(matches!(
        identity
            .authorize(
                &service_principal,
                ResourceType::Platform,
                PermissionLevel::Read,
                None,
            )
            .await,
        Err(IdentityError::Forbidden)
    ));

    accounts
        .revoke_token(account.id, token.credential.id, owner.actor_id)
        .await
        .unwrap();
    assert!(identity.authenticate_bearer(&token.token).await.is_err());

    let bounded = accounts
        .create(
            CreateServiceAccountRequest {
                name: "bounded-runner".into(),
                description: None,
                is_enabled: true,
                team_ids: vec![],
                role_ids: vec![],
                resource_accesses: vec![],
            },
            owner.actor_id,
        )
        .await
        .unwrap();
    let attempts = (0..12).map(|index| {
        let accounts = Arc::clone(&accounts);
        async move {
            accounts
                .create_token(
                    bounded.id,
                    CreateServiceAccountTokenRequest {
                        name: format!("token-{index}"),
                        expires_at_utc: None,
                        never_expires: false,
                    },
                    owner.actor_id,
                )
                .await
        }
    });
    let results = futures_util::future::join_all(attempts).await;
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 10);
    assert_eq!(
        accounts
            .list_tokens(bounded.id, 1, 100)
            .await
            .unwrap()
            .total_count,
        10
    );

    accounts
        .archive(
            ArchiveServiceAccountsRequest {
                ids: vec![bounded.id],
            }
            .ids,
            owner.actor_id,
        )
        .await
        .unwrap();
    let archived = accounts.get(bounded.id).await.unwrap();
    assert!(archived.archived_at_utc.is_some());
    assert!(!archived.is_enabled);
    assert!(matches!(
        accounts
            .create_token(
                bounded.id,
                CreateServiceAccountTokenRequest {
                    name: "after-archive".into(),
                    expires_at_utc: None,
                    never_expires: false,
                },
                owner.actor_id,
            )
            .await,
        Err(IdentityError::NotFound)
    ));

    pool.close().await;
}

fn metadata() -> SessionMetadata {
    SessionMetadata {
        user_agent: Some("phase3-test".into()),
        ip_address: Some("127.0.0.1".into()),
    }
}

use std::sync::Arc;

use chrono::{Duration, Utc};
use citadel_activities::ActivityFilter;
use citadel_activities::ActivityService;
use citadel_activities::{ActivityEventType, ActivityResourceType};
use citadel_adapters::persistence::postgres::activities::store::PostgresActivityStore;
use citadel_adapters::persistence::postgres::identity::authentication::store::PostgresIdentityStore;
use citadel_adapters::persistence::postgres::identity::authentication::store::StaticEntitlementService;
use citadel_adapters::persistence::postgres::identity::profile::repository::PostgresProfileRepository;
use citadel_adapters::persistence::postgres::identity::service_accounts::repository::PostgresServiceAccountRepository;
use citadel_adapters::persistence::postgres::identity::users::repository::PostgresUserRepository;
use citadel_adapters::security::identity::crypto::Argon2PasswordHasher;
use citadel_adapters::security::identity::crypto::JwtSessionTokenCodec;
use citadel_adapters::security::identity::crypto::OpaqueServiceAccountTokenCodec;
use citadel_database::MigrationRunner;
use citadel_identity::{ADMIN_ROLE_ID, ActorPrincipal, SYSTEM_ACTOR_ID};
use citadel_identity::{
    AddServiceAccountResourceAccess, ArchiveServiceAccounts, ChangeCurrentPassword,
    CreateServiceAccount, CreateServiceAccountToken, IdentityError, IdentityService, IdentityStore,
    InitializeCitadel, NewSession, NoopServiceAccountLastUsedTracker, PatchField,
    PatchUserPreferences, ProfileRepository, ProfileService, ServiceAccountResourceAccess,
    ServiceAccountService, SessionMetadata, SystemClock, UpdateCurrentProfile, UserReadService,
};
use citadel_identity::{AuthenticatedPrincipalType, UserDateTimeFormat, UserTheme};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
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
    let service_account_store = Arc::new(PostgresServiceAccountRepository::new(pool.clone()));
    let entitlement = Arc::new(StaticEntitlementService::new(true));
    let service_tokens = Arc::new(OpaqueServiceAccountTokenCodec);
    let clock = Arc::new(SystemClock);
    let identity = Arc::new(IdentityService::new(
        identity_store.clone(),
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
        clock.clone(),
    ));
    let profile_store = Arc::new(PostgresProfileRepository::new(pool.clone()));
    let profiles = ProfileService::new(profile_store.clone(), Arc::clone(&identity), clock);
    let users = UserReadService::new(Arc::new(PostgresUserRepository::new(pool.clone())));

    assert!(identity.setup_status().await.unwrap().requires_setup);
    let (login, session) = identity
        .initialize(
            InitializeCitadel {
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
    let profile = profiles.get(&owner).await.unwrap();
    assert_eq!(profile.display_name, "owner");
    assert_eq!(profile.email, "owner@example.test");
    assert!(profile.authentication.can_change_password);
    assert!(profile.authentication.can_use_local_password_mfa);
    assert!(
        profile
            .direct_roles
            .iter()
            .any(|role| role.id == ADMIN_ROLE_ID)
    );
    let preferences = profiles.get_preferences(&owner).await.unwrap();
    assert_eq!(preferences.time_zone, None);
    assert_eq!(preferences.date_time_format, UserDateTimeFormat::System);
    assert_eq!(preferences.theme, UserTheme::System);
    assert!(!preferences.is_persisted);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM userpreferences WHERE userid = $1")
            .bind(owner.subject_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let preferences = profiles
        .patch_preferences(
            &owner,
            PatchUserPreferences {
                time_zone: PatchField::Value("Europe/Paris".into()),
                date_time_format: PatchField::Value(UserDateTimeFormat::TwentyFourHour),
                theme: PatchField::Value(UserTheme::Dark),
            },
        )
        .await
        .unwrap();
    assert_eq!(preferences.time_zone.as_deref(), Some("Europe/Paris"));
    assert_eq!(
        preferences.date_time_format,
        UserDateTimeFormat::TwentyFourHour
    );
    assert_eq!(preferences.theme, UserTheme::Dark);
    assert!(preferences.is_persisted);
    let stored = sqlx::query_as::<_, (String, String, String)>(
        "SELECT timezone, datetimeformat, theme FROM userpreferences WHERE userid = $1",
    )
    .bind(owner.subject_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        stored,
        (
            "Europe/Paris".to_owned(),
            "TwentyFourHour".to_owned(),
            "Dark".to_owned()
        )
    );
    let time_zone_patch = profiles.patch_preferences(
        &owner,
        PatchUserPreferences {
            time_zone: PatchField::Value("America/New_York".into()),
            ..PatchUserPreferences::default()
        },
    );
    let theme_patch = profiles.patch_preferences(
        &owner,
        PatchUserPreferences {
            theme: PatchField::Value(UserTheme::Light),
            ..PatchUserPreferences::default()
        },
    );
    let (time_zone_result, theme_result) = tokio::join!(time_zone_patch, theme_patch);
    time_zone_result.unwrap();
    theme_result.unwrap();
    let preferences = profiles.get_preferences(&owner).await.unwrap();
    assert_eq!(preferences.time_zone.as_deref(), Some("America/New_York"));
    assert_eq!(preferences.theme, UserTheme::Light);
    assert_eq!(
        preferences.date_time_format,
        UserDateTimeFormat::TwentyFourHour
    );
    assert!(matches!(
        profiles
            .patch_preferences(
                &owner,
                PatchUserPreferences {
                    time_zone: PatchField::Value("Romance Standard Time".into()),
                    ..PatchUserPreferences::default()
                },
            )
            .await,
        Err(IdentityError::Validation(_))
    ));
    let profile = profiles
        .update(
            &owner,
            UpdateCurrentProfile {
                display_name: "owner-renamed".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(profile.display_name, "owner-renamed");
    assert!(
        profile_store
            .rename_user(
                owner.subject_id,
                "must-roll-back",
                ActorId::new(Uuid::now_v7()),
                Utc::now(),
            )
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT name FROM users WHERE id = $1")
            .bind(owner.subject_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "owner-renamed"
    );
    let (renamed_login, renamed_session) = identity
        .login(
            citadel_identity::Login {
                email_or_name: "owner-renamed".into(),
                password: "correct-horse-battery-staple".into(),
            },
            metadata_for(Some(
                "Mozilla/5.0 (Windows NT 10.0) AppleWebKit Chrome/140.0 Safari/537.36",
            )),
        )
        .await
        .unwrap();
    assert!(renamed_login.access_token.is_some());
    let foreign_actor_id = Uuid::now_v7();
    let foreign_user_id = Uuid::now_v7();
    let foreign_session_id = Uuid::now_v7();
    let now = Utc::now();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(foreign_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"
INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name, password)
VALUES ($1, $2, $3, $4, 'other@example.test', 'other-user', NULL)
"#,
    )
    .bind(foreign_user_id)
    .bind(foreign_actor_id)
    .bind(now)
    .bind(SYSTEM_ACTOR_ID)
    .execute(&pool)
    .await
    .unwrap();
    let platform_override_id = Uuid::now_v7();
    sqlx::query(
        r#"
INSERT INTO resourceaccesses
    (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions)
VALUES ($1, $2, 1, $3, 0, 1)
"#,
    )
    .bind(Uuid::now_v7())
    .bind(foreign_actor_id)
    .bind(platform_override_id)
    .execute(&pool)
    .await
    .unwrap();
    let listed_users = users.list(1, 10, None).await.unwrap();
    assert_eq!(listed_users.total_count, 2);
    assert!(
        listed_users
            .items
            .iter()
            .any(|user| user.id == owner.subject_id && user.name == "owner-renamed")
    );
    assert!(
        listed_users
            .items
            .iter()
            .any(|user| user.id == foreign_user_id && user.resource_accesses.is_none())
    );
    let filtered_users = users.list(1, 10, Some(" other ")).await.unwrap();
    assert_eq!(filtered_users.total_count, 1);
    assert_eq!(filtered_users.items[0].id, foreign_user_id);
    let searched_users = users.search("OTHER@EXAMPLE", 10).await.unwrap();
    assert_eq!(searched_users.len(), 1);
    assert_eq!(searched_users[0].id, foreign_user_id);
    let foreign_user = users.get(foreign_user_id).await.unwrap();
    assert!(foreign_user.is_enabled);
    assert_eq!(
        foreign_user.resource_accesses.as_ref().unwrap()[0].resource_id,
        platform_override_id
    );
    assert!(matches!(
        users.get(Uuid::now_v7()).await,
        Err(IdentityError::NotFound)
    ));
    sqlx::query(
        r#"
INSERT INTO refreshtokens (id, createdat, expiresat, ipaddress, lastseenat, useragent, userid)
VALUES ($1, $2, $3, '127.0.0.2', $2, 'foreign-session', $4)
"#,
    )
    .bind(foreign_session_id)
    .bind(now)
    .bind(now + Duration::days(1))
    .bind(foreign_user_id)
    .execute(&pool)
    .await
    .unwrap();
    let sessions = profiles
        .list_sessions(&owner, Some(&session.refresh_token))
        .await
        .unwrap();
    assert!(sessions.can_revoke_other_sessions);
    assert_eq!(sessions.sessions.len(), 2);
    assert!(sessions.sessions[0].is_current);
    let current_session_id = sessions.sessions[0].id;
    let other_session = sessions
        .sessions
        .iter()
        .find(|candidate| !candidate.is_current)
        .unwrap();
    assert_eq!(other_session.display_name, "Chrome on Windows");
    let other_session_id = other_session.id;
    assert!(matches!(
        profiles
            .revoke_session(&owner, Some(&session.refresh_token), foreign_session_id,)
            .await,
        Err(IdentityError::NotFound)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM refreshtokens WHERE id = $1")
            .bind(foreign_session_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(matches!(
        profiles
            .revoke_session(&owner, Some(&session.refresh_token), current_session_id,)
            .await,
        Err(IdentityError::NotFound)
    ));
    profiles
        .revoke_session(&owner, Some(&session.refresh_token), other_session_id)
        .await
        .unwrap();
    assert!(
        identity
            .refresh(&renamed_session.refresh_token, metadata())
            .await
            .is_err()
    );
    assert!(matches!(
        profiles
            .revoke_session(&owner, Some(&session.refresh_token), Uuid::now_v7())
            .await,
        Err(IdentityError::NotFound)
    ));

    for user_agent in ["Firefox/140.0 (Linux)", "curl/8.16.0"] {
        identity
            .login(
                citadel_identity::Login {
                    email_or_name: "owner@example.test".into(),
                    password: "correct-horse-battery-staple".into(),
                },
                metadata_for(Some(user_agent)),
            )
            .await
            .unwrap();
    }
    let revoked = profiles
        .revoke_other_sessions(&owner, Some(&session.refresh_token))
        .await
        .unwrap();
    assert_eq!(revoked.count, 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM refreshtokens WHERE id = $1")
            .bind(foreign_session_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    let sessions = profiles
        .list_sessions(&owner, Some(&session.refresh_token))
        .await
        .unwrap();
    assert_eq!(sessions.sessions.len(), 1);
    assert!(sessions.sessions[0].is_current);
    identity
        .refresh(&session.refresh_token, metadata())
        .await
        .unwrap();
    assert!(matches!(
        profiles.revoke_other_sessions(&owner, None).await,
        Err(IdentityError::Validation(_))
    ));

    let (_, password_other_session) = identity
        .login(
            citadel_identity::Login {
                email_or_name: "owner@example.test".into(),
                password: "correct-horse-battery-staple".into(),
            },
            metadata_for(Some("password-change-other-session")),
        )
        .await
        .unwrap();
    let stale_password_hash =
        sqlx::query_scalar::<_, String>("SELECT password FROM users WHERE id = $1")
            .bind(owner.subject_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(matches!(
        profiles
            .change_password(
                &owner,
                Some(&session.refresh_token),
                ChangeCurrentPassword {
                    current_password: "wrong-current-password".into(),
                    new_password: "new-correct-horse-battery-staple".into(),
                },
            )
            .await,
        Err(IdentityError::Validation(_))
    ));
    assert!(matches!(
        profiles
            .change_password(
                &owner,
                Some(&session.refresh_token),
                ChangeCurrentPassword {
                    current_password: "correct-horse-battery-staple".into(),
                    new_password: "short".into(),
                },
            )
            .await,
        Err(IdentityError::Validation(_))
    ));
    profiles
        .change_password(
            &owner,
            Some(&session.refresh_token),
            ChangeCurrentPassword {
                current_password: "correct-horse-battery-staple".into(),
                new_password: "new-correct-horse-battery-staple".into(),
            },
        )
        .await
        .unwrap();
    identity
        .refresh(&session.refresh_token, metadata())
        .await
        .unwrap();
    assert!(
        identity
            .refresh(&password_other_session.refresh_token, metadata())
            .await
            .is_err()
    );
    assert!(
        identity
            .login(
                citadel_identity::Login {
                    email_or_name: "owner@example.test".into(),
                    password: "correct-horse-battery-staple".into(),
                },
                metadata(),
            )
            .await
            .is_err()
    );
    let (_, new_password_session) = identity
        .login(
            citadel_identity::Login {
                email_or_name: "owner@example.test".into(),
                password: "new-correct-horse-battery-staple".into(),
            },
            metadata_for(Some("new-password-session")),
        )
        .await
        .unwrap();

    let stale_session_id = Uuid::now_v7();
    assert!(matches!(
        identity_store
            .create_session(
                &NewSession {
                    id: stale_session_id,
                    user_id: owner.subject_id,
                    created_at: Utc::now(),
                    expires_at: Utc::now() + Duration::days(1),
                    metadata: metadata_for(Some("stale-password-login")),
                },
                Some(&stale_password_hash),
                10,
            )
            .await,
        Err(IdentityError::InvalidCredentials)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM refreshtokens WHERE id = $1")
            .bind(stale_session_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );

    profiles
        .change_password(
            &owner,
            None,
            ChangeCurrentPassword {
                current_password: "new-correct-horse-battery-staple".into(),
                new_password: "final-correct-horse-battery-staple".into(),
            },
        )
        .await
        .unwrap();
    for revoked_session in [&session.refresh_token, &new_password_session.refresh_token] {
        assert!(identity.refresh(revoked_session, metadata()).await.is_err());
    }
    identity
        .login(
            citadel_identity::Login {
                email_or_name: "owner@example.test".into(),
                password: "final-correct-horse-battery-staple".into(),
            },
            metadata(),
        )
        .await
        .unwrap();

    let oidc_provider_id = Uuid::now_v7();
    sqlx::query(
        r#"
INSERT INTO oidcproviders
    (id, clientid, createdbyactorid, displayname, issuer, name, scopes, updatedat)
VALUES
    ($1, 'fixture-client', $2, 'Fixture OIDC', 'https://issuer.example.test',
     'fixture-oidc', 'openid profile email', $3)
"#,
    )
    .bind(oidc_provider_id)
    .bind(SYSTEM_ACTOR_ID)
    .bind(Utc::now())
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
INSERT INTO oidcexternallogins
    (id, email, providerid, subject, userid)
VALUES
    ($1, 'other@example.test', $2, 'foreign-subject', $3)
"#,
    )
    .bind(Uuid::now_v7())
    .bind(oidc_provider_id)
    .bind(foreign_user_id)
    .execute(&pool)
    .await
    .unwrap();
    let oidc_principal = ActorPrincipal {
        subject_id: foreign_user_id,
        actor_id: ActorId::new(foreign_actor_id),
        name: "other-user".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    assert!(matches!(
        profiles
            .change_password(
                &oidc_principal,
                None,
                ChangeCurrentPassword {
                    current_password: "ignored-current-password".into(),
                    new_password: "another-secure-password".into(),
                },
            )
            .await,
        Err(IdentityError::Validation(message))
            if message == "Password changes are managed by the identity provider."
    ));

    let protected_resource = Uuid::now_v7();
    let mut account = accounts
        .create(
            CreateServiceAccount {
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
    account = accounts
        .add_role(account.id, ADMIN_ROLE_ID, owner.actor_id)
        .await
        .unwrap();
    assert!(account.roles.iter().any(|role| role.id == ADMIN_ROLE_ID));
    account = accounts
        .remove_role(account.id, ADMIN_ROLE_ID, owner.actor_id)
        .await
        .unwrap();
    assert!(account.roles.iter().all(|role| role.id != ADMIN_ROLE_ID));

    let removable_resource = Uuid::now_v7();
    account = accounts
        .add_resource_access(
            account.id,
            AddServiceAccountResourceAccess {
                resource_type: ResourceType::Platform,
                resource_id: removable_resource,
                permission_level: PermissionLevel::Read,
                specific_permissions: vec![],
            },
            owner.actor_id,
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
        .remove_resource_access(account.id, access_id, owner.actor_id)
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
            CreateServiceAccountToken {
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
                CreateServiceAccountToken {
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
    assert!(matches!(
        profiles.get(&service_principal).await,
        Err(IdentityError::Forbidden)
    ));
    assert!(matches!(
        profiles.get_preferences(&service_principal).await,
        Err(IdentityError::Forbidden)
    ));
    assert!(matches!(
        profiles.list_sessions(&service_principal, None).await,
        Err(IdentityError::Forbidden)
    ));
    assert!(matches!(
        profiles
            .change_password(
                &service_principal,
                None,
                ChangeCurrentPassword {
                    current_password: "ignored-current-password".into(),
                    new_password: "another-secure-password".into(),
                },
            )
            .await,
        Err(IdentityError::Forbidden)
    ));
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
            CreateServiceAccount {
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
                    CreateServiceAccountToken {
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
            ArchiveServiceAccounts {
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
                CreateServiceAccountToken {
                    name: "after-archive".into(),
                    expires_at_utc: None,
                    never_expires: false,
                },
                owner.actor_id,
            )
            .await,
        Err(IdentityError::NotFound)
    ));

    let activities = ActivityService::new(Arc::new(PostgresActivityStore::new(pool.clone())));
    let user_activities = activities
        .list(
            &activity_access(&owner),
            ActivityFilter {
                resource_id: Some(owner.subject_id),
                resource_type: Some(ActivityResourceType::User),
                page_size: Some(100),
                ..ActivityFilter::default()
            },
        )
        .await
        .unwrap();
    for required in [
        ActivityEventType::UserProfileUpdated,
        ActivityEventType::UserPreferencesUpdated,
        ActivityEventType::UserPasswordChanged,
        ActivityEventType::UserSessionRevoked,
        ActivityEventType::UserOtherSessionsRevoked,
    ] {
        assert!(
            user_activities
                .items
                .iter()
                .any(|activity| activity.event_type == required),
            "missing {required:?}"
        );
    }
    assert!(user_activities.items.iter().all(|activity| {
        let normalized = activity.info_json.to_ascii_lowercase();
        !normalized.contains("correct-horse")
            && !normalized.contains("passwordhash")
            && !normalized.contains("refreshtoken")
    }));
    let first_activity = user_activities.items[0].id;
    assert_eq!(
        activities
            .get(&activity_access(&owner), first_activity)
            .await
            .unwrap()
            .id,
        first_activity
    );
    assert_eq!(
        activities
            .list(
                &activity_access(&oidc_principal),
                ActivityFilter {
                    resource_type: Some(ActivityResourceType::User),
                    ..ActivityFilter::default()
                },
            )
            .await
            .unwrap()
            .total_count,
        0
    );
    assert!(matches!(
        activities
            .get(&activity_access(&oidc_principal), first_activity)
            .await,
        Err(citadel_activities::ActivityError::NotFound)
    ));

    pool.close().await;
}

fn metadata() -> SessionMetadata {
    metadata_for(Some("phase3-test"))
}

fn metadata_for(user_agent: Option<&str>) -> SessionMetadata {
    SessionMetadata {
        user_agent: user_agent.map(str::to_owned),
        ip_address: Some("127.0.0.1".into()),
    }
}

fn activity_access(principal: &ActorPrincipal) -> citadel_activities::ActivityAccess {
    citadel_activities::ActivityAccess {
        actor_id: principal.actor_id,
        administrator: principal.is_administrator(),
    }
}

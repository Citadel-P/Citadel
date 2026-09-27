use chrono::Utc;
use citadel_adapters::persistence::postgres::identity::{
    actors::repository::PostgresActorRepository, authentication::store::PostgresIdentityStore,
    roles::repository::PostgresRoleRepository, teams::repository::PostgresTeamRepository,
    users::repository::PostgresUserRepository,
};
use citadel_database::MigrationRunner;
use citadel_identity::*;
use citadel_primitives::{ActorId, PermissionLevel, ResourceType};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

fn queries(family: &str) -> u64 {
    counter(family, "iterations")
}
fn counter(family: &str, metric: &str) -> u64 {
    let mut metrics = String::new();
    citadel_runtime::runtime_metrics::render_runtime_metrics(&mut metrics);
    let prefix = format!("citadel_runtime_{metric}_total{{family=\"{family}\"}} ");
    metrics
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .unwrap()
        .parse()
        .unwrap()
}
fn user(name: &str) -> NewUserMutation {
    let id = Uuid::now_v7();
    NewUserMutation {
        id,
        actor_id: ActorId::new(Uuid::now_v7()),
        name: format!("{name}-{id}"),
        email: format!("{id}@cache.test"),
        password_hash: "fixture".into(),
        is_enabled: true,
        created_by_actor_id: ActorId::new(SYSTEM_ACTOR_ID),
        created_at: Utc::now(),
        team_ids: vec![],
        role_ids: vec![],
        resource_accesses: vec![],
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn committed_mutations_invalidate_only_affected_principals_and_batch_denials_are_cached() {
    let url = std::env::var("CITADEL_PHASE3_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let store = PostgresIdentityStore::new(pool.clone());
    let users = PostgresUserRepository::new(pool.clone());
    let teams = PostgresTeamRepository::new(pool.clone());
    let roles = PostgresRoleRepository::new(pool.clone());
    let alice = user("alice");
    let mut bob = user("bob");
    bob.role_ids.push(ADMIN_ROLE_ID);
    users.create(&alice, true).await.unwrap();
    users.create(&bob, true).await.unwrap();
    let changed_by = ActorId::new(SYSTEM_ACTOR_ID);
    let kind = ResourceType::Platform;
    let ids: Vec<_> = (0..64).map(|_| Uuid::now_v7()).collect();
    let mut batch = ids.clone();
    batch.extend(&ids);
    let before = queries("AuthorizationResourceQuery");
    let denied = store
        .resource_permissions(alice.actor_id, kind, &batch)
        .await
        .unwrap();
    assert_eq!(denied.len(), ids.len());
    assert!(denied.values().all(Option::is_none));
    assert_eq!(queries("AuthorizationResourceQuery") - before, 1);
    let warm_hits = counter("AuthorizationResourceLookup", "units");
    let warm_negatives = counter("AuthorizationNegativeHit", "units");
    for _ in 0..100 {
        assert_eq!(
            store
                .resource_permissions(alice.actor_id, kind, &batch)
                .await
                .unwrap(),
            denied
        );
    }
    assert_eq!(queries("AuthorizationResourceQuery") - before, 1);
    assert_eq!(
        counter("AuthorizationResourceLookup", "units") - warm_hits,
        6400
    );
    assert_eq!(
        counter("AuthorizationNegativeHit", "units") - warm_negatives,
        6400
    );
    println!(
        "authorization gate: cold resource misses=64 ACL SQL=1; warm resource hits=6400 negative hits=6400 ACL SQL=0"
    );
    store.authorization_snapshot(bob.actor_id).await.unwrap();
    let mut alice_watch = store.authorization_changes(alice.actor_id).unwrap();
    let bob_watch = store.authorization_changes(bob.actor_id).unwrap();
    let access = UserResourceAccessInput {
        resource_type: kind,
        resource_id: ids[0],
        permission_level: PermissionLevel::Read,
        specific_permissions: vec![],
    };
    users
        .add_resource_access(alice.id, &access, changed_by, Utc::now(), true)
        .await
        .unwrap();
    assert!(alice_watch.has_changed().unwrap());
    alice_watch.borrow_and_update();
    assert!(
        store
            .resource_permission(alice.actor_id, kind, ids[0])
            .await
            .unwrap()
            .is_some()
    );
    users
        .remove_resource_access(alice.id, &access, changed_by, Utc::now())
        .await
        .unwrap();
    assert!(
        store
            .resource_permission(alice.actor_id, kind, ids[0])
            .await
            .unwrap()
            .is_none()
    );
    let role = NewRoleMutation {
        id: Uuid::now_v7(),
        name: format!("cache-role-{}", Uuid::now_v7()),
        permissions: vec![RolePermissionDetails {
            resource_type: kind,
            permission_level: PermissionLevel::Write,
            specific_permissions: vec![],
        }],
        changed_by_actor_id: changed_by,
        changed_at: Utc::now(),
    };
    roles.create(&role).await.unwrap();
    users
        .add_role(alice.id, role.id, changed_by, Utc::now(), true)
        .await
        .unwrap();
    assert!(
        store
            .authorization_snapshot(alice.actor_id)
            .await
            .unwrap()
            .permits(kind, PermissionLevel::Write, None)
    );
    users
        .remove_role(alice.id, role.id, changed_by, Utc::now())
        .await
        .unwrap();
    assert!(
        !store
            .authorization_snapshot(alice.actor_id)
            .await
            .unwrap()
            .permits(kind, PermissionLevel::Read, None)
    );

    let team = NewTeamMutation {
        id: Uuid::now_v7(),
        actor_id: ActorId::new(Uuid::now_v7()),
        name: format!("cache-team-{}", Uuid::now_v7()),
        created_by_actor_id: changed_by,
        created_at: Utc::now(),
        user_ids: vec![alice.id],
        role_ids: vec![role.id],
        resource_accesses: vec![],
    };
    teams.create(&team, true).await.unwrap();
    assert!(
        store
            .authorization_snapshot(alice.actor_id)
            .await
            .unwrap()
            .permits(kind, PermissionLevel::Write, None)
    );
    roles
        .patch_permissions(role.id, Some(&[]), changed_by, Utc::now(), true)
        .await
        .unwrap();
    assert!(
        !store
            .authorization_snapshot(alice.actor_id)
            .await
            .unwrap()
            .permits(kind, PermissionLevel::Read, None)
    );
    roles
        .patch_permissions(
            role.id,
            Some(&role.permissions),
            changed_by,
            Utc::now(),
            true,
        )
        .await
        .unwrap();
    assert!(
        store
            .resource_permission(alice.actor_id, kind, ids[1])
            .await
            .unwrap()
            .is_some()
    );
    teams
        .patch(
            team.id,
            &TeamPatchMutation {
                user_ids: Some(vec![]),
                ..Default::default()
            },
            changed_by,
            Utc::now(),
            true,
        )
        .await
        .unwrap();
    assert!(
        store
            .resource_permission(alice.actor_id, kind, ids[1])
            .await
            .unwrap()
            .is_none(),
        "removed member must be captured before replacement"
    );
    teams
        .add_member(team.id, alice.actor_id, changed_by, Utc::now(), true)
        .await
        .unwrap();
    assert!(
        store
            .authorization_snapshot(alice.actor_id)
            .await
            .unwrap()
            .permits(kind, PermissionLevel::Read, None)
    );
    // Team enablement removes inherited access without touching Alice's row.
    let actors = PostgresActorRepository::new(pool.clone());
    actors
        .set_enabled(team.actor_id.value(), false)
        .await
        .unwrap();
    assert!(
        !store
            .authorization_snapshot(alice.actor_id)
            .await
            .unwrap()
            .permits(kind, PermissionLevel::Read, None)
    );
    actors
        .set_enabled(team.actor_id.value(), true)
        .await
        .unwrap();
    assert!(
        store
            .authorization_snapshot(alice.actor_id)
            .await
            .unwrap()
            .permits(kind, PermissionLevel::Read, None)
    );
    roles
        .delete(&[role.id], changed_by, Utc::now())
        .await
        .unwrap();
    assert!(
        !store
            .authorization_snapshot(alice.actor_id)
            .await
            .unwrap()
            .permits(kind, PermissionLevel::Read, None),
        "deleted role must retain pre-cascade impact"
    );
    teams
        .add_resource_access(
            team.id,
            &TeamResourceAccessInput {
                resource_type: kind,
                resource_id: ids[2],
                permission_level: PermissionLevel::Read,
                specific_permissions: vec![],
            },
            changed_by,
            Utc::now(),
            true,
        )
        .await
        .unwrap();
    assert!(
        store
            .resource_permission(alice.actor_id, kind, ids[2])
            .await
            .unwrap()
            .is_some()
    );
    teams
        .delete(&[team.id], changed_by, Utc::now())
        .await
        .unwrap();
    assert!(
        store
            .resource_permission(alice.actor_id, kind, ids[2])
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        !bob_watch.has_changed().unwrap(),
        "Alice mutations must not flush Bob"
    );
    let before = queries("AuthorizationScopeQuery");
    store.authorization_snapshot(bob.actor_id).await.unwrap();
    assert_eq!(queries("AuthorizationScopeQuery"), before);
    alice_watch.borrow_and_update();
    assert!(
        users
            .add_role(alice.id, Uuid::now_v7(), changed_by, Utc::now(), true)
            .await
            .is_err()
    );
    assert!(
        !alice_watch.has_changed().unwrap(),
        "rollback must not invalidate"
    );
    let accounts = citadel_adapters::persistence::postgres::identity::service_accounts::repository::PostgresServiceAccountRepository::new(pool.clone());
    let account = NewServiceAccount {
        id: Uuid::now_v7(),
        actor_id: ActorId::new(Uuid::now_v7()),
        name: format!("cache-account-{}", Uuid::now_v7()),
        description: None,
        is_enabled: true,
        created_by_actor_id: changed_by,
        created_at: Utc::now(),
        team_ids: vec![],
        role_ids: vec![],
        resource_accesses: vec![],
    };
    accounts.create(&account).await.unwrap();
    assert!(
        store
            .resource_permission(account.actor_id, kind, ids[0])
            .await
            .unwrap()
            .is_none()
    );
    let access = ServiceAccountResourceAccess {
        id: Some(Uuid::now_v7()),
        resource_name: None,
        resource_type: kind,
        resource_id: ids[0],
        permission_level: PermissionLevel::Read,
        specific_permissions: vec![],
    };
    accounts
        .add_resource_access(account.id, &access, changed_by, Utc::now())
        .await
        .unwrap();
    assert!(
        store
            .resource_permission(account.actor_id, kind, ids[0])
            .await
            .unwrap()
            .is_some()
    );
    accounts
        .archive(&[account.id], changed_by, Utc::now())
        .await
        .unwrap();
    assert!(
        store
            .resource_permission(account.actor_id, kind, ids[0])
            .await
            .unwrap()
            .is_none()
    );
    let actors = PostgresActorRepository::new(pool.clone());
    actors
        .set_enabled(alice.actor_id.value(), false)
        .await
        .unwrap();
    assert!(
        !store
            .authorization_snapshot(alice.actor_id)
            .await
            .unwrap()
            .enabled
    );
    actors
        .set_enabled(alice.actor_id.value(), true)
        .await
        .unwrap();
    store
        .resource_permissions(alice.actor_id, kind, &ids)
        .await
        .unwrap();
    let bob_positive = store
        .resource_permission(bob.actor_id, kind, ids[0])
        .await
        .unwrap();
    assert!(bob_positive.is_some());
    let identity = identity_service(store.clone());
    pool.close().await;
    let bob_principal = ActorPrincipal {
        subject_id: bob.id,
        actor_id: bob.actor_id,
        name: bob.name.clone(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    // This ID has never been looked up. A sufficient global grant skips ACL SQL.
    identity
        .authorize_resource(
            &bob_principal,
            kind,
            Uuid::now_v7(),
            PermissionLevel::Read,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .resource_permission(bob.actor_id, kind, ids[0])
            .await
            .unwrap(),
        bob_positive
    );

    // Actual closed pool proves positive scope and negative ACL hits need no SQL.
    assert!(
        store
            .authorization_snapshot(bob.actor_id)
            .await
            .unwrap()
            .enabled
    );
    assert!(
        store
            .resource_permissions(alice.actor_id, kind, &ids)
            .await
            .unwrap()
            .values()
            .all(Option::is_none)
    );
    println!(
        "64 duplicated cold IDs: 1 ACL query; 100 warm batches: 0 ACL queries; Bob stays warm across Alice/Team/Role mutations"
    );
}

fn identity_service(store: PostgresIdentityStore) -> IdentityService {
    use citadel_adapters::persistence::postgres::identity::authentication::store::StaticEntitlementService;
    use citadel_adapters::security::identity::crypto::{
        Argon2PasswordHasher, JwtSessionTokenCodec, OpaqueServiceAccountTokenCodec,
    };
    use std::sync::Arc;
    IdentityService::new(
        Arc::new(store),
        Arc::new(Argon2PasswordHasher::default()),
        Arc::new(
            JwtSessionTokenCodec::new(&[7_u8; 32], "fixture".into(), "fixture".into()).unwrap(),
        ),
        Arc::new(OpaqueServiceAccountTokenCodec),
        Arc::new(StaticEntitlementService::new(true)),
        Arc::new(SystemClock),
        Arc::new(NoopServiceAccountLastUsedTracker),
        chrono::Duration::minutes(15),
        chrono::Duration::days(30),
    )
}

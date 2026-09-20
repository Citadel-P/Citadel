use std::collections::HashMap;

use citadel_adapters::persistence::postgres::identity::authentication::store::PostgresIdentityStore;
use citadel_database::MigrationRunner;
use citadel_identity::AuthenticatedPrincipalType;
use citadel_identity::{ActorPrincipal, IdentityStore, PermissionGrant, permission_matrix};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use uuid::Uuid;

const AUTHORIZATION_CASES: &str =
    include_str!("../../../../../test/fixtures/identity-authorization-cases.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizationMatrix {
    schema_version: u32,
    permission_matrix: HashMap<ResourceType, ExpectedPermissionCapability>,
    cases: Vec<AuthorizationCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpectedPermissionCapability {
    maximum_level: PermissionLevel,
    specifics: HashMap<SpecificPermission, PermissionLevel>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizationCase {
    name: String,
    principal_type: FixturePrincipalType,
    roles: Vec<String>,
    actor_enabled: bool,
    team_enabled: Option<bool>,
    direct_role_grants: Vec<RoleGrant>,
    team_role_grants: Vec<RoleGrant>,
    direct_resource_grants: Vec<ResourceGrant>,
    team_resource_grants: Vec<ResourceGrant>,
    expected_administrator: bool,
    probes: Vec<AuthorizationProbe>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
enum FixturePrincipalType {
    User,
    ServiceAccount,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RoleGrant {
    resource_type: ResourceType,
    level: PermissionLevel,
    specifics: Vec<SpecificPermission>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceGrant {
    resource: ResourceTarget,
    resource_type: ResourceType,
    level: PermissionLevel,
    specifics: Vec<SpecificPermission>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
enum ResourceTarget {
    Target,
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthorizationProbe {
    scope: ProbeScope,
    resource_type: ResourceType,
    level: PermissionLevel,
    specific: Option<SpecificPermission>,
    expected: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
enum ProbeScope {
    Global,
    Target,
    Other,
}

#[test]
fn rust_permission_matrix_matches_the_shared_dotnet_contract() {
    let expected: AuthorizationMatrix = serde_json::from_str(AUTHORIZATION_CASES).unwrap();
    assert_eq!(expected.schema_version, 1);
    let actual = permission_matrix();
    assert_eq!(actual.len(), expected.permission_matrix.len());

    for (resource_type, expected_capability) in expected.permission_matrix {
        let capability = actual
            .get(&resource_type)
            .unwrap_or_else(|| panic!("missing {resource_type:?}"));
        assert_eq!(
            capability.maximum_level, expected_capability.maximum_level,
            "{resource_type:?} maximum level"
        );
        let specifics = capability
            .specifics
            .iter()
            .copied()
            .collect::<HashMap<_, _>>();
        assert_eq!(
            specifics, expected_capability.specifics,
            "{resource_type:?} specific permissions"
        );
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE3_DATABASE_URL"]
async fn postgres_authorization_matches_the_shared_dotnet_matrix() {
    let database_url = std::env::var("CITADEL_PHASE3_DATABASE_URL")
        .expect("CITADEL_PHASE3_DATABASE_URL is required for this fixture");
    MigrationRunner::migrate(&database_url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .unwrap();
    let store = PostgresIdentityStore::new(pool.clone());
    let matrix: AuthorizationMatrix = serde_json::from_str(AUTHORIZATION_CASES).unwrap();
    assert_eq!(matrix.schema_version, 1);

    for case in matrix.cases {
        verify_case(&pool, &store, case).await;
    }

    pool.close().await;
}

async fn verify_case(pool: &PgPool, store: &PostgresIdentityStore, case: AuthorizationCase) {
    let actor_id = Uuid::now_v7();
    let subject_id = Uuid::now_v7();
    let target_id = Uuid::now_v7();
    let other_id = Uuid::now_v7();
    let mut transaction = pool.begin().await.unwrap();

    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, $2, $3)")
        .bind(actor_id)
        .bind(case.actor_enabled)
        .bind(match case.principal_type {
            FixturePrincipalType::User => "User",
            FixturePrincipalType::ServiceAccount => "ServiceAccount",
        })
        .execute(&mut *transaction)
        .await
        .unwrap();

    insert_role_grants(
        &mut transaction,
        actor_id,
        &case.direct_role_grants,
        &case.name,
    )
    .await;
    insert_resource_grants(
        &mut transaction,
        actor_id,
        &case.direct_resource_grants,
        target_id,
        other_id,
    )
    .await;

    if case.team_enabled.is_some()
        || !case.team_role_grants.is_empty()
        || !case.team_resource_grants.is_empty()
    {
        let team_actor_id = Uuid::now_v7();
        let team_id = Uuid::now_v7();
        sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, $2, 'Team')")
            .bind(team_actor_id)
            .bind(case.team_enabled.unwrap_or(true))
            .execute(&mut *transaction)
            .await
            .unwrap();
        sqlx::query("INSERT INTO teams (id, actorid, name) VALUES ($1, $2, $3)")
            .bind(team_id)
            .bind(team_actor_id)
            .bind(format!("differential-team-{team_id}"))
            .execute(&mut *transaction)
            .await
            .unwrap();
        sqlx::query("INSERT INTO actorteammemberships (memberactorid, teamid) VALUES ($1, $2)")
            .bind(actor_id)
            .bind(team_id)
            .execute(&mut *transaction)
            .await
            .unwrap();
        insert_role_grants(
            &mut transaction,
            team_actor_id,
            &case.team_role_grants,
            &case.name,
        )
        .await;
        insert_resource_grants(
            &mut transaction,
            team_actor_id,
            &case.team_resource_grants,
            target_id,
            other_id,
        )
        .await;
    }

    transaction.commit().await.unwrap();

    let principal = ActorPrincipal {
        subject_id,
        actor_id: ActorId::new(actor_id),
        name: case.name.clone(),
        principal_type: match case.principal_type {
            FixturePrincipalType::User => AuthenticatedPrincipalType::User,
            FixturePrincipalType::ServiceAccount => AuthenticatedPrincipalType::ServiceAccount,
        },
        credential_id: None,
        roles: case.roles.clone(),
    };
    assert_eq!(
        principal.is_administrator(),
        case.expected_administrator,
        "{}: administrator boundary",
        case.name
    );

    let snapshot = store
        .authorization_snapshot(ActorId::new(actor_id))
        .await
        .unwrap();
    for probe in case.probes {
        let actual = match probe.scope {
            ProbeScope::Global => {
                snapshot.permits(probe.resource_type, probe.level, probe.specific)
            }
            ProbeScope::Target | ProbeScope::Other => {
                let resource_id = match probe.scope {
                    ProbeScope::Target => target_id,
                    ProbeScope::Other => other_id,
                    ProbeScope::Global => unreachable!(),
                };
                store
                    .resource_permission(ActorId::new(actor_id), probe.resource_type, resource_id)
                    .await
                    .unwrap()
                    .is_some_and(|grant| grants(grant, probe.level, probe.specific))
            }
        };
        assert_eq!(actual, probe.expected, "{}: {probe:?}", case.name);
    }
}

async fn insert_role_grants(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: Uuid,
    grants: &[RoleGrant],
    case_name: &str,
) {
    for (index, grant) in grants.iter().enumerate() {
        let role_id = Uuid::now_v7();
        sqlx::query("INSERT INTO roles (id, name, roletype) VALUES ($1, $2, 'Custom')")
            .bind(role_id)
            .bind(format!(
                "differential-{case_name}-{index}-{}",
                Uuid::now_v7()
            ))
            .execute(&mut **transaction)
            .await
            .unwrap();
        sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
            .bind(actor_id)
            .bind(role_id)
            .execute(&mut **transaction)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(Uuid::now_v7())
        .bind(grant.level as i32)
        .bind(grant.resource_type as i32)
        .bind(role_id)
        .bind(specific_mask(&grant.specifics))
        .execute(&mut **transaction)
        .await
        .unwrap();
    }
}

async fn insert_resource_grants(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: Uuid,
    grants: &[ResourceGrant],
    target_id: Uuid,
    other_id: Uuid,
) {
    for grant in grants {
        let resource_id = match grant.resource {
            ResourceTarget::Target => target_id,
            ResourceTarget::Other => other_id,
        };
        sqlx::query(
            "INSERT INTO resourceaccesses (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(Uuid::now_v7())
        .bind(actor_id)
        .bind(grant.level as i32)
        .bind(resource_id)
        .bind(grant.resource_type as i32)
        .bind(specific_mask(&grant.specifics))
        .execute(&mut **transaction)
        .await
        .unwrap();
    }
}

fn specific_mask(specifics: &[SpecificPermission]) -> i32 {
    specifics
        .iter()
        .fold(0, |mask, permission| mask | *permission as i32)
}

fn grants(
    grant: PermissionGrant,
    level: PermissionLevel,
    specific: Option<SpecificPermission>,
) -> bool {
    grant.level.grants(level) && specific.is_none_or(|permission| grant.has_specific(permission))
}

use crate::RoleType;
use crate::UserDateTimeFormat;
use crate::UserTheme;
use crate::*;
use chrono::Utc;
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use uuid::Uuid;

#[test]
fn user_mutations_preserve_identity_and_audit_state() {
    let actor_id = ActorId::new(Uuid::now_v7());
    let created_by = ActorId::new(Uuid::now_v7());
    let created_at = Utc::now();
    let mut user = User::new(
        "owner".to_owned(),
        "owner@example.test".to_owned(),
        Some("hash-v1".to_owned()),
        actor_id,
        created_by,
        created_at,
    );
    let id = user.id();

    user.rename("new-owner".to_owned());
    user.set_password_hash("hash-v2".to_owned());

    assert_eq!(user.id(), id);
    assert_eq!(user.actor_id(), actor_id);
    assert_eq!(user.created_by_actor_id(), created_by);
    assert_eq!(user.created_at(), created_at);
    assert_eq!(user.name(), "new-owner");
    assert_eq!(user.password_hash(), Some("hash-v2"));
}

#[test]
fn team_rename_preserves_resource_and_actor_identity() {
    let actor_id = ActorId::new(Uuid::now_v7());
    let mut team = Team::new("operations".to_owned(), actor_id);
    let id = team.id();

    team.rename("platform operations".to_owned());

    assert_eq!(team.id(), id);
    assert_eq!(team.actor_id(), actor_id);
    assert_eq!(team.name(), "platform operations");
}

#[test]
fn system_roles_reject_mutation_while_custom_roles_preserve_identity() {
    let permission = RolePermission::new(ResourceType::Registry, PermissionLevel::Read, Vec::new());
    let mut custom = Role::new_custom("Operator".to_owned(), vec![permission.clone()]);
    let custom_id = custom.id();
    custom.rename("Custom operator".to_owned()).unwrap();
    custom.set_permissions(Vec::new()).unwrap();
    assert_eq!(custom.id(), custom_id);
    assert_eq!(custom.name(), "Custom operator");
    assert!(custom.permissions().is_empty());

    let mut system = Role::from_persistence(
        ADMIN_ROLE_ID,
        "Admin".to_owned(),
        RoleType::System,
        vec![permission],
    );
    assert_eq!(
        system.rename("Other".to_owned()),
        Err(RoleMutationError::SystemRole)
    );
    assert_eq!(
        system.set_permissions(Vec::new()),
        Err(RoleMutationError::SystemRole)
    );
}

#[test]
fn preference_updates_preserve_user_ownership() {
    let user_id = Uuid::now_v7();
    let created_at = Utc::now();
    let updated_at = created_at + chrono::Duration::minutes(1);
    let mut preferences = UserPreferences::new(
        user_id,
        "UTC".to_owned(),
        UserDateTimeFormat::System,
        UserTheme::System,
        created_at,
    );

    preferences.update(
        "Europe/Paris".to_owned(),
        UserDateTimeFormat::TwentyFourHour,
        UserTheme::Dark,
        updated_at,
    );

    assert_eq!(preferences.user_id(), user_id);
    assert_eq!(preferences.time_zone(), "Europe/Paris");
    assert_eq!(
        preferences.date_time_format(),
        UserDateTimeFormat::TwentyFourHour
    );
    assert_eq!(preferences.theme(), UserTheme::Dark);
    assert_eq!(preferences.updated_at(), updated_at);
}

#[test]
fn service_account_sensitive_permissions_are_explicit() {
    let matrix = permission_matrix();
    let capabilities = &matrix[&ResourceType::ServiceAccount];
    assert_eq!(
        capabilities.specifics,
        &[
            (SpecificPermission::Use, PermissionLevel::Read),
            (SpecificPermission::ManageCredentials, PermissionLevel::Read)
        ]
    );
    assert!(
        !matrix[&ResourceType::User]
            .specifics
            .iter()
            .any(|(permission, _)| *permission == SpecificPermission::ManageCredentials)
    );
}

#[test]
fn authorization_requires_level_and_specific_bit() {
    let actor = ActorId::new(Uuid::now_v7());
    let snapshot = AuthorizationSnapshot {
        actor_id: actor,
        enabled: true,
        direct_and_team_permissions: vec![PermissionGrant {
            resource_type: ResourceType::ServiceAccount,
            level: PermissionLevel::Read,
            specifics: SpecificPermission::Use.into(),
        }],
    };
    assert!(snapshot.permits(
        ResourceType::ServiceAccount,
        PermissionLevel::Read,
        Some(SpecificPermission::Use)
    ));
    assert!(!snapshot.permits(
        ResourceType::ServiceAccount,
        PermissionLevel::Read,
        Some(SpecificPermission::ManageCredentials)
    ));
    assert!(!snapshot.permits(ResourceType::ServiceAccount, PermissionLevel::Write, None));
}

use citadel_adapters::PostgresAuthorizedPlatformReader;
use citadel_platforms::AuthorizedPlatformReader;
use citadel_primitives::ActorId;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE0_DATABASE_URL"]
async fn actor_authorized_platform_read_preserves_direct_and_team_scope() {
    let database_url = std::env::var("CITADEL_PHASE0_DATABASE_URL")
        .expect("CITADEL_PHASE0_DATABASE_URL is required for this fixture");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .unwrap();

    sqlx::raw_sql(
        r#"
        CREATE TEMP TABLE Actors (Id uuid PRIMARY KEY, Type text NOT NULL, IsEnabled boolean NOT NULL);
        CREATE TEMP TABLE Users (Id uuid PRIMARY KEY, ActorId uuid NOT NULL, Name text NOT NULL);
        CREATE TEMP TABLE Teams (Id uuid PRIMARY KEY, ActorId uuid NOT NULL, Name text NOT NULL);
        CREATE TEMP TABLE ActorTeamMemberships (TeamId uuid NOT NULL, MemberActorId uuid NOT NULL);
        CREATE TEMP TABLE Roles (Id uuid PRIMARY KEY, Name text NOT NULL, RoleType text NOT NULL);
        CREATE TEMP TABLE ActorRoles (ActorId uuid NOT NULL, RoleId uuid NOT NULL);
        CREATE TEMP TABLE Permissions (Id uuid PRIMARY KEY, RoleId uuid NOT NULL, ResourceType integer NOT NULL, PermissionLevel integer NOT NULL, SpecificPermissions integer NOT NULL);
        CREATE TEMP TABLE ResourceAccesses (Id uuid PRIMARY KEY, ResourceId uuid NOT NULL, ActorId uuid NOT NULL, ResourceType integer NOT NULL, PermissionLevel integer NOT NULL, SpecificPermissions integer NOT NULL);
        CREATE TEMP TABLE Platforms (Id uuid PRIMARY KEY, Name text NOT NULL, Address text NOT NULL, Status text NOT NULL, ConnectorType text NOT NULL);
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let user_id = Uuid::now_v7();
    let user_actor_id = Uuid::now_v7();
    let disabled_actor_id = Uuid::now_v7();
    let team_id = Uuid::now_v7();
    let team_actor_id = Uuid::now_v7();
    let direct_platform_id = Uuid::now_v7();
    let team_platform_id = Uuid::now_v7();
    let denied_platform_id = Uuid::now_v7();

    sqlx::query("INSERT INTO Actors (Id, Type, IsEnabled) VALUES ($1, 'User', true), ($2, 'User', false), ($3, 'Team', true)")
        .bind(user_actor_id)
        .bind(disabled_actor_id)
        .bind(team_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO Users (Id, ActorId, Name) VALUES ($1, $2, 'fixture-user')")
        .bind(user_id)
        .bind(user_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO Teams (Id, ActorId, Name) VALUES ($1, $2, 'fixture-team')")
        .bind(team_id)
        .bind(team_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ActorTeamMemberships (TeamId, MemberActorId) VALUES ($1, $2)")
        .bind(team_id)
        .bind(user_actor_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO Platforms (Id, Name, Address, Status, ConnectorType) VALUES ($1, 'direct', 'unix:///direct', 'Healthy', 'Local'), ($2, 'team', 'unix:///team', 'Healthy', 'Local'), ($3, 'denied', 'unix:///denied', 'Healthy', 'Local')")
        .bind(direct_platform_id)
        .bind(team_platform_id)
        .bind(denied_platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO ResourceAccesses (Id, ResourceId, ActorId, ResourceType, PermissionLevel, SpecificPermissions) VALUES ($1, $2, $3, 0, 1, 0), ($4, $5, $6, 0, 1, 0), ($7, $8, $9, 0, 1, 0)")
        .bind(Uuid::now_v7())
        .bind(direct_platform_id)
        .bind(user_actor_id)
        .bind(Uuid::now_v7())
        .bind(team_platform_id)
        .bind(team_actor_id)
        .bind(Uuid::now_v7())
        .bind(denied_platform_id)
        .bind(disabled_actor_id)
        .execute(&pool)
        .await
        .unwrap();

    let reader = PostgresAuthorizedPlatformReader::new(pool.clone());
    let by_user_id = reader.list_authorized(ActorId::new(user_id)).await.unwrap();
    let by_actor_id = reader
        .list_authorized(ActorId::new(user_actor_id))
        .await
        .unwrap();

    assert_eq!(
        by_user_id
            .iter()
            .map(|platform| platform.name.as_str())
            .collect::<Vec<_>>(),
        vec!["direct", "team"]
    );
    assert_eq!(by_user_id, by_actor_id);
    assert!(
        reader
            .list_authorized(ActorId::new(disabled_actor_id))
            .await
            .unwrap()
            .is_empty(),
        "disabled Actors must not retain direct resource access"
    );
    assert!(
        !by_user_id
            .iter()
            .any(|platform| platform.id == denied_platform_id)
    );

    pool.close().await;
}

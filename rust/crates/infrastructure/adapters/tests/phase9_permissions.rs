use citadel_adapters::postgres::{
    alerts::PostgresAlertRepository, automation::PostgresAutomationRepository,
};
use citadel_alerts::{AlertChannelConfiguration, AlertRepository};
use citadel_automation::{AutomationActionConfiguration, AutomationRepository};
use citadel_database::MigrationRunner;
use citadel_primitives::{ActorId, PermissionLevel, ResourceType};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires disposable CITADEL_PHASE7_DATABASE_URL"]
async fn migrated_lists_and_capabilities_reject_invalid_permission_levels() {
    let url = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let actor = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(actor.value())
        .execute(&pool)
        .await
        .unwrap();
    let alerts = PostgresAlertRepository::new(pool.clone());
    let channel = alerts
        .create_channel(
            actor,
            &AlertChannelConfiguration {
                name: format!("ordinal-{}", actor.value()),
                alert_destination: "Generic".into(),
                url: "https://example.test/hook".into(),
                is_active: true,
            },
        )
        .await
        .unwrap();
    let automation = PostgresAutomationRepository::new(pool.clone());
    let mut input: AutomationActionConfiguration = serde_json::from_value(json!({
        "name":format!("ordinal-{}", actor.value()), "code":"console.log('ok')",
        "enabled":true,"scheduleEnabled":false,"alertOnFailure":false
    }))
    .unwrap();
    input.validate(actor).unwrap();
    let action = automation.create(actor, &input).await.unwrap();
    // Create grants are unrelated to this reader: test only the explicit ACLs.
    let reader = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'System')")
        .bind(reader.value())
        .execute(&pool)
        .await
        .unwrap();
    for (kind, id) in [
        (ResourceType::AlertChannel, channel.id),
        (ResourceType::AutomationAction, action.id),
    ] {
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,0,$3,$4,0)")
            .bind(Uuid::now_v7()).bind(reader.value()).bind(id).bind(kind as i32)
            .execute(&pool).await.unwrap();
    }
    for (raw, visible, level) in [
        (0, false, PermissionLevel::None),
        (1, true, PermissionLevel::Read),
        (2, true, PermissionLevel::Write),
        (4, true, PermissionLevel::Execute),
        (3, false, PermissionLevel::None),
        (7, false, PermissionLevel::None),
    ] {
        sqlx::query("UPDATE resourceaccesses SET permissionlevel=$2 WHERE actorid=$1")
            .bind(reader.value())
            .bind(raw)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            alerts
                .list_channels(reader, false)
                .await
                .unwrap()
                .iter()
                .any(|c| c.id == channel.id),
            visible,
            "Alert level {raw}"
        );
        assert_eq!(
            automation
                .list(reader, false)
                .await
                .unwrap()
                .iter()
                .any(|a| a.id == action.id),
            visible,
            "Automation level {raw}"
        );
        assert_eq!(
            automation.permissions(reader, &[action.id]).await.unwrap()[&action.id],
            level
        );
    }
    pool.close().await;
}

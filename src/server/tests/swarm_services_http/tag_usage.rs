use super::*;
use citadel_adapters::persistence::postgres::{
    notifications::PostgresNotifications,
    tags::{PostgresTagRepository, links::USAGE_CHANGED_CHANNEL},
};
use citadel_runtime::notifications::{NotificationSource, RuntimeNotification};
use citadel_tags::{NewTag, TagRepository, TaggableResourceType};

pub(super) async fn partial_deletion(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    platform: Uuid,
    first: Uuid,
    spec: &Value,
) -> Uuid {
    let created = request(
        app,
        Method::POST,
        "/api/v1/swarmServices",
        Some(admin.clone()),
        Some(json!({"name":format!("second-{first}"),"platformId":platform,"spec":spec})),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    let second = Uuid::parse_str(response_json(created).await["id"].as_str().unwrap()).unwrap();
    sqlx::query("UPDATE swarmservices SET dockerserviceid='reject-delete' WHERE id=$1")
        .bind(second)
        .execute(pool)
        .await
        .unwrap();
    let tags = PostgresTagRepository::new(pool.clone());
    let tag = tags
        .create_tag(
            admin.actor_id,
            &NewTag {
                name: format!("partial-{first}"),
                color: "#445566".into(),
            },
        )
        .await
        .unwrap();
    for id in [first, second] {
        tags.replace_resource_tags(
            admin.actor_id,
            TaggableResourceType::SwarmService,
            id,
            &[tag.id],
        )
        .await
        .unwrap();
    }
    let count = || async {
        tags.list_tags(admin.actor_id, true)
            .await
            .unwrap()
            .into_iter()
            .find(|t| t.id == tag.id)
            .unwrap()
            .usage_count
    };
    assert_eq!(count().await, 2);
    let mut catalogue = PostgresNotifications::connect(pool, USAGE_CHANGED_CHANNEL)
        .await
        .unwrap();
    let response = request(
        app,
        Method::DELETE,
        "/api/v1/swarmServices",
        Some(admin.clone()),
        Some(json!([first, second])),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(3), catalogue.recv())
            .await
            .unwrap()
            .unwrap(),
        RuntimeNotification::TagUsageChanged
    ));
    assert_eq!(count().await, 1);
    assert!(
        tags.get_resource_tags(TaggableResourceType::SwarmService, second)
            .await
            .unwrap()
            .iter()
            .any(|t| t.id == tag.id)
    );
    sqlx::query("UPDATE swarmservices SET dockerserviceid=NULL WHERE id=$1")
        .bind(second)
        .execute(pool)
        .await
        .unwrap();
    tags.delete_tag(tag.id).await.unwrap();
    second
}

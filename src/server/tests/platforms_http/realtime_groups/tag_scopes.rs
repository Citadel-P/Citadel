use super::*;

pub(super) async fn check_platform_scopes(
    f: &Fixture,
    socket: &mut Socket,
    hub: &citadel_server::realtime::RealtimeHub,
) {
    let prod = Uuid::now_v7();
    let stage = Uuid::now_v7();
    let initial_name: String = sqlx::query_scalar("SELECT name FROM tags WHERE id=$1")
        .bind(f.tag_id)
        .fetch_one(&f.pool)
        .await
        .unwrap();
    let staging_name = format!("stage-{}", f.tag_id);
    let join = |id, name: &str| json!([{"subscriptionId":id, "group":"platforms", "tags":[name]}]);
    assert!(invoke_args(socket, "JoinGroup", join(prod, &initial_name)).await["error"].is_null());
    assert_eq!(receive(socket).await["target"], "CollectionSnapshotStarted");
    let first = receive(socket).await;
    assert_eq!(first["subscriptionId"], json!(prod));
    assert_eq!(first["arguments"][0]["id"], json!(f.platform_id));
    assert!(invoke_args(socket, "JoinGroup", join(stage, &staging_name)).await["error"].is_null());
    let empty = receive(socket).await;
    assert_eq!(empty["subscriptionId"], json!(stage));
    assert_eq!(empty["target"], "CollectionSnapshotStarted");
    assert!(invoke(socket, "JoinGroup", "platforms").await["error"].is_null());
    let all = receive(socket).await;
    assert_eq!(all["target"], "PlatformUpdated");
    assert!(all.get("subscriptionId").is_none());
    for (new_name, leaving, entering) in
        [(&staging_name, prod, stage), (&initial_name, stage, prod)]
    {
        sqlx::query("UPDATE tags SET name=$2, normalizedname=lower($2) WHERE id=$1")
            .bind(f.tag_id)
            .bind(new_name)
            .execute(&f.pool)
            .await
            .unwrap();
        hub.publish_tag_change(&[citadel_tags::TaggableResourceType::Platform]);
        let mut frames = Vec::new();
        for _ in 0..4 {
            frames.push(receive(socket).await);
        }
        assert!(frames.iter().any(|e| e["target"] == "TagCatalogChanged"));
        assert!(frames.iter().any(|e| e["subscriptionId"] == json!(leaving)
            && e["target"] == "PlatformsDeleted"
            && e["arguments"][0] == json!(f.platform_id)));
        assert!(frames.iter().any(|e| e["subscriptionId"] == json!(entering)
            && e["target"] == "PlatformUpdated"
            && e["arguments"][0]["id"] == json!(f.platform_id)));
        assert!(
            frames
                .iter()
                .any(|e| e.get("subscriptionId").is_none() && e["target"] == "PlatformUpdated")
        );
    }
    for name in [prod.to_string(), stage.to_string(), "platforms".into()] {
        assert!(invoke(socket, "LeaveGroup", &name).await["error"].is_null());
    }
}

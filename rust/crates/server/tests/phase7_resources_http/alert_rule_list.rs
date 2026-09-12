use super::*;

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    administrator: &ActorPrincipal,
    rule: &Value,
    channel: &Value,
) {
    let listed = list(app, administrator).await;
    let rules = listed["alertRules"].as_array().unwrap();
    assert!(
        rules.iter().all(|rule| rule["channels"].is_array()),
        "Every table row must include a channels array"
    );
    assert!(rules.iter().any(|rule| {
        rule["type"] == "SwarmServiceOperationFailed" && rule["channels"] == json!([])
    }));
    let listed_rule = rules.iter().find(|item| item["id"] == rule["id"]).unwrap();
    assert_eq!(listed_rule["channels"], json!([channel]));

    let reader = seed_regular_user(pool).await;
    assert_eq!(list(app, &reader).await["alertRules"], json!([]));
    grant_read(pool, &reader, ResourceType::Alert, rule).await;
    let listed = list(app, &reader).await;
    assert_eq!(listed["alertRules"].as_array().unwrap().len(), 1);
    assert_eq!(listed["alertRules"][0]["id"], rule["id"]);
    assert_eq!(
        listed["alertRules"][0]["channels"],
        json!([]),
        "Reading a rule must not expose an unreadable linked channel"
    );

    grant_read(pool, &reader, ResourceType::AlertChannel, channel).await;
    assert_eq!(
        list(app, &reader).await["alertRules"][0]["channels"],
        json!([channel])
    );
}

async fn list(app: &Router, principal: &ActorPrincipal) -> Value {
    response_json(
        request(
            app,
            Method::GET,
            "/api/v1/alertRules",
            Some(principal.clone()),
            None,
        )
        .await,
    )
    .await
}

async fn grant_read(
    pool: &sqlx::PgPool,
    principal: &ActorPrincipal,
    resource_type: ResourceType,
    resource: &Value,
) {
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,$4,0)")
        .bind(Uuid::now_v7())
        .bind(principal.actor_id.value())
        .bind(Uuid::parse_str(resource["id"].as_str().unwrap()).unwrap())
        .bind(resource_type as i32)
        .execute(pool)
        .await
        .unwrap();
}

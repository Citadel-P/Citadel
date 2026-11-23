use super::*;

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    resource_actor: &ActorPrincipal,
    administrator: &ActorPrincipal,
    tag: &Value,
    unassigned_name: &str,
    resources: (&str, &str),
) {
    let (registry_id, repository_id) = resources;
    let tag_id = tag["id"].as_str().unwrap();
    for (collection, id) in [
        ("registries", registry_id),
        ("gitRepositories", repository_id),
    ] {
        tag_filters::verify(
            app,
            pool,
            if collection == "registries" {
                citadel_tags::TaggableResourceType::Registry
            } else {
                citadel_tags::TaggableResourceType::GitRepository
            },
            Uuid::parse_str(id).unwrap(),
        )
        .await;
        let name = tag["name"].as_str().unwrap();
        for (second_tag, expected) in [(name, true), (tag_id, false), (unassigned_name, false)] {
            let response = request(
                app,
                Method::GET,
                &format!("/api/v1/{collection}?tags={name}&tags={second_tag}"),
                Some(resource_actor.clone()),
                None,
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            let body = response_json(response).await;
            let rows = body[collection].as_array().unwrap();
            assert_eq!(rows.len(), usize::from(expected), "{collection} filter");
            if expected {
                assert_eq!(rows[0]["id"], id);
                assert_eq!(rows[0]["tags"][0]["id"], tag_id);
            }
        }
    }
    let tag_list = |actor| request(app, Method::GET, "/api/v1/tags", Some(actor), None);
    let visible_count = |body: Value| {
        body["tags"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == tag_id)
            .unwrap()["usageCount"]
            .as_i64()
            .unwrap()
    };
    let normalized = response_json(
        request(
            app,
            Method::GET,
            "/api/v1/tags/_filter?TaGs=%20Ops%20&tags=OPS&tags=unknown%2B%25%26",
            Some(resource_actor.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        normalized,
        json!({"Ops":"ops","OPS":"ops","unknown+%&":"unknown+%&"})
    );
    let original_count = visible_count(response_json(tag_list(resource_actor.clone()).await).await);
    assert!(original_count >= 2);
    let orphan = Uuid::now_v7();
    sqlx::query("INSERT INTO resourcetags(resourceid,resourcetype,tagid,createdbyactorid) VALUES ($1,'Registry',$2,$3)")
        .bind(orphan).bind(Uuid::parse_str(tag_id).unwrap()).bind(administrator.actor_id.value()).execute(pool).await.unwrap();
    assert_eq!(
        visible_count(response_json(tag_list(resource_actor.clone()).await).await),
        original_count
    );
    let admin_count = visible_count(response_json(tag_list(administrator.clone()).await).await);
    // The built-in registry exists but only the administrator can see it here.
    sqlx::query("INSERT INTO resourcetags(resourceid,resourcetype,tagid,createdbyactorid) VALUES ($1,'Registry',$2,$3)")
        .bind(Uuid::from_u128(0x100)).bind(Uuid::parse_str(tag_id).unwrap()).bind(administrator.actor_id.value()).execute(pool).await.unwrap();
    assert_eq!(
        visible_count(response_json(tag_list(resource_actor.clone()).await).await),
        original_count
    );
    assert_eq!(
        visible_count(response_json(tag_list(administrator.clone()).await).await),
        admin_count + 1
    );
    // Two team grants and a direct grant all reach the same registry.
    for _ in 0..2 {
        let team = Uuid::now_v7();
        sqlx::query("WITH actor AS (INSERT INTO actors(id,isenabled,type) VALUES ($1,true,'Team') RETURNING id), team AS (INSERT INTO teams(id,actorid,name) SELECT id,id,$2 FROM actor RETURNING id), member AS (INSERT INTO actorteammemberships(teamid,memberactorid) SELECT id,$3 FROM team) INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) SELECT $4,id,$5,$6,$7,0 FROM team")
            .bind(team).bind(format!("team-{team}")).bind(resource_actor.actor_id.value()).bind(Uuid::now_v7())
            .bind(PermissionLevel::Read as i32).bind(Uuid::parse_str(registry_id).unwrap())
            .bind(ResourceType::Registry as i32).execute(pool).await.unwrap();
    }
    let patched = request(
        app,
        Method::PATCH,
        &format!("/api/v1/tags/{tag_id}"),
        Some(resource_actor.clone()),
        Some(json!({"color":"#445566"})),
    )
    .await;
    assert_eq!(patched.status(), StatusCode::OK);
    assert_eq!(response_json(patched).await["usageCount"], original_count);
}

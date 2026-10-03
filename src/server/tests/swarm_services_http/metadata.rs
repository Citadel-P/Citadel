use super::*;

pub(super) async fn verify(app: &Router, pool: &sqlx::PgPool, admin: &ActorPrincipal, id: Uuid) {
    let uri = format!("/api/v1/swarmServices/{id}/_metadata");
    let mut denied = admin.clone();
    denied.roles.clear();
    denied.actor_id = ActorId::new(Uuid::now_v7());
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &uri,
            None,
            Some(json!({"description":"no"}))
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &uri,
            Some(denied),
            Some(json!({"description":"no"}))
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let before: (Value, String, String, i64) = sqlx::query_as(
        "SELECT spec,desiredspechash,controlstate,rowversion FROM swarmservices WHERE id=$1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    for body in [
        json!({}),
        json!({"tags":[]}),
        json!({"description":12}),
        json!({"description":"x".repeat(601)}),
    ] {
        assert_eq!(
            request(app, Method::PATCH, &uri, Some(admin.clone()), Some(body))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for description in [json!("cache description"), Value::Null] {
        let response = request(
            app,
            Method::PATCH,
            &uri,
            Some(admin.clone()),
            Some(json!({"description":description})),
        )
        .await;
        let status = response.status();
        let body = response_json(response).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["description"], description);
        let stored: Option<String> =
            sqlx::query_scalar("SELECT description FROM swarmservices WHERE id=$1")
                .bind(id)
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(serde_json::to_value(stored).unwrap(), description);
    }
    let after: (Value, String, String, i64) = sqlx::query_as(
        "SELECT spec,desiredspechash,controlstate,rowversion FROM swarmservices WHERE id=$1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(
        (&before.0, &before.1, &before.2),
        (&after.0, &after.1, &after.2)
    );
    assert_eq!(after.3, before.3 + 2);
    let activities: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='SwarmServiceUpdated'").bind(id).fetch_one(pool).await.unwrap();
    assert_eq!(activities, 2);
    sqlx::query("UPDATE swarmservices SET controlstate='Processing' WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &uri,
            Some(admin.clone()),
            Some(json!({"description":"must not overwrite"}))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE swarmservices SET controlstate='Idle' WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1 AND eventtype='SwarmServiceUpdated'").bind(id).fetch_one(pool).await.unwrap();
    assert_eq!(count, activities);
}

use super::*;

// Ports BackupEndpoints' rename/metadata persistence and authorization cases;
// additionally proves simultaneous header edits cannot overwrite each other.
pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    policy: &Value,
) {
    let id = policy["id"].as_str().unwrap();
    let uuid = Uuid::parse_str(id).unwrap();
    let path = format!("/api/v1/backupPolicies/{id}/_metadata");
    let rename = "/api/v1/backupPolicies/rename";
    let reader = seed_regular_user(pool).await;
    for (method, url, body) in [
        (Method::POST, rename, json!({"id":id,"name":"new name"})),
        (Method::PATCH, path.as_str(), json!({"description":"ops"})),
    ] {
        for (principal, expected) in [
            (None, StatusCode::UNAUTHORIZED),
            (Some(reader.clone()), StatusCode::FORBIDDEN),
        ] {
            assert_eq!(
                request(app, method.clone(), url, principal, Some(body.clone()))
                    .await
                    .status(),
                expected
            );
        }
    }
    for body in [
        json!({}),
        json!({"id":Uuid::nil(),"name":"name"}),
        json!({"id":id,"name":" "}),
        json!({"id":id,"name":"a".repeat(129)}),
    ] {
        assert_eq!(
            request(app, Method::POST, rename, Some(admin.clone()), Some(body))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for body in [
        json!([]),
        json!({"description":123}),
        json!({"description":"a".repeat(601)}),
    ] {
        assert_eq!(
            request(app, Method::PATCH, &path, Some(admin.clone()), Some(body))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let name = format!("renamed-{id}");
    let (renamed, updated) = tokio::join!(
        request(
            app,
            Method::POST,
            rename,
            Some(admin.clone()),
            Some(json!({"id":id,"name":format!(" {name} ")}))
        ),
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"description":" ops ","source":{"$type":"CitadelSystem"},"enabled":false}))
        ),
    );
    assert_eq!(renamed.status(), StatusCode::OK);
    assert_eq!(updated.status(), StatusCode::OK);
    let saved = response_json(
        request(
            app,
            Method::GET,
            &format!("/api/v1/backupPolicies/{id}"),
            Some(admin.clone()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(saved["name"], name);
    assert_eq!(saved["description"], "ops");
    assert_eq!(saved["source"], policy["source"]);
    assert_eq!(saved["enabled"], policy["enabled"]);
    assert_eq!(
        saved["rowVersion"].as_i64(),
        policy["rowVersion"].as_i64().map(|v| v + 2)
    );
    let activities: Vec<String> = sqlx::query_scalar(
        "SELECT info FROM activityevents WHERE resourceid=$1 ORDER BY createdat",
    )
    .bind(uuid)
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(activities.len(), 2);
    let details: Vec<Value> = activities
        .iter()
        .map(|v| serde_json::from_str(v).unwrap())
        .collect();
    let renamed = details
        .iter()
        .find(|v| v["$type"] == "BackupPolicyRenamed")
        .unwrap();
    assert_eq!(renamed["OldName"], policy["name"]);
    assert_eq!(renamed["NewName"], name);
    let updated = details
        .iter()
        .find(|v| v["$type"] == "BackupPolicyUpdated")
        .unwrap();
    assert_eq!(updated["NewPolicy"]["Description"], "ops");
    assert_eq!(
        updated["NewPolicy"]["SourceKey"],
        format!("{}:data", policy["source"]["platformId"].as_str().unwrap())
    );
    assert!(updated["NewPolicy"].get("Webhook").is_none());

    let mut other = policy.clone();
    other["name"] = json!(format!("other-{id}"));
    assert_eq!(
        request(
            app,
            Method::POST,
            "/api/v1/backupPolicies",
            Some(admin.clone()),
            Some(other.clone())
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            app,
            Method::POST,
            rename,
            Some(admin.clone()),
            Some(json!({"id":id,"name":other["name"]}))
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM activityevents WHERE resourceid=$1")
        .bind(uuid)
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
    for patch in [json!({"description":null}), json!({})] {
        let response = request(app, Method::PATCH, &path, Some(admin.clone()), Some(patch)).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response_json(response).await["description"].is_null());
    }
    assert_eq!(
        request(
            app,
            Method::POST,
            rename,
            Some(admin.clone()),
            Some(json!({"id":Uuid::now_v7(),"name":"missing"}))
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &format!("/api/v1/backupPolicies/{}/_metadata", Uuid::now_v7()),
            Some(admin.clone()),
            Some(json!({}))
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    // Leave the existing execution assertions' name snapshot unchanged.
    assert_eq!(
        request(
            app,
            Method::POST,
            rename,
            Some(admin.clone()),
            Some(json!({"id":id,"name":policy["name"]}))
        )
        .await
        .status(),
        StatusCode::OK
    );
}

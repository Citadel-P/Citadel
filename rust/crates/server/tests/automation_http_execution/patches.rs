use super::*;

pub(super) async fn verify(
    app: &Router,
    admin: &ActorPrincipal,
    denied: &ActorPrincipal,
    store: &PostgresAutomationRepository,
) {
    let action = create(app, admin, "console.log('original');", true, 10).await;
    let id = Uuid::parse_str(action["id"].as_str().unwrap()).unwrap();
    let path = format!("/api/v1/automation/actions/{id}");
    for (principal, status) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(denied.clone()), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            request(
                app,
                Method::PATCH,
                &path,
                principal,
                Some(json!({"timeoutSeconds":"bad"}))
            )
            .await
            .status(),
            status
        );
    }
    for (payload, field) in [
        (json!({"timeoutSeconds":"bad"}), "$.timeoutSeconds"),
        (json!({"code":null}), "$.code"),
        (
            json!({"webhook":{"provider":"Unknown"}}),
            "$.webhook.provider",
        ),
        (json!({"webhook":{"enabled":"yes"}}), "$.webhook.enabled"),
    ] {
        let response = request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(payload),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        let errors = body(response).await;
        assert!(errors["errors"][field][0].as_str().is_some(), "{errors}");
    }
    let response = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"code":"console.log('edited');","description":"updated"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated = body(response).await;
    assert_eq!(updated["code"], "console.log('edited');");
    assert_eq!(updated["name"], action["name"]);
    assert_eq!(updated["timeoutSeconds"], action["timeoutSeconds"]);
    assert_eq!(updated["runAsActorId"], action["runAsActorId"]);
    let metadata = format!("{path}/_metadata");
    assert_eq!(
        request(
            app,
            Method::PATCH,
            &metadata,
            Some(admin.clone()),
            Some(json!({"code":"forbidden"}))
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let response = request(
        app,
        Method::PATCH,
        &metadata,
        Some(admin.clone()),
        Some(json!({"description":null})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let saved = store.get(id).await.unwrap();
    assert!(saved.description.is_none());
    assert_eq!(saved.code, "console.log('edited');");

    let response = request(app, Method::PATCH, &path, Some(admin.clone()), Some(json!({
        "webhook":{"enabled":false,"provider":"GitLab","authScheme":"GitLabSignedToken","secret":"saved-secret"}
    }))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let saved_webhook = body(response).await["webhook"].clone();
    assert_eq!(saved_webhook["provider"], "GitLab");
    assert_eq!(saved_webhook["authScheme"], "GitLabSignedToken");
    let response = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"description":"preserve webhook"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await["webhook"], saved_webhook);
    let response = request(
        app,
        Method::PATCH,
        &path,
        Some(admin.clone()),
        Some(json!({"webhook":null})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body(response).await["webhook"].is_null());
    assert!(store.get(id).await.unwrap().webhook.is_none());
}

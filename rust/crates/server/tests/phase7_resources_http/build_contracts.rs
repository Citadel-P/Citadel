use super::*;

pub(super) async fn verify(app: &Router, admin: &ActorPrincipal, project: &str) {
    for path in ["/api/v1/buildProjects", "/api/v1/buildAgentPools"] {
        assert_eq!(
            request(
                app,
                Method::POST,
                path,
                None,
                Some(json!({"enabled":"bad"}))
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(
                app,
                Method::POST,
                path,
                Some(admin.clone()),
                Some(json!({"enabled":"bad"}))
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let path = format!("/api/v1/buildProjects/{project}");
    for input in [
        json!({"builderKind":"Invalid"}),
        json!({"webhook":{"provider":"Invalid"}}),
        json!([]),
    ] {
        assert_eq!(
            request(app, Method::PATCH, &path, None, Some(input.clone()))
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(app, Method::PATCH, &path, Some(admin.clone()), Some(input))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let hook = json!({"enabled":false,"provider":"Generic","authScheme":"BearerToken","secret":null,"branchFilter":"main"});
    let updated = response_json(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"webhook":hook})),
        )
        .await,
    )
    .await;
    assert_eq!(updated["webhook"], hook);
    let omitted = response_json(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"target":"release"})),
        )
        .await,
    )
    .await;
    assert_eq!(omitted["webhook"], hook);
    let rotated = response_json(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"webhook":{"enabled":true,"secret":"rotated-build-secret"}})),
        )
        .await,
    )
    .await;
    assert_eq!(rotated["webhook"]["enabled"], true);
    assert_eq!(rotated["webhook"]["provider"], "Generic");
    assert_eq!(rotated["webhook"]["authScheme"], "BearerToken");
    assert_eq!(rotated["webhook"]["branchFilter"], "main");
    let secret_only = response_json(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"webhook":{"secret":"rotated-again"}})),
        )
        .await,
    )
    .await;
    assert_eq!(secret_only["webhook"]["enabled"], true);
    assert_eq!(secret_only["webhook"]["provider"], "Generic");
    assert_eq!(secret_only["webhook"]["branchFilter"], "main");
    let cleared = response_json(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"webhook":null,"target":null})),
        )
        .await,
    )
    .await;
    assert!(cleared["webhook"].is_null());
    assert!(cleared["target"].is_null());
    let pool=response_json(request(app,Method::POST,"/api/v1/buildAgentPools",Some(admin.clone()),Some(json!({"name":"native-build-contract","enabled":true,"providerSpec":{"$type":"SelfManagedVm","connectionMode":"EdgeAgent"}}))).await).await;
    assert_eq!(pool["providerSpec"]["maxWorkers"], 1);
    assert_eq!(pool["providerSpec"]["architecture"], "Amd64");
    let path = format!("/api/v1/buildAgentPools/{}", pool["id"].as_str().unwrap());
    let preserved = response_json(
        request(
            app,
            Method::PATCH,
            &path,
            Some(admin.clone()),
            Some(json!({"providerSpec":null,"maxActiveBuilders":null,"enabled":null})),
        )
        .await,
    )
    .await;
    assert_eq!(preserved["providerSpec"], pool["providerSpec"]);
    assert_eq!(preserved["maxActiveBuilders"], pool["maxActiveBuilders"]);
    assert_eq!(preserved["enabled"], true);
    assert_eq!(
        request(app, Method::DELETE, &path, Some(admin.clone()), None)
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
}

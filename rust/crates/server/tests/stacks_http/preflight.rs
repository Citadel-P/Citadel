use super::*;

pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    platform: Uuid,
) {
    let path = "/api/v1/stacks/preflight/swarm";
    let input = json!({"name":"preflight","platformId":platform,"stackSource":"WebEditor",
        "spec":{"$type":"WebEditor","composeFile":"services:\n  web:\n    image: nginx:alpine\n", "destroyBeforeDeploy":false}});
    assert_eq!(
        request(app, Method::POST, path, None, Some(input.clone()))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("UPDATE platforms SET platformdescriptor=$2 WHERE id=$1")
        .bind(platform)
        .bind(json!({"$type":"DockerSwarm","controlAvailable":true,"localNodeState":"active"}))
        .execute(pool)
        .await
        .unwrap();
    let response = request(
        app,
        Method::POST,
        path,
        Some(admin.clone()),
        Some(input.clone()),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["isCompatible"], true);
    let mut invalid = input.clone();
    invalid["spec"]["destroyBeforeDeploy"] = json!(true);
    let response =
        response_json(request(app, Method::POST, path, Some(admin.clone()), Some(invalid)).await)
            .await;
    assert_eq!(response["isCompatible"], false);
    assert!(
        response["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["code"] == "stack.destroy_before_deploy")
    );
    sqlx::query("UPDATE platforms SET status='Offline' WHERE id=$1")
        .bind(platform)
        .execute(pool)
        .await
        .unwrap();
    let response =
        response_json(request(app, Method::POST, path, Some(admin.clone()), Some(input)).await)
            .await;
    assert_eq!(response["isCompatible"], false);
    assert!(
        response["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["code"] == "platform.offline")
    );
    sqlx::query("UPDATE platforms SET status='Online' WHERE id=$1")
        .bind(platform)
        .execute(pool)
        .await
        .unwrap();
}

use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn get_container_ports_the_complete_dotnet_summary_snapshot_without_deployment_secrets() {
    let f = fixture().await;
    let id = Uuid::now_v7();
    let image = Uuid::now_v7();
    let deployment = Uuid::now_v7();
    let docker_id = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    sqlx::query("INSERT INTO images(id,name,tags,dockerimageid,size,containers,platformid,createdat) VALUES($1,'image-01','[\"sha256:abcd1234\"]','image0123',1000,1,$2,'0001-01-01T00:00:00Z')")
        .bind(image).bind(f.platform_id).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,status,spec) VALUES($1,'deployment-01',$2,$3,'Created',$4)")
        .bind(deployment).bind(f.platform_id).bind(SYSTEM_ACTOR_ID)
        .bind(json!({"EnvironmentVariables":["TOKEN=never-expose-deployment-configuration"]}))
        .execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO containers(id,name,dockercontainerid,dockerimageid,platformid,imageid,deploymentid,state,created,updated,ports) VALUES($1,'deployment-01',$2,'container-01-id',$3,$4,$5,'Created',1768686293,0,'{}')")
        .bind(id).bind(&docker_id).bind(f.platform_id).bind(image).bind(deployment).execute(&f.pool).await.unwrap();
    let expected = json!({
        "id":id,"platformId":f.platform_id,"containerId":docker_id,"name":"deployment-01",
        "dockerImageId":"container-01-id","created":1768686293,"state":"Created","controlState":"Idle","updated":0,
        "isSystem":false,"hasCitadelOwnershipLabels":false,"isSwarmTask":false,"deploymentId":deployment,
        "imageView":{"id":image,"tags":["sha256:abcd1234"],"name":"image-01","dockerImageId":"image0123",
            "size":1000.0,"isInUse":true,"platformId":f.platform_id,"createdAt":"0001-01-01T00:00:00Z","controlState":"Idle","isStale":false},
        "deploymentView":{"id":deployment,"name":"deployment-01","platformId":f.platform_id,"createdAt":"0001-01-01T00:00:00Z",
            "createdByActorId":Uuid::nil(),"status":"Created","controlState":"Idle","platformStatus":"Offline",
            "autoUpdateState":{"lastCheckedAt":"0001-01-01T00:00:00Z","status":"Unknown"}},
        "capabilities":{"canViewLogs":true,"canInspect":true,"canOpenTerminal":true,"canPull":true,
            "canManageNodeAgents":true,"canRead":true,"canWrite":true,"canExecute":true}
    });
    for reference in [id.to_string(), docker_id.clone(), docker_id[..12].into()] {
        let response = send(
            &f,
            &format!("/api/v1/containers/{reference}"),
            Some(f.administrator.clone()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut body = json_body(response).await;
        // The .NET verified snapshot omits null/empty optional collections.
        // All nonempty fields (including unexpected additions) remain compared.
        omit_empty(&mut body);
        assert_eq!(body, expected);
    }
    let reader = super::lookup::subject(&f).await;
    super::lookup::grant(
        &f,
        reader.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let body = json_body(send(&f, &format!("/api/v1/containers/{id}"), Some(reader)).await).await;
    assert_eq!(body["deploymentView"]["name"], "deployment-01");
    assert!(body["deploymentView"].get("spec").is_none());
    assert!(
        !body
            .to_string()
            .contains("never-expose-deployment-configuration")
    );
    f.docker_server.abort();
}

fn omit_empty(value: &mut Value) {
    match value {
        Value::Object(properties) => {
            for child in properties.values_mut() {
                omit_empty(child);
            }
            properties.retain(|_, child| {
                !child.is_null()
                    && !child.as_array().is_some_and(Vec::is_empty)
                    && !child.as_object().is_some_and(serde_json::Map::is_empty)
            });
        }
        Value::Array(items) => {
            for child in items {
                omit_empty(child);
            }
        }
        _ => {}
    }
}

// Ports GetContainerTests.Get_Container_ReturnsSuccess and
// Get_Container_ByPersistedId_ReturnsSuccess using persisted PostgreSQL rows.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn get_container_by_docker_and_persisted_id_returns_the_same_resource() {
    let f = fixture().await;
    let id = Uuid::now_v7();
    let docker_id = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
    sqlx::query("INSERT INTO containers(id,dockercontainerid,dockerimageid,name,platformid,created,updated,state,ports) VALUES($1,$2,'sha256:fixture','get-container',$3,1,1,'Created','{}')")
        .bind(id).bind(&docker_id).bind(f.platform_id).execute(&f.pool).await.unwrap();
    let mut expected = None;
    for reference in [id.to_string(), docker_id.clone(), docker_id[..12].into()] {
        let response = send(
            &f,
            &format!("/api/v1/containers/{reference}"),
            Some(f.administrator.clone()),
        )
        .await;
        let status = response.status();
        let body = json_body(response).await;
        assert_eq!(status, StatusCode::OK, "{reference}: {body}");
        assert_eq!(body["id"], id.to_string());
        assert_eq!(body["containerId"], docker_id);
        assert_eq!(body["name"], "get-container");
        assert_eq!(body["capabilities"]["canRead"], true);
        if let Some(expected) = &expected {
            assert_eq!(&body, expected);
        } else {
            expected = Some(body);
        }
    }
    let reader = super::lookup::subject(&f).await;
    let url = format!("/api/v1/containers/{docker_id}");
    assert_eq!(
        send(&f, &url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&f, &url, Some(reader.clone())).await.status(),
        StatusCode::FORBIDDEN
    );
    super::lookup::grant(
        &f,
        reader.actor_id.value(),
        citadel_primitives::ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    assert_eq!(send(&f, &url, Some(reader)).await.status(), StatusCode::OK);
    for (reference, expected) in [
        ("short".into(), StatusCode::BAD_REQUEST),
        (Uuid::nil().to_string(), StatusCode::BAD_REQUEST),
        (Uuid::now_v7().to_string(), StatusCode::NOT_FOUND),
        ("f".repeat(64), StatusCode::NOT_FOUND),
    ] {
        assert_eq!(
            send(
                &f,
                &format!("/api/v1/containers/{reference}"),
                Some(f.administrator.clone())
            )
            .await
            .status(),
            expected
        );
    }

    // A short Docker ID cannot silently choose between two persisted rows.
    let duplicate = format!("{}{}", &docker_id[..12], "b".repeat(52));
    sqlx::query("INSERT INTO containers(id,dockercontainerid,dockerimageid,name,platformid,created,updated,state,ports) VALUES($1,$2,'sha256:fixture','ambiguous-container',$3,1,1,'Created','{}')")
        .bind(Uuid::now_v7()).bind(duplicate).bind(f.platform_id).execute(&f.pool).await.unwrap();
    for suffix in ["", "/inspect"] {
        assert_eq!(
            send(
                &f,
                &format!("/api/v1/containers/{}{suffix}", &docker_id[..12]),
                Some(f.administrator.clone())
            )
            .await
            .status(),
            StatusCode::CONFLICT
        );
    }
    f.docker_server.abort();
}

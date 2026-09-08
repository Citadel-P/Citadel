use super::*;
use citadel_adapters::{
    container_mutations::ContainerRuntimeRouter,
    crypto::AesGcmSecretProtector,
    deployment_runtime::DeploymentRuntimeRouter,
    deployment_store::{PostgresContainerAdoption, PostgresDeploymentStore},
};
use citadel_deployments::DeploymentService;
use citadel_server::deployments_http::{self, DeploymentsHttpState};
use tokio_util::sync::CancellationToken;

// Ports ContainerAdoptionDraftFactoryTests + adoption endpoint persistence,
// secret redaction, permission, stale-preview and concurrent ownership scenarios.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn adoption_is_authorized_non_mutating_and_atomically_links_secrets_and_activity() {
    exercise_adoption(false).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn external_image_adoption_preserves_the_existing_ui_contract() {
    exercise_adoption(true).await;
}

async fn exercise_adoption(external: bool) {
    let mut f = fixture().await;
    let image = format!("sha256:{}", "a".repeat(64));
    let container = "b".repeat(64);
    let id = Uuid::now_v7();
    let image_id = Uuid::now_v7();
    let socket = std::env::temp_dir().join(format!("adopt-{}.sock", Uuid::now_v7()));
    let listener = UnixListener::bind(&socket).unwrap();
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let recorded = requests.clone();
    let document = Arc::new(tokio::sync::RwLock::new(
        json!({"Id":container,"Config":{"Image":"nginx:latest","Env":["API_KEY=keep-me-private","LOG_LEVEL=info"],"Labels":{},"Entrypoint":["/entrypoint"]},"HostConfig":{"RestartPolicy":{"Name":"always"},"PortBindings":{"80/tcp":[{"HostPort":"8080"}]}},"NetworkSettings":{"Networks":{"bridge":{}}},"Mounts":[]}),
    ));
    let current = document.clone();
    let docker_image = image.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut buffer).await.unwrap();
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            let line = request.lines().next().unwrap().to_owned();
            recorded.lock().await.push(line.clone());
            assert!(
                line.starts_with("GET "),
                "Adoption must never mutate Docker: {line}"
            );
            let body = if line.contains("/version ") {
                json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"})
            } else if line.contains("/containers/") {
                current.read().await.clone()
            } else {
                json!({"Id":docker_image,"Config":{"Entrypoint":["/entrypoint"]}})
            };
            let body = body.to_string();
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
    });
    sqlx::query("UPDATE platforms SET platformdescriptor='{\"$type\":\"Docker\"}',connectortype='Local' WHERE id=$1").bind(f.platform_id).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO images(id,dockerimageid,name,platformid,createdat,tags) VALUES($1,$2,'nginx',$3,now(),'[\"nginx:latest\"]')").bind(image_id).bind(&image).bind(f.platform_id).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO containers(id,dockercontainerid,dockerimageid,name,platformid,created,updated,state,ports,imageid) VALUES($1,$2,$3,'/nginx',$4,1,1,'Running','{}',$5)").bind(id).bind(&container).bind(&image).bind(f.platform_id).bind(image_id).execute(&f.pool).await.unwrap();
    let docker = DockerClient::new(&socket, StdDuration::from_secs(2)).unwrap();
    let protector = Arc::new(AesGcmSecretProtector::new(&[11; 32]).unwrap());
    let service = DeploymentService::new(
        Arc::new(PostgresDeploymentStore::new(f.pool.clone())),
        Arc::new(DeploymentRuntimeRouter::new(
            f.pool.clone(),
            docker.clone(),
            None,
        )),
        Arc::new(StaticEntitlementService::new(true)),
        CancellationToken::new(),
    )
    .with_adoption(Arc::new(PostgresContainerAdoption::new(
        f.pool.clone(),
        ContainerRuntimeRouter::new(
            f.pool.clone(),
            docker,
            None,
            f.lookup_state.platforms.edge.clone(),
        ),
        protector.clone(),
        &[11; 32],
    )));
    f.app = f.app.merge(deployments_http::router(DeploymentsHttpState {
        identity: f.lookup_state.platforms.identity.clone(),
        deployments: Arc::new(service),
    }));
    let draft_url = format!("/api/v1/containers/{id}/adoption-draft");
    let adopt_url = format!("/api/v1/containers/{id}/adopt");
    assert_eq!(
        send(&f, &draft_url, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send(&f, &draft_url, Some(super::lookup::subject(&f).await))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    assert!(requests.lock().await.is_empty());
    for (system, swarm_task, ownership, control, stack) in [
        (true, false, false, "Idle", None),
        (false, true, false, "Idle", None),
        (false, false, true, "Idle", None),
        (false, false, false, "Processing", None),
        (false, false, false, "Idle", Some("compose-project")),
    ] {
        sqlx::query("UPDATE containers SET issystem=$2,isswarmtask=$3,hascitadelownershiplabels=$4,controlstate=$5,stack=$6 WHERE id=$1")
            .bind(id).bind(system).bind(swarm_task).bind(ownership).bind(control).bind(stack).execute(&f.pool).await.unwrap();
        let response = send(&f, &draft_url, Some(f.administrator.clone())).await;
        assert_eq!(
            response.status(),
            if stack.is_some() {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::CONFLICT
            }
        );
        assert!(requests.lock().await.is_empty());
    }
    sqlx::query("UPDATE containers SET issystem=false,isswarmtask=false,hascitadelownershiplabels=false,controlstate='Idle',stack=NULL WHERE id=$1")
        .bind(id).execute(&f.pool).await.unwrap();
    let draft = json_body(send(&f, &draft_url, Some(f.administrator.clone())).await).await;
    assert!(!draft.to_string().contains("keep-me-private"));
    assert_eq!(draft["canImportSensitiveEnvironmentValues"], true);
    assert_eq!(draft["draft"]["spec"]["ports"], json!(["8080:80/tcp"]));
    let mut input = draft["draft"].clone();
    if external {
        input["spec"]["image"] = json!({"$type":"External","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"nginx"});
    }
    input.as_object_mut().unwrap().remove("platformId");
    input["previewFingerprint"] = draft["previewFingerprint"].clone();
    input["importSensitiveEnvironmentAsSecrets"] = json!(true);
    let mut stale = input.clone();
    stale["previewFingerprint"] = json!("0".repeat(64));
    assert_eq!(
        send_json(&f, Method::POST, &adopt_url, f.administrator.clone(), stale)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let mut unrelated = input.clone();
    unrelated["spec"]["image"] = json!({"$type":"External","registryId":"00000000-0000-0000-0000-000000000100","imageTag":"redis"});
    assert_eq!(
        send_json(
            &f,
            Method::POST,
            &adopt_url,
            f.administrator.clone(),
            unrelated
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let mut invalid = input.clone();
    invalid["tagIds"] = json!([Uuid::now_v7()]);
    assert_eq!(
        send_json(
            &f,
            Method::POST,
            &adopt_url,
            f.administrator.clone(),
            invalid
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM deployments WHERE platformid=$1")
            .bind(f.platform_id)
            .fetch_one(&f.pool)
            .await
            .unwrap(),
        0
    );
    // A real runtime change invalidates the signed draft, not just a malformed signature.
    document.write().await["Config"]["Cmd"] = json!(["changed"]);
    assert_eq!(
        send_json(
            &f,
            Method::POST,
            &adopt_url,
            f.administrator.clone(),
            input.clone()
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    document.write().await["Config"]
        .as_object_mut()
        .unwrap()
        .remove("Cmd");
    let (first, second) = tokio::join!(
        send_json(
            &f,
            Method::POST,
            &adopt_url,
            f.administrator.clone(),
            input.clone()
        ),
        send_json(&f, Method::POST, &adopt_url, f.administrator.clone(), input)
    );
    let response = if first.status() == StatusCode::OK {
        assert_eq!(second.status(), StatusCode::CONFLICT);
        first
    } else {
        assert_eq!(first.status(), StatusCode::CONFLICT);
        assert_eq!(second.status(), StatusCode::OK);
        second
    };
    let adopted = json_body(response).await;
    assert_eq!(adopted["status"], "Healthy");
    assert!(!adopted.to_string().contains("keep-me-private"));
    let deployment_id = Uuid::parse_str(adopted["id"].as_str().unwrap()).unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT deploymentid FROM containers WHERE id=$1")
            .bind(id)
            .fetch_one(&f.pool)
            .await
            .unwrap(),
        Some(deployment_id)
    );
    let encrypted:String=sqlx::query_scalar("SELECT v.encryptedvalue FROM internalsecretvalues v JOIN resourcebindings b ON b.secretid=v.secretid WHERE b.resourceid=$1").bind(deployment_id).fetch_one(&f.pool).await.unwrap();
    assert!(!encrypted.contains("keep-me-private"));
    assert_eq!(
        protector.unprotect(&encrypted).unwrap().as_slice(),
        b"keep-me-private"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT eventtype FROM activityevents WHERE resourceid=$1")
            .bind(deployment_id)
            .fetch_one(&f.pool)
            .await
            .unwrap(),
        "DeploymentAdopted"
    );
    let activity: serde_json::Value =
        sqlx::query_scalar("SELECT info::jsonb FROM activityevents WHERE resourceid=$1")
            .bind(deployment_id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(activity["ContainerId"], container);
    assert_eq!(activity["ContainerName"], "/nginx");
    assert!(!activity.to_string().contains("keep-me-private"));
    assert_eq!(
        send(&f, &draft_url, Some(f.administrator.clone()))
            .await
            .status(),
        StatusCode::CONFLICT
    );
    assert!(requests.lock().await.iter().all(|r| r.starts_with("GET ")));
    server.abort();
    f.docker_server.abort();
    std::fs::remove_file(socket).unwrap();
}

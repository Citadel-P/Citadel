#![cfg(unix)]
//! Actual signed Agent Build + push, with a disposable Git checkout, Registry
//! and PostgreSQL. No published images or existing user workloads are touched.
use citadel_adapters::connectors::agent::client::AgentClient;
use citadel_adapters::connectors::agent::client::AgentRequestSigner;
use citadel_adapters::external::builds::executor::AgentDockerBuildExecutor;
use citadel_adapters::persistence::postgres::builds::PostgresBuildRepository;
use citadel_adapters::persistence::postgres::builds::credentials::PostgresBuildRegistryCredentialResolver;
use citadel_builds::*;
use citadel_execution::{ProcessLimits, ProcessRequest};
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_primitives::ActorId;
use citadel_processes::run;
use futures_util::{FutureExt, future::BoxFuture};
use serde_json::json;
use std::{panic::AssertUnwindSafe, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zeroize::Zeroizing;

const REGISTRY: &str =
    "registry@sha256:a3d8aaa63ed8681a604f1dea0aa03f100d5895b6a58ace528858a7b332415373";
struct NoSecrets;
impl BuildSecretResolver for NoSecrets {
    fn resolve(&self, _: Uuid) -> BoxFuture<'_, Result<Zeroizing<String>, BuildError>> {
        Box::pin(async { panic!("The fixture references no secrets") })
    }
}
struct Logs {
    store: Arc<PostgresBuildRepository>,
    id: Uuid,
}
impl BuildLogSink for Logs {
    fn append<'a>(
        &'a self,
        stream: &'a str,
        message: &'a str,
    ) -> BoxFuture<'a, Result<(), BuildError>> {
        Box::pin(async move {
            self.store
                .append_log(
                    self.id,
                    &BuildLog {
                        stream: stream.into(),
                        message: message.into(),
                    },
                )
                .await
                .map(|_| ())
        })
    }
}
async fn command(program: &str, args: &[&str]) -> Vec<u8> {
    let result = run(
        ProcessRequest::new(program)
            .args(args.iter().copied())
            .limits(ProcessLimits {
                timeout: Duration::from_secs(120),
                ..Default::default()
            }),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(
        result.succeeded(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    result.stdout
}

async fn nested(daemon: &str, args: &[&str]) -> Vec<u8> {
    let mut arguments = vec!["exec", daemon, "docker"];
    arguments.extend_from_slice(args);
    command("docker", &arguments).await
}

#[tokio::test]
#[ignore = "requires Test-Phase7AgentBuild.ps1 with a published Agent candidate"]
async fn signed_agent_builds_committed_source_pushes_digest_and_persists_progress() {
    let database = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&database)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&database)
        .await
        .unwrap();
    let network = std::env::var("CITADEL_PHASE7_AGENT_NETWORK").unwrap();
    let image = std::env::var("CITADEL_PHASE7_AGENT_IMAGE").unwrap();
    let use_edge = std::env::var("CITADEL_PHASE7_BUILD_EDGE").as_deref() == Ok("1");
    let edge_registry = citadel_adapters::connectors::edge::EdgeRegistry::default();
    let edge_cancel = CancellationToken::new();
    let mut edge_server = None;
    let suffix = Uuid::now_v7().simple().to_string();
    let agent_name = format!("citadel-build-agent-{suffix}");
    let registry_name = format!("citadel-build-registry-{suffix}");
    let daemon_name = format!("citadel-build-daemon-{suffix}");
    let socket_volume = format!("citadel-build-socket-{suffix}");
    let directory = std::env::temp_dir().join(format!("citadel-build-{suffix}"));
    tokio::fs::create_dir(&directory).await.unwrap();
    let result=AssertUnwindSafe(async {
        command("docker",&["run","--detach","--name",&registry_name,"--network",&network,REGISTRY]).await;
        let host=format!("{registry_name}:5000");
        let mount=format!("{socket_volume}:/var/run");
        command("docker",&["run","--detach","--privileged","--name",&daemon_name,"--network",&network,"--volume",&mount,"--env","DOCKER_TLS_CERTDIR=","docker:27.5.1-dind","--host=unix:///var/run/docker.sock","--bip=10.224.0.1/24","--insecure-registry",&host]).await;
        tokio::time::timeout(Duration::from_secs(45),async {
            loop {
                let result=run(ProcessRequest::new("docker").args(["exec",&daemon_name,"docker","info"]),&CancellationToken::new()).await.unwrap();
                if result.succeeded() { break; }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }).await.unwrap();
        let address=format!("http://{agent_name}:9000");
        let actor=ActorId::new(SYSTEM_ACTOR_ID);let platform=Uuid::now_v7();let registry=Uuid::now_v7();let repository=Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,$4,0,0,0,$3,0,'{\"$type\":\"DockerStandalone\"}','Online',0)").bind(platform).bind(&address).bind(format!("build-{suffix}")).bind(if use_edge { "EdgeAgent" } else { "Agent" }).execute(&pool).await.unwrap();
        let cancel=CancellationToken::new();
        let client = if use_edge {
            use citadel_adapters::connectors::edge::EdgeIntake;
use citadel_adapters::connectors::edge::EdgeTarget;
use citadel_adapters::persistence::postgres::platforms::edge::store::PostgresEdgeStore;
            use citadel_contracts::citadel::edge::v1::edge_agent_service_server::EdgeAgentServiceServer;
            let store = PostgresEdgeStore::new(pool.clone());
            let target = EdgeTarget::platform(platform);
            let (_, token, _) = store.create_enrollment(&target, SYSTEM_ACTOR_ID).await.unwrap();
            let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let incoming = futures_util::stream::unfold(listener, |listener| async move {
                Some((listener.accept().await.map(|(stream, _)| stream), listener))
            });
            let intake = EdgeIntake::new(store, edge_registry.clone());
            let shutdown = edge_cancel.clone();
            edge_server = Some(tokio::spawn(async move {
                tonic::transport::Server::builder().add_service(EdgeAgentServiceServer::new(intake))
                    .serve_with_incoming_shutdown(incoming, shutdown.cancelled_owned()).await.unwrap();
            }));
            let core = std::env::var("CITADEL_PHASE7_AGENT_HOST").unwrap();
            command("docker", &["run", "--detach", "--name", &agent_name, "--network", &network, "--volume", &mount,
                "--env", "CITADEL_AGENT_MODE=edge", "--env", &format!("CITADEL_CORE_URL=http://{core}:{port}"),
                "--env", &format!("CITADEL_EDGE_ENROLLMENT_TOKEN={token}"), &image]).await;
            tokio::time::timeout(Duration::from_secs(45), async {
                while edge_registry.get(&target).is_err() { tokio::time::sleep(Duration::from_millis(250)).await; }
            }).await.unwrap();
            None
        } else {
        let mut key=[0;32];getrandom::fill(&mut key).unwrap();let signer=AgentRequestSigner::from_bytes(&key);key.fill(0);
        command("docker",&["run","--detach","--name",&agent_name,"--network",&network,"--volume",&mount,"--env",&format!("HUB_PUBLIC_KEY={}",signer.public_key_base64()),"--env","CITADEL_AGENT_TLS_MODE=Disabled",&image]).await;
        Some(tokio::time::timeout(Duration::from_secs(45),async {
            loop {
                if let Ok(client)=AgentClient::connect(&address,signer.clone(),Duration::from_secs(10),true).await && client.handshake(&cancel).await.is_ok() { break client; }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }).await.unwrap())
        };
        sqlx::query("INSERT INTO registries(id,configuration,createdbyactorid,name,registryhost,status) VALUES($1,'{\"AuthEnabled\":false}'::json,$2,$3,$4,'Enabled')").bind(registry).bind(SYSTEM_ACTOR_ID).bind(format!("registry-{suffix}")).bind(&host).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/fixture.git','Idle')").bind(repository).bind(SYSTEM_ACTOR_ID).bind(format!("repo-{suffix}")).execute(&pool).await.unwrap();
        let checkout=directory.join(repository.to_string());tokio::fs::create_dir(&checkout).await.unwrap();
        let checkout=checkout.to_str().unwrap();
        command("git",&["init","--initial-branch=main",checkout]).await;
        tokio::fs::write(format!("{checkout}/Dockerfile"),"FROM scratch\nCOPY marker /marker\n").await.unwrap();
        tokio::fs::write(format!("{checkout}/marker"),format!("committed-{suffix}")).await.unwrap();
        command("git",&["-C",checkout,"add","Dockerfile","marker"]).await;
        command("git",&["-C",checkout,"-c","user.name=Citadel acceptance","-c","user.email=acceptance@example.test","commit","-m","Fixture"]).await;
        let commit=String::from_utf8(command("git",&["-C",checkout,"rev-parse","HEAD"]).await).unwrap().trim().to_owned();
        command("git",&["-C",checkout,"update-ref","refs/remotes/origin/main",&commit]).await;
        // Uncommitted data must never enter an Agent build archive.
        tokio::fs::write(format!("{checkout}/marker"),"uncommitted-must-not-be-built").await.unwrap();
        let mut input:BuildProjectConfiguration=serde_json::from_value(json!({"name":format!("project-{suffix}"),"enabled":true,"gitRepositoryId":repository,"branch":"main","platformId":platform,"registryId":registry,"imageRepository":format!("citadel-acceptance-{suffix}"),"tagTemplates":["{shortSha}"],"timeoutSeconds":120,"retentionRunCount":2})).unwrap();
        input.validate().unwrap();
        let store=Arc::new(PostgresBuildRepository::new(pool.clone()));let project=store.create(actor,&input).await.unwrap();
        let queued=store.enqueue(actor,project.id,"Manual").await.unwrap();let claim=store.claim_next(chrono::Utc::now()-chrono::Duration::minutes(5)).await.unwrap().unwrap();
        assert_eq!(claim.run.id,queued.id);
        let credentials=Arc::new(PostgresBuildRegistryCredentialResolver::new(pool.clone()));
        let executor=match client {
            Some(client) => AgentDockerBuildExecutor::new(directory.clone(),client,Arc::new(NoSecrets),credentials,1024*1024),
            None => AgentDockerBuildExecutor::new_edge(directory.clone(),edge_registry.clone(),Arc::new(NoSecrets),credentials,1024*1024),
        };
        let completed=executor.execute(&claim,&Logs{store:store.clone(),id:queued.id},&cancel).await;
        assert_eq!(completed.status,citadel_builds::BuildRunStatus::Succeeded,"{completed:?}");
        assert_eq!(completed.resolved_commit_sha.as_deref(),Some(commit.as_str()));
        let reference=completed.image_references.first().unwrap().clone();
        let digest=completed.image_digest.as_ref().expect("Push must report the Registry digest");
        let tag=reference.rsplit(':').next().unwrap();
        let manifest=reqwest::Client::builder().timeout(Duration::from_secs(10)).build().unwrap().get(format!("http://{registry_name}:5000/v2/citadel-acceptance-{suffix}/manifests/{tag}"))
            .header("Accept","application/vnd.docker.distribution.manifest.v2+json, application/vnd.oci.image.manifest.v1+json").send().await.unwrap();
        assert!(manifest.status().is_success());assert_eq!(manifest.headers()["docker-content-digest"],digest.as_str());
        assert!(store.finish(&claim,&completed).await.unwrap());
        let persisted=store.get_run(queued.id).await.unwrap();assert_eq!(persisted.status,citadel_builds::BuildRunStatus::Succeeded);assert_eq!(persisted.image_digest,Some(digest.clone()));
        assert!(!store.logs(queued.id).await.unwrap().is_empty(),"Streamed progress must be persisted");
        // Inspect the built filesystem without executing the scratch image.
        let container=String::from_utf8(nested(&daemon_name,&["create",&reference,"/unused"]).await).unwrap().trim().to_owned();
        nested(&daemon_name,&["cp",&format!("{container}:/marker"),"/tmp/exported-marker"]).await;
        nested(&daemon_name,&["rm",&container]).await;
        assert_eq!(command("docker",&["exec",&daemon_name,"cat","/tmp/exported-marker"]).await,format!("committed-{suffix}").as_bytes());
    }).catch_unwind().await;
    edge_cancel.cancel();
    for name in [&agent_name, &registry_name, &daemon_name] {
        let _ = run(
            ProcessRequest::new("docker").args(["rm", "--force", "--volumes", name]),
            &CancellationToken::new(),
        )
        .await;
    }
    let _ = run(
        ProcessRequest::new("docker").args(["volume", "rm", &socket_volume]),
        &CancellationToken::new(),
    )
    .await;
    let _ = tokio::fs::remove_dir_all(&directory).await;
    if let Some(server) = edge_server {
        let _ = tokio::time::timeout(Duration::from_secs(10), server).await;
    }
    pool.close().await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

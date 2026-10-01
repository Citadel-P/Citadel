//! Ports ForgejoPush_ShouldUpdateAndDeployGitStack and the Vault KV v2
//! resolve/inject/redact scenario against actual external services and Docker.
#![cfg(unix)]
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_adapters::{
    connectors::{docker::DockerClient, routing::stacks::StackRuntimeRouter},
    filesystem::stacks::materializer::GitStackSourceMaterializer,
    persistence::postgres::{
        activities::store::PostgresActivityStore,
        automation::PostgresAutomationRepository,
        git::{
            accounts::PostgresGitAccountRepository,
            repositories::PostgresGitRepositoryExecutionPersistence,
        },
        stacks::{PostgresStackRepository, bindings::PostgresStackBindingResolver},
    },
    security::identity::crypto::AesGcmSecretProtector,
};
use citadel_automation::{
    AutomationError, AutomationRunTokenIssuer, AutomationRuntimeConfig, AutomationService,
};
use citadel_git::{GitAccountService, GitCli, GitRepositoryExecutionService};
use citadel_identity::SYSTEM_ACTOR_ID;
use citadel_primitives::ActorId;
use citadel_server::api::routes::{webhooks as webhooks_http, webhooks::WebhooksHttpState};
use citadel_stacks::{
    NoopStackChangeNotifier, StackEntitlements, StackError, StackRepository, StackService,
};
use futures_util::{FutureExt, future::BoxFuture};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::{panic::AssertUnwindSafe, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const USER: &str = "citadel-acceptance";
const PASSWORD: &str = "CitadelAcceptance2026";
const TOKEN: &str = "citadel-vault-acceptance-root-token";
const VALUE: &str = "citadel-vault-acceptance-value-7b8d6e9c";
const HOOK: &str = "citadel-forgejo-acceptance-webhook-secret";
#[path = "external_integrations/inspection.rs"]
mod inspection_tests;
#[path = "external_integrations/vault.rs"]
mod vault_tests;
struct Entitlements;
impl StackEntitlements for Entitlements {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, StackError>> {
        Box::pin(async { Ok(true) })
    }
    fn operational_guardrails(&self) -> BoxFuture<'_, Result<bool, StackError>> {
        Box::pin(async { Ok(true) })
    }
}
struct NoAutomation;
impl AutomationRunTokenIssuer for NoAutomation {
    fn issue<'a>(
        &'a self,
        _: ActorId,
        _: Uuid,
        _: Duration,
    ) -> BoxFuture<'a, Result<String, AutomationError>> {
        Box::pin(async { panic!("This acceptance fixture does not run automations") })
    }
}

async fn api(client: &reqwest::Client, method: reqwest::Method, url: String, body: Value) -> Value {
    let response = client
        .request(method, url)
        .basic_auth(USER, Some(PASSWORD))
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(status.is_success(), "Forgejo returned {status}: {body}");
    serde_json::from_str(&body).unwrap()
}
fn compose(version: &str) -> String {
    format!(
        "services:\n  web:\n    image: alpine:3.21\n    command: [sleep, '300']\n    environment:\n      VERSION: '{version}'\n      TOKEN: ${{INJECTED}}\n"
    )
}
async fn commit(
    client: &reqwest::Client,
    repo_api: &str,
    file_sha: Option<&str>,
    version: &str,
) -> Value {
    api(client, if file_sha.is_some() { reqwest::Method::PUT } else { reqwest::Method::POST },
        format!("{repo_api}/contents/compose.yml"),
        json!({"branch":"main","content":STANDARD.encode(compose(version)),"message":format!("Version {version}"),"sha":file_sha})).await
}
async fn wait_commit(git: &GitRepositoryExecutionService, repository: Uuid, commit: &str) {
    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            if git
                .list_refs(repository)
                .await
                .unwrap()
                .iter()
                .any(|r| r.resolved_commit_sha.as_deref() == Some(commit))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("Repository did not synchronize the expected commit");
}
async fn assert_runtime(docker: &DockerClient, stack: Uuid, version: &str) {
    let containers = docker.list_containers(true).await.unwrap();
    let owned: Vec<_> = containers
        .iter()
        .filter(|c| c.labels.get("com.citadel.stack-id") == Some(&stack.to_string()))
        .collect();
    assert_eq!(owned.len(), 1);
    let inspected = docker.inspect_container(&owned[0].id).await.unwrap();
    assert!(inspected.state.running);
    assert!(
        inspected
            .config
            .environment
            .contains(&format!("VERSION={version}"))
    );
    assert!(
        inspected
            .config
            .environment
            .contains(&format!("TOKEN={VALUE}"))
    );
}

#[tokio::test]
#[ignore = "requires Test-Phase7Integrations.ps1: real Forgejo, Vault, PostgreSQL and Docker"]
async fn forgejo_push_applies_git_stack_with_vault_secret_and_redacted_audit() {
    let database = std::env::var("CITADEL_PHASE7_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&database)
        .await
        .unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&database)
        .await
        .unwrap();
    let forgejo = std::env::var("CITADEL_PHASE7_FORGEJO_URL").unwrap();
    let vault = std::env::var("CITADEL_PHASE7_VAULT_URL").unwrap();
    let callback = std::env::var("CITADEL_PHASE7_CALLBACK_HOST").unwrap();
    let docker = DockerClient::new("/var/run/docker-host.sock", Duration::from_secs(30)).unwrap();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let platform = Uuid::now_v7();
    let repository = Uuid::now_v7();
    let root = std::env::temp_dir().join(format!("citadel-provider-acceptance-{platform}"));
    tokio::fs::create_dir(&root).await.unwrap();
    let shutdown = CancellationToken::new();
    let mut tasks = tokio::task::JoinSet::new();
    let mut stack_id = None;
    let result = AssertUnwindSafe(async {
        let repo_name = format!("stack-{}", repository.simple());
        let repo_api = format!("{forgejo}/api/v1/repos/{USER}/{repo_name}");
        let remote = api(&client, reqwest::Method::POST, format!("{forgejo}/api/v1/user/repos"), json!({"name":repo_name,"private":true,"default_branch":"main"})).await;
        let clone_url = remote["clone_url"].as_str().unwrap();
        let initial = commit(&client, &repo_api, None, "one").await;
        let initial_commit = initial["commit"]["sha"].as_str().unwrap();
        let file_sha = initial["content"]["sha"].as_str().unwrap();
        let protector = Arc::new(AesGcmSecretProtector::new(&[61;32]).unwrap());
        let accounts = Arc::new(GitAccountService::new(Arc::new(PostgresGitAccountRepository::new(pool.clone())), protector.clone()));
        let account = accounts.create(ActorId::new(SYSTEM_ACTOR_ID), serde_json::from_value(json!({
            "name":format!("forgejo-{repository}"),"domain":reqwest::Url::parse(&forgejo).unwrap().host_str().unwrap(),
            "transport":"Http","authType":"Basic","configuration":{"$type":"Basic","username":USER,"password":PASSWORD}
        })).unwrap()).await.unwrap();
        sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
            .bind(platform).bind(platform.to_string()).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate,gitaccountid) VALUES($1,$2,'main',$3,'Unknown','Manual',$4,'Idle',$5)")
            .bind(repository).bind(SYSTEM_ACTOR_ID).bind(repo_name).bind(clone_url).bind(account.id).execute(&pool).await.unwrap();
        let git = Arc::new(GitRepositoryExecutionService::new(
            Arc::new(PostgresGitRepositoryExecutionPersistence::new(pool.clone())), accounts,
            Arc::new(GitCli::new(std::sync::Arc::new(citadel_processes::SystemProcess), std::sync::Arc::new(citadel_adapters::filesystem::git_workspace::LocalGitWorkspace), Duration::from_secs(30))), root.join("git"), Duration::from_secs(120)));
        let token = shutdown.child_token();
        let worker_git = git.clone();
        tasks.spawn(async move {
            while !token.is_cancelled() {
                worker_git.process_one(&token).await.unwrap();
                tokio::select! { ()=token.cancelled()=>break, ()=tokio::time::sleep(Duration::from_millis(100))=>{} }
            }
        });
        git.request_sync(ActorId::new(SYSTEM_ACTOR_ID), repository, Some("main")).await.unwrap();
        wait_commit(&git, repository, initial_commit).await;

        let response = client.post(format!("{vault}/v1/secret/data/citadel/acceptance")).header("x-vault-token",TOKEN)
            .json(&json!({"data":{"api_key":VALUE}})).send().await.unwrap();
        assert!(response.status().is_success());
        let provider = vault_tests::verify(&pool,protector.clone(),&vault).await;
        let secret = Uuid::now_v7();
        sqlx::query("INSERT INTO secretdefinitions(id,name,providertype,providerid,externalpath,externalkey,externalversion) VALUES($1,$2,'VaultCompatibleKvV2',$3,'citadel/acceptance','api_key',1)")
            .bind(secret).bind(secret.to_string()).bind(provider.id).execute(&pool).await.unwrap();
        let stacks = Arc::new(StackService::new(
Arc::new(citadel_server::tasks::stacks::TrackedStackTasks::new(citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new()))),Arc::new(PostgresStackRepository::new(pool.clone())),
            Arc::new(StackRuntimeRouter::new(pool.clone(),docker.clone(),None)),
            Arc::new(PostgresStackBindingResolver::new(pool.clone(),protector).unwrap()),
            Arc::new(NoopStackChangeNotifier),shutdown.child_token())
            .with_source_materializer(Arc::new(GitStackSourceMaterializer::new(git.clone())))
            .with_entitlements(Arc::new(Entitlements)));
        let stack = stacks.create(ActorId::new(SYSTEM_ACTOR_ID),true,serde_json::from_value::<citadel_server::api::resources::stacks::requests::CreateStackInput>(json!({
            "name":format!("provider-test-{}",platform.simple()),"platformId":platform,"stackSource":"Git","spec":{
                "$type":"Git","gitRepoId":repository,"branch":"main","composePaths":["compose.yml"],"updateBehavior":"StackAutoDeploy",
                "preDeploy":{"path":".","commands":["printf '%s' \"$INJECTED\""]},
                "webhook":{"enabled":true,"provider":"GitHub","secret":HOOK}
            }
        })).unwrap().try_into().unwrap()).await.unwrap();
        stack_id=Some(stack.id);
        sqlx::query("INSERT INTO resourcebindings(id,kind,name,resourceid,scope,secretdeliverymode,secretid) VALUES($1,'Secret','INJECTED',$2,'Stack','EnvironmentVariable',$3)")
            .bind(Uuid::now_v7()).bind(stack.id).bind(secret).execute(&pool).await.unwrap();
        let mut progress = stacks.apply(ActorId::new(SYSTEM_ACTOR_ID),true,citadel_stacks::ApplyStack{id:stack.id,recreate:None}).await.unwrap();
        let mut output = Vec::new();
        while let Some(item) = progress.recv().await {
            assert!(!serde_json::to_string(&citadel_server::api::resources::stacks::views::StackStreamItem::from(item.clone())).unwrap().contains(VALUE));
            output.push(item);
        }
        let initial_view=PostgresStackRepository::new(pool.clone()).get_authorized(ActorId::new(SYSTEM_ACTOR_ID),true,stack.id).await.unwrap();
        assert_eq!(initial_view.status,citadel_stacks::StackReleaseStatus::Healthy,"Initial Stack Apply failed: {output:?}");
        assert_runtime(&docker,stack.id,"one").await;
        inspection_tests::verify(&pool, &docker, &provider, platform, stack.id, "one").await;

    let automation_shutdown = tokio_util::sync::CancellationToken::new();
    let automation_tasks = citadel_runtime::DynamicTasks::new(automation_shutdown.clone());

        let automation=Arc::new(AutomationService::new(std::sync::Arc::new(citadel_processes::SystemProcess), std::sync::Arc::new(citadel_adapters::filesystem::automation_workspace::LocalAutomationWorkspace),
        Arc::new(citadel_server::tasks::automation::TrackedAutomationTasks::new(automation_tasks.clone())),
        automation_shutdown.clone(),Arc::new(PostgresAutomationRepository::new(pool.clone())),Arc::new(NoAutomation),AutomationRuntimeConfig{
            deno_path:"deno".into(),work_root:root.join("automation"),internal_base_url:"http://unused".into(),endpoint_catalog_json:"[]".into(),maximum_log_bytes:1024,stale_after:Duration::from_secs(60)}));
        let app=webhooks_http::router(WebhooksHttpState{git:git.clone(),automation,backups:None,builds:None,stacks:Some(stacks.clone()),services:None,alerts:None,audit:Some(Arc::new(PostgresActivityStore::new(pool.clone())))});
        let listener=tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
        let port=listener.local_addr().unwrap().port();
        let token=shutdown.child_token();
        tasks.spawn(async move { axum::serve(listener,app).with_graceful_shutdown(token.cancelled_owned()).await.unwrap(); });
        let hook_url=format!("http://{callback}:{port}/listener/github/stack/{}/deploy",stack.id);
        api(&client,reqwest::Method::POST,format!("{repo_api}/hooks"),json!({"type":"gitea","active":true,"events":["push"],"config":{"url":hook_url,"content_type":"json","secret":HOOK}})).await;
        let denied=client.post(&hook_url).header("x-forgejo-signature","invalid").json(&json!({})).send().await.unwrap();
        assert_eq!(denied.status(),reqwest::StatusCode::UNAUTHORIZED);
        let updated=commit(&client,&repo_api,Some(file_sha),"two").await;
        let updated_commit=updated["commit"]["sha"].as_str().unwrap();
        tokio::time::timeout(Duration::from_secs(45),async {
            loop {
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM stackwebhookdeployqueue WHERE stackid=$1").bind(stack.id).fetch_one(&pool).await.unwrap();
                if count==1 { break; }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }).await.expect("The real Forgejo delivery did not queue Stack Apply");
        assert_eq!(stacks.process_webhooks().await.unwrap(),1);
        let applied=PostgresStackRepository::new(pool.clone()).get_authorized(ActorId::new(SYSTEM_ACTOR_ID),true,stack.id).await.unwrap();
        assert_eq!(applied.status,citadel_stacks::StackReleaseStatus::Healthy);
        assert_eq!(applied.source.as_ref().unwrap().resolved_commit_sha,updated_commit);
        assert_runtime(&docker,stack.id,"two").await;
        inspection_tests::verify(&pool, &docker, &provider, platform, stack.id, "two").await;
        let audits:Vec<String>=sqlx::query_scalar("SELECT row_to_json(a)::text FROM activityevents a WHERE resourceid=ANY($1)")
            .bind(vec![stack.id,repository]).fetch_all(&pool).await.unwrap();
        assert!(audits.iter().any(|a|a.contains("StackWebhookReceived")));
        for audit in audits { for secret in [VALUE,TOKEN,HOOK,PASSWORD] { assert!(!audit.contains(secret),"Audit leaked a fixture credential"); } }
    }).catch_unwind().await;
    shutdown.cancel();
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    // Exact ownership filters: never remove another test's or user's resources.
    if let Some(id) = stack_id {
        cleanup(&docker, id).await;
    }
    tokio::fs::remove_dir_all(&root).await.unwrap();
    pool.close().await;
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

async fn cleanup(docker: &DockerClient, id: Uuid) {
    let mut projects = std::collections::BTreeSet::new();
    for container in docker
        .list_containers(true)
        .await
        .unwrap()
        .into_iter()
        .filter(|c| c.labels.get("com.citadel.stack-id") == Some(&id.to_string()))
    {
        if let Some(project) = container.labels.get("com.docker.compose.project") {
            projects.insert(project.clone());
        }
        docker
            .delete_container(&container.id, false, true)
            .await
            .unwrap();
    }
    for network in docker
        .list_networks()
        .await
        .unwrap()
        .into_iter()
        .filter(|n| {
            n.labels.get("com.citadel.stack-id") == Some(&id.to_string())
                || n.labels
                    .get("com.docker.compose.project")
                    .is_some_and(|p| projects.contains(p))
        })
    {
        docker.delete_network(&network.id).await.unwrap();
    }
}

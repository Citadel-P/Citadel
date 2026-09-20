use super::*;
use citadel_stacks::{StackBindingResolverPort, StackEntitlements, StackError, StackService};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct Entitlement(AtomicBool);
impl StackEntitlements for Entitlement {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, StackError>> {
        Box::pin(async { Ok(self.0.load(Ordering::Relaxed)) })
    }
}
struct Bindings;
impl StackBindingResolverPort for Bindings {
    fn resolve<'a>(
        &'a self,
        _id: Uuid,
        _names: &'a [String],
    ) -> BoxFuture<'a, Result<citadel_stacks::ResolvedStackBindings, StackError>> {
        Box::pin(async { Ok(Default::default()) })
    }
}
pub fn service(pool: sqlx::PgPool) -> (Arc<StackService>, Arc<Entitlement>) {
    let entitlement = Arc::new(Entitlement::default());
    let docker = citadel_adapters::docker::DockerClient::new(
        "/no-webhook-http-docker.sock",
        std::time::Duration::from_secs(1),
    )
    .unwrap();
    let service = StackService::new(
        Arc::new(citadel_server::api::stacks::TrackedStackTasks::new(
            citadel_application::DynamicTasks::new(tokio_util::sync::CancellationToken::new()),
        )),
        Arc::new(citadel_adapters::postgres::stacks::PostgresStackRepository::new(pool.clone())),
        Arc::new(citadel_adapters::stack_runtime::StackRuntimeRouter::new(
            pool, docker, None,
        )),
        Arc::new(Bindings),
        Arc::new(citadel_stacks::NoopStackChangeNotifier),
        CancellationToken::new(),
    )
    .with_entitlements(entitlement.clone());
    (Arc::new(service), entitlement)
}
async fn send(app: &Router, id: Uuid, secret: &str, payload: Value) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/listener/generic/stack/{id}/deploy"))
                .header("authorization", format!("Bearer {secret}"))
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}
pub async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    stacks: &StackService,
    entitlement: &Entitlement,
) {
    use citadel_identity::SYSTEM_ACTOR_ID;
    use citadel_primitives::ActorId;
    let platform = Uuid::now_v7();
    let repository = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$2,'Local',0,0,0,$2,0,'{\"$type\":\"Docker\"}','Online',0)")
        .bind(platform).bind(platform.to_string()).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url,controlstate) VALUES($1,$2,'main',$3,'Healthy','Manual','https://example.test/team/repository.git','Idle')")
        .bind(repository).bind(SYSTEM_ACTOR_ID).bind(repository.to_string()).execute(pool).await.unwrap();
    let input=serde_json::from_value::<citadel_server::api::stacks::requests::CreateStackInput>(json!({"name":format!("webhook-http-{platform}"),"platformId":platform,"stackSource":"Git","spec":{
        "$type":"Git","gitRepoId":repository,"branch":"main","composePaths":["compose.yml"],"updateBehavior":"StackAutoDeploy",
        "webhook":{"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"disposable-stack-hook"}
    }})).unwrap().try_into().unwrap();
    let stack = stacks
        .create(ActorId::new(SYSTEM_ACTOR_ID), true, input)
        .await
        .unwrap();
    let payload = json!({"branch":"main","commitSha":"a".repeat(40),"repository":"https://example.test/team/repository.git"});
    let count = async || {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM stackwebhookdeployqueue WHERE stackid=$1",
        )
        .bind(stack.id)
        .fetch_one(pool)
        .await
        .unwrap()
    };
    assert_eq!(
        send(app, stack.id, "wrong", payload.clone()).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(count().await, 0);
    assert_eq!(
        send(app, stack.id, "disposable-stack-hook", payload.clone()).await,
        StatusCode::ACCEPTED
    );
    assert_eq!(
        count().await,
        0,
        "unlicensed webhook must not queue execution"
    );
    entitlement.0.store(true, Ordering::Relaxed);
    for field in ["repository", "branch"] {
        let mut unrelated = payload.clone();
        unrelated[field] = json!("unrelated");
        assert_eq!(
            send(app, stack.id, "disposable-stack-hook", unrelated).await,
            StatusCode::ACCEPTED
        );
        assert_eq!(count().await, 0);
    }
    let mut malformed = payload.clone();
    malformed["commitSha"] = json!("--upload-pack=bad");
    assert_eq!(
        send(app, stack.id, "disposable-stack-hook", malformed).await,
        StatusCode::BAD_REQUEST
    );
    for _ in 0..2 {
        assert_eq!(
            send(app, stack.id, "disposable-stack-hook", payload.clone()).await,
            StatusCode::ACCEPTED
        );
    }
    assert_eq!(
        count().await,
        1,
        "accepted deliveries are durably coalesced"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT controlstate FROM stacks WHERE id=$1")
            .bind(stack.id)
            .fetch_one(pool)
            .await
            .unwrap(),
        "Idle",
        "HTTP does not own Docker execution"
    );
    // Revoking the entitlement between reception and processing discards the job.
    entitlement.0.store(false, Ordering::Relaxed);
    assert_eq!(stacks.process_webhooks().await.unwrap(), 0);
    assert_eq!(count().await, 0);
    let duplicate = stacks
        .duplicate_draft(ActorId::new(SYSTEM_ACTOR_ID), true, stack.id)
        .await
        .unwrap();
    assert!(
        matches!(&duplicate.draft.spec, citadel_stacks::StackSpec::Git { webhook: Some(webhook), .. } if webhook.secret.is_none()),
        "duplicate config must not copy authentication credentials"
    );
}

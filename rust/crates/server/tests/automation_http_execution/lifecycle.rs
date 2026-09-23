use super::*;

pub(super) async fn verify_request_drop_during_claim(
    service: &Arc<AutomationService>,
    store: &PostgresAutomationRepository,
    actor: ActorId,
    id: Uuid,
    db: &PgPool,
    tasks: &citadel_runtime::DynamicTasks,
) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while tasks.active() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let before = store.list_runs(id, 100).await.unwrap().len();
    let mut lock = db.begin().await.unwrap();
    sqlx::query("SELECT id FROM actions WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    let service = service.clone();
    let request = tokio::spawn(async move {
        service
            .run(
                actor,
                id,
                "Test",
                &json!({}),
                Some(30),
                Some("await new Promise(()=>setInterval(()=>{},1000));"),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while tasks.active() == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    lock.commit().await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let runs = store.list_runs(id, 100).await.unwrap();
            if runs.len() == before + 1 && tasks.active() == 0 {
                assert_eq!(runs[0].status, "Cancelled");
                assert_eq!(store.get(id).await.unwrap().control_state, "Idle");
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

pub(super) async fn verify(
    service: &Arc<AutomationService>,
    store: &PostgresAutomationRepository,
    actor: ActorId,
    id: Uuid,
    shutdown: &tokio_util::sync::CancellationToken,
    tasks: &citadel_runtime::DynamicTasks,
) {
    let mut progress = service
        .run(
            actor,
            id,
            "Test",
            &json!({}),
            None,
            Some("console.log('shutdown-ready'); await new Promise(()=>setInterval(()=>{},1000));"),
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let item = progress.recv().await.expect("run ended before shutdown");
            if item
                .stream
                .as_deref()
                .is_some_and(|line| line.contains("shutdown-ready"))
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    let run = store.list_runs(id, 1).await.unwrap().remove(0);
    assert_eq!(run.status, "Running");
    assert!(tasks.active() > 0);

    // Keep the viewer connected: shutdown itself must cancel and persist cleanup.
    shutdown.cancel();
    tasks.drain(Duration::from_secs(10)).await.unwrap();
    assert_eq!(tasks.active(), 0);
    assert_eq!(store.get_run(id, run.id).await.unwrap().status, "Cancelled");
    assert_eq!(store.get(id).await.unwrap().control_state, "Idle");

    let before = store.list_runs(id, 100).await.unwrap().len();
    assert!(matches!(
        service
            .run(actor, id, "Manual", &json!({}), None, None)
            .await,
        Err(citadel_automation::AutomationError::Conflict(_))
    ));
    assert_eq!(
        store.list_runs(id, 100).await.unwrap().len(),
        before,
        "closed admission must not create a durable claim"
    );
    while progress.recv().await.is_some() {}
}

use super::*;

pub(super) async fn verify(
    app: &Router,
    admin: &ActorPrincipal,
    service: &Arc<ScriptService>,
    store: &PostgresScriptRepository,
) {
    let mut runs = Vec::new();
    for _ in 0..6 {
        let action = create(
            app,
            admin,
            "await new Promise(r=>setTimeout(r,4000)); console.log('queued-work-complete');",
            true,
            10,
        )
        .await;
        let id = Uuid::parse_str(action["id"].as_str().unwrap()).unwrap();
        let mut progress = service
            .run(admin.actor_id, id, "Manual", &json!({}), None, None)
            .await
            .expect("busy execution slots must still accept a durable run");
        let queued = progress.recv().await.unwrap();
        assert_eq!(
            queued.status,
            Some(citadel_scripts::ScriptRunStatus::Queued)
        );
        runs.push((id, queued.run_id.unwrap()));
        drop(progress);
    }
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let mut running = 0;
            let mut queued = 0;
            for &(action, run) in &runs {
                match store.get_run(action, run).await.unwrap().status {
                    citadel_scripts::ScriptRunStatus::Running => running += 1,
                    citadel_scripts::ScriptRunStatus::Queued => queued += 1,
                    other => panic!("unexpected early status: {other:?}"),
                }
            }
            assert!(running <= service.options().max_parallel_runs);
            if running == 4 && queued == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let (cancel_action, cancel_run) = runs[5];
    service.cancel(cancel_action, cancel_run).await.unwrap();
    assert_eq!(
        store
            .get_run(cancel_action, cancel_run)
            .await
            .unwrap()
            .status,
        citadel_scripts::ScriptRunStatus::Cancelled
    );
    tokio::time::timeout(Duration::from_secs(12), async {
        loop {
            let mut finished = 0;
            for &(action, run) in &runs[..5] {
                if store.get_run(action, run).await.unwrap().status
                    == citadel_scripts::ScriptRunStatus::Succeeded
                {
                    finished += 1;
                }
            }
            if finished == 5 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let cancelled = store.get_run(cancel_action, cancel_run).await.unwrap();
    assert!(
        cancelled.started_at.is_none(),
        "queued cancellation must prevent execution"
    );
}

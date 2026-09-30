use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

fn target() -> ReconciliationTarget {
    ReconciliationTarget {
        id: uuid::Uuid::now_v7(),
        name: "fixture".into(),
        address: "local".into(),
        connector_type: citadel_platforms::ConnectorKind::Local,
        platform_type: citadel_platforms::PlatformKind::Docker,
        agent: None,
    }
}
fn transition(target: &ReconciliationTarget, online: bool) -> PlatformHealthTransition {
    PlatformHealthTransition {
        target: target.clone(),
        online,
    }
}
async fn settle() {
    for _ in 0..30 {
        tokio::task::yield_now().await;
    }
}

#[tokio::test(start_paused = true)]
async fn stable_probes_do_no_lifecycle_work_and_one_recovery_transition_runs_once() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = bounded_channel(256, QueueOverflowPolicy::Wait);
    let target = target();
    let plans = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let (recoveries, effects, token) = (plans.clone(), calls.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run_transitions(&token, receiver, |progress| {
            effects.fetch_add(1, Ordering::SeqCst);
            if progress.transition.online {
                recoveries.fetch_add(1, Ordering::SeqCst);
            }
            async { (progress, Ok::<_, &str>(())) }
        })
        .await;
    });
    let mut state = HealthState {
        online: Some(true),
        ..Default::default()
    };
    for _ in 0..100 {
        assert_eq!(state.observe(true), None);
    }
    settle().await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for _ in 0..3 {
        if let Some(online) = state.observe(false) {
            sender
                .send(transition(&target, online), &cancel)
                .await
                .ok()
                .unwrap();
        }
    }
    settle().await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for _ in 0..100 {
        if let Some(online) = state.observe(true) {
            sender
                .send(transition(&target, online), &cancel)
                .await
                .ok()
                .unwrap();
        }
    }
    settle().await;
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(plans.load(Ordering::SeqCst), 1);
    cancel.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn failed_effect_retains_progress_without_repeating_recovery() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = bounded_channel(4, QueueOverflowPolicy::Wait);
    let plans = Arc::new(AtomicUsize::new(0));
    let attempts = Arc::new(AtomicUsize::new(0));
    let (recoveries, effects, token) = (plans.clone(), attempts.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run_transitions(&token, receiver, |mut progress| {
            if !progress.recovery_requested {
                recoveries.fetch_add(1, Ordering::SeqCst);
                progress.recovery_requested = true;
            }
            let result = if effects.fetch_add(1, Ordering::SeqCst) == 0 {
                Err("alert failed")
            } else {
                Ok(())
            };
            async move { (progress, result) }
        })
        .await;
    });
    sender
        .send(transition(&target(), true), &cancel)
        .await
        .ok()
        .unwrap();
    settle().await;
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    tokio::time::advance(RETRY).await;
    settle().await;
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(plans.load(Ordering::SeqCst), 1);
    cancel.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn latest_transition_supersedes_retry_and_other_platforms_progress() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = bounded_channel(4, QueueOverflowPolicy::Wait);
    let failing = target();
    let healthy = target();
    let failing_id = failing.id;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (effects, token) = (calls.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run_transitions(&token, receiver, |progress| {
            let key = (progress.transition.target.id, progress.transition.online);
            effects.lock().unwrap().push(key);
            let result = if key == (failing_id, false) {
                Err("offline failed")
            } else {
                Ok(())
            };
            async move { (progress, result) }
        })
        .await;
    });
    sender
        .send(transition(&failing, false), &cancel)
        .await
        .ok()
        .unwrap();
    settle().await;
    sender
        .send(transition(&healthy, true), &cancel)
        .await
        .ok()
        .unwrap();
    sender
        .send(transition(&failing, true), &cancel)
        .await
        .ok()
        .unwrap();
    settle().await;
    tokio::time::advance(RETRY * 2).await;
    settle().await;
    assert_eq!(calls.lock().unwrap().len(), 3);
    assert!(calls.lock().unwrap().contains(&(healthy.id, true)));
    assert!(calls.lock().unwrap().contains(&(failing.id, true)));
    cancel.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn transitions_received_during_effect_are_retained_and_shutdown_drops_effect() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = bounded_channel(4, QueueOverflowPolicy::Wait);
    let target = target();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let dropped = Arc::new(AtomicUsize::new(0));
    struct Guard(Arc<AtomicUsize>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let release = Arc::new(tokio::sync::Notify::new());
    let (effects, drops, wake, token) = (
        calls.clone(),
        dropped.clone(),
        release.clone(),
        cancel.clone(),
    );
    let worker = tokio::spawn(async move {
        run_transitions(&token, receiver, |progress| {
            effects.lock().unwrap().push(progress.transition.online);
            let wake = wake.clone();
            let drops = drops.clone();
            async move {
                let _guard = Guard(drops);
                if !progress.transition.online {
                    wake.notified().await;
                } else {
                    std::future::pending::<()>().await;
                }
                (progress, Ok::<_, &str>(()))
            }
        })
        .await;
    });
    sender
        .send(transition(&target, false), &cancel)
        .await
        .ok()
        .unwrap();
    settle().await;
    sender
        .send(transition(&target, true), &cancel)
        .await
        .ok()
        .unwrap();
    settle().await;
    release.notify_one();
    settle().await;
    assert_eq!(*calls.lock().unwrap(), vec![false, true]);
    cancel.cancel();
    worker.await.unwrap();
    assert_eq!(dropped.load(Ordering::SeqCst), 2);
}

#[derive(Default)]
struct Alerts {
    fail_next: std::sync::atomic::AtomicBool,
    observations: Mutex<Vec<bool>>,
}
impl AlertEventSink for Alerts {
    fn observe<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> futures_util::future::BoxFuture<
        'a,
        Result<Option<citadel_alerts::AlertEvent>, citadel_alerts::AlertError>,
    > {
        self.observations.lock().unwrap().push(observation.matched);
        let fail = self.fail_next.swap(false, Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                Err(citadel_alerts::AlertError::Storage("fixture".into()))
            } else {
                Ok(None)
            }
        })
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn committed_lifecycle_retries_alert_without_repeating_online_recovery() {
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let platform = uuid::Uuid::now_v7();
    let deployment = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',0,0,0,0,0,'{\"$type\":\"Docker\"}','Online')").bind(platform).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$1::text,$2,'{}','Healthy',$3)").bind(deployment).bind(platform).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,deploymentid,dockercontainerid,dockerimageid,name,state,created,updated,ports) VALUES($1,$2,$3,'fixture','image','web','Running',1,1,'[]')").bind(uuid::Uuid::now_v7()).bind(platform).bind(deployment).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockernodeid,dockercontainerid,dockerimageid,name,state,created,updated,ports) VALUES($1,$2,'worker','node-fixture','image','node-web','Running',1,1,'[]')").bind(uuid::Uuid::now_v7()).bind(platform).execute(&pool).await.unwrap();
    let targets = PlatformRuntimeRegistry::new(pool.clone(), None);
    targets.refresh().await.unwrap();
    let target = targets
        .snapshot()
        .await
        .iter()
        .find(|t| t.id == platform)
        .unwrap()
        .clone();
    let cancel = CancellationToken::new();
    let (refreshes, receivers) = event_refresh_channels(4);
    let plans = Arc::new(Mutex::new(Vec::new()));
    let (requests, token) = (plans.clone(), cancel.clone());
    let scopes = tokio::spawn(async move {
        run_refresh_workers(
            &token,
            receivers,
            RETRY,
            |key| {
                requests.lock().unwrap().push(key);
                async { Ok::<_, &str>(None::<()>) }
            },
            |_, ()| async { Ok::<_, &str>(true) },
        )
        .await;
    });
    let alerts = Arc::new(Alerts::default());
    let hub = RealtimeHub::new(16, Arc::new(Metrics::default()));
    let mut updates = hub.subscribe();
    let worker = LifecycleWorker {
        targets: targets.clone(),
        pool: pool.clone(),
        refreshes,
        realtime: Some(hub),
        alerts: alerts.clone(),
    };
    worker
        .apply(&mut Progress::new(transition(&target, false)), &cancel)
        .await
        .unwrap();
    let status: (String,String,String) = sqlx::query_as("SELECT p.status,d.status,c.state FROM platforms p JOIN deployments d ON d.platformid=p.id JOIN containers c ON c.deploymentid=d.id WHERE p.id=$1").bind(platform).fetch_one(&pool).await.unwrap();
    assert_eq!(
        status,
        ("Offline".into(), "Degraded".into(), "Offline".into())
    );
    let node: String = sqlx::query_scalar(
        "SELECT state FROM containers WHERE platformid=$1 AND dockernodeid='worker'",
    )
    .bind(platform)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(node, "Running");
    tokio::time::timeout(Duration::from_secs(2), updates.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(plans.lock().unwrap().is_empty());
    alerts.fail_next.store(true, Ordering::SeqCst);
    let mut online = Progress::new(transition(&target, true));
    assert!(worker.apply(&mut online, &cancel).await.is_err());
    assert!(online.persisted && online.recovery_requested);
    let status: String = sqlx::query_scalar("SELECT status FROM platforms WHERE id=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Online");
    // A repeated status write would wait on this lock. Alert-only retry must not.
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR UPDATE")
        .bind(platform)
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), worker.apply(&mut online, &cancel))
        .await
        .unwrap()
        .unwrap();
    lock.rollback().await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), updates.recv())
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert_eq!(
        *plans.lock().unwrap(),
        vec![ScopedEventRequest {
            platform_id: platform,
            refresh: EventRefresh::Platform
        }]
    );
    assert_eq!(
        *alerts.observations.lock().unwrap(),
        vec![true, false, false]
    );
    // A delayed transition for an obsolete connector cannot alter persisted state.
    let mut obsolete = target.clone();
    obsolete.address = "obsolete".into();
    worker
        .apply(&mut Progress::new(transition(&obsolete, false)), &cancel)
        .await
        .unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM platforms WHERE id=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Online");
    assert_eq!(alerts.observations.lock().unwrap().len(), 3);
    cancel.cancel();
    scopes.await.unwrap();
    sqlx::query("DELETE FROM containers WHERE platformid=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM deployments WHERE id=$1")
        .bind(deployment)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn local_health_monitor_emits_transition_while_all_database_connections_are_held() {
    use axum::{Router, response::IntoResponse};
    let url = std::env::var("CITADEL_PHASE4_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let platform = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',0,0,0,0,0,'{\"$type\":\"Docker\"}','Online')").bind(platform).execute(&pool).await.unwrap();
    let targets = PlatformRuntimeRegistry::new(pool.clone(), None);
    targets.refresh().await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let docker = DockerClient::with_endpoint(
        format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap(),
        Duration::from_secs(1),
        "/host",
    )
    .unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let probes = calls.clone();
    let router = Router::new().fallback(move |request: axum::extract::Request| {
        let probes = probes.clone();
        async move {
            let path = request.uri().path();
            if path.ends_with("/_ping") {
                probes.fetch_add(1, Ordering::SeqCst);
                "OK".into_response()
            } else if path == "/version" {
                axum::Json(serde_json::json!({"ApiVersion":"1.49","MinAPIVersion":"1.41"}))
                    .into_response()
            } else {
                panic!("health requested unrelated Docker API: {path}");
            }
        }
    });
    let cancel = CancellationToken::new();
    let server_cancel = cancel.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(server_cancel.cancelled_owned())
            .await
            .unwrap();
    });
    let connection = pool.acquire().await.unwrap();
    let (transitions, mut receiver) = bounded_channel(4, QueueOverflowPolicy::Wait);
    let monitor = tokio::spawn(platform_health_monitor(
        cancel.clone(),
        HealthWorker {
            docker,
            targets,
            pool: pool.clone(),
            transitions,
        },
    ));
    let observed = tokio::time::timeout(Duration::from_secs(8), receiver.recv(&cancel))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(observed.target.id, platform);
    assert!(observed.online);
    assert!(calls.load(Ordering::SeqCst) >= 2);
    assert!(
        tokio::time::timeout(Duration::from_millis(5300), receiver.recv(&cancel))
            .await
            .is_err()
    );
    assert!(calls.load(Ordering::SeqCst) >= 3);
    cancel.cancel();
    monitor.await.unwrap().unwrap();
    server.await.unwrap();
    drop(connection);
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test(start_paused = true)]
async fn duplicate_transitions_during_effect_keep_one_plan() {
    let cancel = CancellationToken::new();
    let target = target();
    let (sender, receiver) = bounded_channel(4, QueueOverflowPolicy::Wait);
    let calls = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(tokio::sync::Notify::new());
    let (effects, wake, token) = (calls.clone(), release.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run_transitions(&token, receiver, |progress| {
            effects.fetch_add(1, Ordering::SeqCst);
            let wake = wake.clone();
            async move {
                wake.notified().await;
                (progress, Ok::<_, &str>(()))
            }
        })
        .await;
    });
    sender
        .send(transition(&target, true), &cancel)
        .await
        .ok()
        .unwrap();
    settle().await;
    for _ in 0..100 {
        sender
            .send(transition(&target, true), &cancel)
            .await
            .ok()
            .unwrap();
    }
    settle().await;
    release.notify_one();
    settle().await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    cancel.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn failed_platform_backlog_cannot_hide_a_newer_online_transition() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = bounded_channel(512, QueueOverflowPolicy::Wait);
    let plans = Arc::new(AtomicUsize::new(0));
    let (recoveries, token) = (plans.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run_transitions(&token, receiver, |progress| {
            let result = if progress.transition.online {
                recoveries.fetch_add(1, Ordering::SeqCst);
                Ok(())
            } else {
                Err("offline alert unavailable")
            };
            async move { (progress, result) }
        })
        .await;
    });
    let recovering = target();
    sender
        .send(transition(&recovering, false), &cancel)
        .await
        .ok()
        .unwrap();
    for _ in 0..260 {
        sender
            .send(transition(&target(), false), &cancel)
            .await
            .ok()
            .unwrap();
    }
    settle().await;
    sender
        .send(transition(&recovering, true), &cancel)
        .await
        .ok()
        .unwrap();
    settle().await;
    assert_eq!(plans.load(Ordering::SeqCst), 1);
    cancel.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn offline_alerts_are_rechecked_without_repeating_effects_and_stop_on_recovery() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = bounded_channel(4, QueueOverflowPolicy::Wait);
    let target = target();
    let attempts = Arc::new(AtomicUsize::new(0));
    let writes = Arc::new(AtomicUsize::new(0));
    let (observations, effects, token) = (attempts.clone(), writes.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run_transitions(&token, receiver, |mut progress| {
            if !progress.persisted {
                effects.fetch_add(1, Ordering::SeqCst);
                progress.persisted = true;
            }
            progress.recheck_alert = !progress.transition.online;
            observations.fetch_add(1, Ordering::SeqCst);
            async { (progress, Ok::<_, &str>(())) }
        })
        .await;
    });
    sender
        .send(transition(&target, false), &cancel)
        .await
        .ok()
        .unwrap();
    settle().await;
    for _ in 0..21 {
        tokio::time::advance(OFFLINE_ALERT_INTERVAL).await;
        settle().await;
    }
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        22,
        "observations continue past the ten-minute cooldown"
    );
    assert_eq!(writes.load(Ordering::SeqCst), 1, "status effects run once");
    sender
        .send(transition(&target, true), &cancel)
        .await
        .ok()
        .unwrap();
    settle().await;
    tokio::time::advance(OFFLINE_ALERT_INTERVAL * 2).await;
    settle().await;
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        23,
        "recovery stops offline rechecks"
    );
    assert_eq!(writes.load(Ordering::SeqCst), 2);
    cancel.cancel();
    worker.await.unwrap();
}

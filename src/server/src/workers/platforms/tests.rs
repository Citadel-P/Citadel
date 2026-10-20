use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn agent_container_event_uses_the_bound_platform_without_scanning_local_docker() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let platform = uuid::Uuid::now_v7();
    let deployment = uuid::Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,cpucount,imagecount,memtotal,networkcount,volumecount,address,connectortype,platformdescriptor,status) VALUES($1,$2,0,0,0,0,0,$3,'Agent','{\"$type\":\"Docker\"}','Online')")
        .bind(platform).bind(format!("event-{platform}")).bind(format!("https://{platform}.invalid")).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,$2,$3,'{}','Healthy',$4)")
        .bind(deployment).bind(format!("event-{deployment}")).bind(platform).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,deploymentid,dockercontainerid,dockerimageid,name,created,updated,state,ports) VALUES($1,$2,$3,'container-event','image','web',1,1,'Running','[]')")
        .bind(uuid::Uuid::now_v7()).bind(platform).bind(deployment).execute(&pool).await.unwrap();
    let docker = DockerClient::new(
        "/nonexistent-citadel-event-test.sock",
        Duration::from_millis(100),
    )
    .unwrap();
    let event = InventoryEvent {
        resource: None,
        stream_recovered: false,
        swarm_scope: false,
        kind: RuntimeEventKind::Container(ContainerChange::Exited),
        platform_id: Some(platform),
        container_id: Some("container-event".into()),
        container_state: Some("exited".into()),
        container_name: Some("web".into()),
        container: None,
        source: ReconciliationTrigger::AgentEvent,
        action: "die".into(),
        event_time_millis: None,
    };
    let targets = PlatformRuntimeRegistry::new(pool.clone(), None);
    targets.refresh().await.unwrap();
    let hub = crate::realtime::RealtimeHub::new(16, Arc::new(crate::metrics::Metrics::default()));
    let mut updates = hub.subscribe();
    assert!(
        apply_container_event(
            &event,
            &docker,
            crate::workers::test_support::projections(&pool).as_ref(),
            Some(&hub),
            &CancellationToken::new(),
            &targets
        )
        .await
        .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM deployments WHERE id=$1")
            .bind(deployment)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Stopped"
    );
    tokio::time::timeout(Duration::from_secs(2), updates.recv())
        .await
        .unwrap()
        .unwrap();
    let revision: i64 = sqlx::query_scalar("SELECT rowversion FROM containers WHERE platformid=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    // Holding the owner lock proves a duplicate does not attempt reconciliation.
    let mut owner = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM deployments WHERE id=$1 FOR NO KEY UPDATE")
        .bind(deployment)
        .fetch_one(&mut *owner)
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(
            Duration::from_secs(1),
            apply_container_event(
                &event,
                &docker,
                crate::workers::test_support::projections(&pool).as_ref(),
                Some(&hub),
                &CancellationToken::new(),
                &targets,
            )
        )
        .await
        .expect("duplicate must not wait for the deployment lock")
        .unwrap(),
        "no-op is handled without fallback enumeration"
    );
    owner.rollback().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT rowversion FROM containers WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap(),
        revision
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(200), updates.recv())
            .await
            .is_err(),
        "no duplicate realtime event after the coalescing window"
    );
    pool.close().await;
}
#[test]
fn local_event_bursts_cannot_hide_an_agent_reconciliation_trigger() {
    let (local, mut local_receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
    let (agent, mut agent_receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
    let agent = AgentReconciliationSignal {
        queue: agent,
        full: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    };
    let local_event = InventoryEvent {
        resource: None,
        stream_recovered: true,
        swarm_scope: false,
        kind: RuntimeEventKind::Container(ContainerChange::Running),
        platform_id: None,
        container_id: None,
        container_state: None,
        container_name: None,
        container: None,
        source: ReconciliationTrigger::LocalEvent,
        action: "start".into(),
        event_time_millis: None,
    };
    let agent_event = InventoryEvent {
        resource: None,
        stream_recovered: true,
        swarm_scope: false,
        kind: RuntimeEventKind::SwarmDirty(citadel_docker::client::events::SwarmResource::Service),
        platform_id: None,
        container_id: None,
        container_state: None,
        container_name: None,
        container: None,
        source: ReconciliationTrigger::AgentEvent,
        action: "update".into(),
        event_time_millis: None,
    };

    queue_stream_recovery(&local_event, &local, &agent);
    queue_stream_recovery(&local_event, &local, &agent);
    queue_stream_recovery(&agent_event, &local, &agent);

    assert_eq!(local_receiver.try_recv(), Some(()));
    assert_eq!(local_receiver.try_recv(), None);
    assert_eq!(agent_receiver.try_recv(), Some(None));
}

#[test]
fn overflowing_agent_scopes_upgrade_the_pending_pass_to_all_agents() {
    let (queue, mut receiver) = bounded_channel(1, QueueOverflowPolicy::Reject);
    let signal = AgentReconciliationSignal {
        queue,
        full: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    };
    let first = uuid::Uuid::now_v7();
    signal.request(Some(first));
    signal.request(Some(uuid::Uuid::now_v7()));
    assert_eq!(receiver.try_recv(), Some(Some(first)));
    assert!(signal.full.swap(false, std::sync::atomic::Ordering::AcqRel));
    assert!(receiver.try_recv().is_none());
}

#[test]
fn platform_inventory_failure_maps_to_the_existing_alert_contract() {
    let id = uuid::Uuid::now_v7();
    let target = ReconciliationTarget {
        agent: None,
        id,
        name: "worker-01".into(),
        address: "http://agent:8080".into(),
        connector_type: citadel_platforms::ConnectorKind::Agent,
        platform_type: citadel_platforms::PlatformKind::Docker,
    };
    let error = RuntimeCapabilityError::new(
        citadel_platforms::RuntimeErrorKind::Remote,
        "connection refused",
        true,
    );

    let observation = platform_unreachable_observation(&target, &error);

    assert_eq!(observation.alert_type, "PlatformUnreachable");
    assert_eq!(observation.resource_id, id);
    assert_eq!(observation.resource_type, "Platform");
    assert_eq!(observation.info["PlatformName"], "worker-01");
    assert_eq!(observation.info["Address"], "http://agent:8080");
    assert_eq!(observation.deduplication_component, "inventory");
    assert!(observation.matched);

    let recovered = platform_reachable_observation(&target);
    assert_eq!(recovered.alert_type, observation.alert_type);
    assert_eq!(recovered.deduplication_component, "inventory");
    assert!(!recovered.matched);
}

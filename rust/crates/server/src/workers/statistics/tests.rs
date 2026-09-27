use super::*;

fn scope(node: Option<&str>) -> StatsScope {
    StatsScope {
        platform_id: uuid::Uuid::now_v7(),
        node_id: node.map(str::to_owned),
        connector: "Local".into(),
        address: Some("fixture".into()),
        agent_id: None,
        connected_at: None,
        closed: None,
    }
}
fn sample(id: &str, created: i64) -> RuntimeContainerStat {
    RuntimeContainerStat {
        docker_container_id: id.into(),
        created,
        memory_active: 100.0,
        memory_cache: 0.0,
        cpu_usage: 2.0,
        memory_limit: 200.0,
        rx_bytes: 3.0,
        tx_bytes: 4.0,
    }
}

#[tokio::test]
async fn ingress_preserves_capture_timestamps_and_host_totals_without_database_work() {
    let (containers, mut cq) = writer::channel(8, RuntimeWork::ContainerStatsIngress);
    let (platforms, mut pq) = writer::channel(8, RuntimeWork::PlatformStatsIngress);
    let ingress = StatsIngress {
        containers,
        platforms,
        realtime: None,
    };
    let cancel = CancellationToken::new();
    assert!(
        ingress
            .submit(
                scope(None),
                vec![sample("a", 10), sample("b", 10), sample("a", 11)],
                None,
                &cancel
            )
            .await
    );
    let first = pq.try_recv().unwrap();
    let second = pq.try_recv().unwrap();
    assert_eq!(
        (first.created, first.memory_active, first.cpu_usage),
        (10, 200.0, 4.0)
    );
    assert_eq!((second.created, second.memory_active), (11, 100.0));
    assert!(pq.try_recv().is_err());
    assert_eq!(cq.len(), 3);
    assert_eq!(cq.try_recv().unwrap().sample.created, 10);
}

#[tokio::test]
async fn nodes_never_create_manager_totals_and_empty_hosts_still_emit_platform_samples() {
    let (containers, mut cq) = writer::channel(8, RuntimeWork::ContainerStatsIngress);
    let (platforms, mut pq) = writer::channel(8, RuntimeWork::PlatformStatsIngress);
    let ingress = StatsIngress {
        containers,
        platforms,
        realtime: None,
    };
    let cancel = CancellationToken::new();
    assert!(
        ingress
            .submit(scope(Some("worker")), vec![sample("a", 10)], None, &cancel)
            .await
    );
    assert!(pq.try_recv().is_err());
    assert_eq!(
        cq.try_recv().unwrap().scope.node_id.as_deref(),
        Some("worker")
    );
    let metadata = RuntimePlatformStats {
        disk_used_bytes: Some(50),
        disk_total_bytes: Some(100),
        disk_usage: Some(50.0),
        ..Default::default()
    };
    assert!(
        ingress
            .submit(scope(None), vec![], Some(metadata.clone()), &cancel)
            .await
    );
    let empty = pq.try_recv().unwrap();
    assert_eq!(empty.memory_active, 0.0);
    assert_eq!(empty.metadata, Some(metadata));
}

#[tokio::test]
async fn one_discovery_cycle_has_one_complete_total_across_completion_seconds() {
    let (containers, mut cq) = writer::channel(8, RuntimeWork::ContainerStatsIngress);
    let (platforms, mut pq) = writer::channel(8, RuntimeWork::PlatformStatsIngress);
    let ingress = StatsIngress {
        containers,
        platforms,
        realtime: None,
    };
    let cancel = CancellationToken::new();
    let stats = (0..6)
        .map(|i| sample(&format!("c{i}"), 10 + i / 3))
        .collect();
    assert!(
        ingress
            .submit_cycle(scope(None), stats, None, 9, &cancel)
            .await
    );
    let total = pq.try_recv().unwrap();
    assert_eq!(
        (total.created, total.memory_active, total.cpu_usage),
        (9, 600.0, 12.0)
    );
    assert!(pq.try_recv().is_err());
    for i in 0..6 {
        assert_eq!(cq.try_recv().unwrap().sample.created, 10 + i / 3);
    }
    // Partial cycles only aggregate available observations, never invented zeros.
    assert!(
        ingress
            .submit_cycle(scope(None), vec![sample("ok", 12)], None, 12, &cancel)
            .await
    );
    assert_eq!(pq.try_recv().unwrap().memory_active, 100.0);
}

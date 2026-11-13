use super::{StatsIngress, writer};
use citadel_platforms::{RuntimeContainerStat, RuntimePlatformStats, stats_ingestion::StatsScope};
use citadel_runtime::runtime_metrics::RuntimeWork;
use tokio_util::sync::CancellationToken;

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
        memory_active_cache: None,
        cpu_usage: 2.0,
        memory_limit: 200.0,
        rx_bytes: 3.0,
        tx_bytes: 4.0,
        block_read_bytes: None,
        block_write_bytes: None,
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
        budget: Default::default(),
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
    assert_eq!(first.container_io.len(), 2);
    assert_eq!(first.container_io[0], (&sample("a", 10)).into());
    assert_eq!(second.container_io.len(), 1);
    assert!(pq.try_recv().is_err());
    assert_eq!(cq.len(), 3);
    assert_eq!(cq.try_recv().unwrap().sample.created, 10);
}

#[tokio::test]
async fn nodes_keep_scoped_captures_and_empty_hosts_still_emit_platform_samples() {
    let (containers, mut cq) = writer::channel(8, RuntimeWork::ContainerStatsIngress);
    let (platforms, mut pq) = writer::channel(8, RuntimeWork::PlatformStatsIngress);
    let ingress = StatsIngress {
        containers,
        platforms,
        realtime: None,
        budget: Default::default(),
    };
    let cancel = CancellationToken::new();
    assert!(
        ingress
            .submit(scope(Some("worker")), vec![sample("a", 10)], None, &cancel)
            .await
    );
    let node = pq.try_recv().unwrap();
    assert_eq!(node.scope.node_id.as_deref(), Some("worker"));
    assert_eq!(node.container_io.len(), 1);
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
    assert!(empty.container_io.is_empty());
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
        budget: Default::default(),
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

#[tokio::test]
async fn invalid_sample_cannot_poison_writes_or_publish_a_partial_host_total() {
    let (containers, mut cq) = writer::channel(8, RuntimeWork::ContainerStatsIngress);
    let (platforms, mut pq) = writer::channel(8, RuntimeWork::PlatformStatsIngress);
    let ingress = StatsIngress {
        containers,
        platforms,
        realtime: None,
        budget: Default::default(),
    };
    let mut bad = sample("bad", 10);
    bad.cpu_usage = f64::NAN;
    let mut bad_io = sample("bad-io", 10);
    bad_io.block_read_bytes = Some(-1.0);
    assert!(
        ingress
            .submit(
                scope(None),
                vec![sample("valid", 10), bad, bad_io],
                None,
                &CancellationToken::new()
            )
            .await
    );
    assert_eq!(cq.try_recv().unwrap().sample.docker_container_id, "valid");
    assert!(cq.try_recv().is_err());
    assert!(
        pq.try_recv().is_err(),
        "partial host total must not masquerade as complete"
    );
}

#[tokio::test]
async fn dense_capture_keeps_one_shared_reservation_until_both_writers_release_it() {
    let budget = super::budget::Budget::default();
    let (containers, mut cq) = writer::channel(10_000, RuntimeWork::ContainerStatsIngress);
    let (platforms, mut pq) = writer::channel(2, RuntimeWork::PlatformStatsIngress);
    let ingress = StatsIngress {
        containers,
        platforms,
        realtime: None,
        budget: budget.clone(),
    };
    let stats = (0..10_000)
        .map(|i| sample(&format!("{i:064}"), 10))
        .collect();
    assert!(
        ingress
            .submit_cycle(scope(None), stats, None, 10, &CancellationToken::new())
            .await
    );
    let capture = pq.try_recv().unwrap();
    assert_eq!(capture.container_io.len(), 10_000);
    let actual = serde_json::to_vec(&*capture).unwrap().len();
    assert!(actual < capture.encoded);
    assert!(capture.encoded <= super::budget::BATCH_BYTES);
    let reserved = super::budget::RETAINED_BYTES - budget.available();
    assert!(reserved > actual);
    while let Ok(row) = cq.try_recv() {
        assert!(serde_json::to_vec(&*row).unwrap().len() < row.encoded);
    }
    assert!(budget.available() > super::budget::RETAINED_BYTES - reserved);
    assert!(budget.available() < super::budget::RETAINED_BYTES);
    drop(capture);
    assert_eq!(budget.available(), super::budget::RETAINED_BYTES);
}

#[tokio::test]
async fn oversized_or_saturated_cycle_is_rejected_without_partial_capture_or_wait() {
    let budget = super::budget::Budget::default();
    let (containers, cq) = writer::channel(2, RuntimeWork::ContainerStatsIngress);
    let (platforms, pq) = writer::channel(2, RuntimeWork::PlatformStatsIngress);
    let ingress = StatsIngress {
        containers,
        platforms,
        realtime: None,
        budget: budget.clone(),
    };
    let token = CancellationToken::new();
    assert!(
        ingress
            .submit_cycle(
                scope(None),
                vec![sample(&"x".repeat(super::budget::BATCH_BYTES), 1)],
                None,
                1,
                &token
            )
            .await
    );
    assert_eq!((cq.len(), pq.len()), (0, 0));
    let held = budget.reserve(super::budget::RETAINED_BYTES).unwrap();
    assert!(
        ingress
            .submit_cycle(scope(None), vec![sample("small", 1)], None, 1, &token)
            .await
    );
    assert_eq!((cq.len(), pq.len()), (0, 0));
    drop(held);
    assert_eq!(budget.available(), super::budget::RETAINED_BYTES);
    token.cancel();
    assert!(
        !ingress
            .submit_cycle(scope(None), vec![sample("small", 1)], None, 1, &token)
            .await
    );
    assert_eq!(budget.available(), super::budget::RETAINED_BYTES);
}

#[tokio::test]
async fn dense_host_keeps_all_six_poll_cycles_until_the_normal_minute_flush() {
    let budget = super::budget::Budget::default();
    let (containers, mut cq) = writer::channel(10_000, RuntimeWork::ContainerStatsIngress);
    let (platforms, mut pq) = writer::channel(8, RuntimeWork::PlatformStatsIngress);
    let ingress = StatsIngress {
        containers,
        platforms,
        realtime: None,
        budget: budget.clone(),
    };
    let host = scope(None);
    for cycle in 0..6 {
        let stats = (0..10_000)
            .map(|i| sample(&format!("{i:064}"), cycle * 10))
            .collect();
        assert!(
            ingress
                .submit_cycle(
                    host.clone(),
                    stats,
                    None,
                    cycle * 10,
                    &CancellationToken::new()
                )
                .await
        );
        assert_eq!(
            cq.len(),
            10_000,
            "the byte limit must not hide rejected dense-host cycles"
        );
        while cq.try_recv().is_ok() {}
        assert_eq!(pq.len(), cycle as usize + 1);
    }
    assert!(budget.available() < super::budget::RETAINED_BYTES);
    while pq.try_recv().is_ok() {}
    assert_eq!(budget.available(), super::budget::RETAINED_BYTES);
}

#[tokio::test]
async fn ingress_estimates_each_sample_once_and_keeps_sizes_aligned_after_filtering() {
    let (containers, mut cq) = writer::channel(8, RuntimeWork::ContainerStatsIngress);
    let (platforms, mut pq) = writer::channel(8, RuntimeWork::PlatformStatsIngress);
    let ingress = StatsIngress {
        containers,
        platforms,
        realtime: None,
        budget: Default::default(),
    };
    let scope = scope(None);
    let mut invalid = sample("invalid", 10);
    invalid.memory_active_cache = Some(101.0);
    let mut optional = sample("escaped\"\n\\", 10);
    optional.memory_active_cache = Some(0.0);
    optional.block_read_bytes = Some(20.0);
    let input = vec![
        invalid,
        sample(&"a".repeat(64), 10),
        optional,
        sample(&"large".repeat(1000), 10),
    ];
    let expected: Vec<_> = input
        .iter()
        .skip(1)
        .map(|s| super::budget::container_bytes(super::budget::scope_bytes(&scope), s))
        .collect();
    super::budget::CONTAINER_ESTIMATES.with(|count| count.set(0));
    assert!(
        ingress
            .submit(scope, input, None, &CancellationToken::new())
            .await
    );
    assert_eq!(
        super::budget::CONTAINER_ESTIMATES.with(|count| count.get()),
        4
    );
    for encoded in expected {
        let envelope = cq.try_recv().unwrap();
        assert_eq!(envelope.encoded, encoded);
    }
    assert!(cq.try_recv().is_err());
    assert!(
        pq.try_recv().is_err(),
        "invalid input must suppress the entire host total"
    );
    assert_eq!(ingress.budget.available(), super::budget::RETAINED_BYTES);
}

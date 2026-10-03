use super::*;
use citadel_platforms::{runtime_provider::*, *};
use futures_util::future::BoxFuture;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountedProvider {
    inner: Arc<dyn PlatformRuntimeProvider>,
    resolutions: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
}
struct CountedInventory<'a> {
    inner: Box<dyn NetworkVolumeInventoryPort + 'a>,
    reads: Arc<AtomicUsize>,
}
impl NetworkInventoryPort for CountedInventory<'_> {
    fn list_networks<'a>(
        &'a self,
        token: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeNetworkSummary>, RuntimeCapabilityError>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.inner.list_networks(token)
    }
}
impl VolumeInventoryPort for CountedInventory<'_> {
    fn list_volumes<'a>(
        &'a self,
        token: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeVolumeSummary>, RuntimeCapabilityError>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.inner.list_volumes(token)
    }
}
impl swarm_mutations::SwarmSnapshotPort for CountedProvider {
    fn snapshot<'a>(
        &'a self,
        platform: &'a PlatformDetails,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeInventorySnapshot, RuntimeCapabilityError>> {
        self.inner.snapshot(platform, cancel)
    }
}
impl PlatformRuntimeProvider for CountedProvider {
    fn swarm<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn SwarmRuntimePort + 'a>, RuntimeCapabilityError>> {
        self.inner.swarm(platform, cancel)
    }
    fn image_mutations<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn ImageMutationRuntimePort + 'a>, RuntimeCapabilityError>> {
        self.inner.image_mutations(platform, cancel)
    }
    fn pruning<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn prune::PlatformPrunePort + 'a>, RuntimeCapabilityError>> {
        self.inner.pruning(platform, cancel)
    }
    fn networks<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn NetworkRuntimePort + 'a>, RuntimeCapabilityError>> {
        self.inner.networks(platform, node, cancel)
    }
    fn volumes<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn VolumeRuntimePort + 'a>, RuntimeCapabilityError>> {
        self.inner.volumes(platform, node, cancel)
    }
    fn container_inspection<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<Box<dyn containers::ContainerInspectionPort + 'a>, RuntimeCapabilityError>,
    > {
        self.inner.container_inspection(platform, node, cancel)
    }
    fn images<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<
            Box<dyn citadel_platforms::images::ImageInspectionPort + 'a>,
            RuntimeCapabilityError,
        >,
    > {
        self.inner.images(platform, node, cancel)
    }
    fn logs<'a>(
        &'a self,
        platform: Uuid,
        node: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<Box<dyn citadel_platforms::logs::LogReadPort + 'a>, RuntimeCapabilityError>,
    > {
        self.inner.logs(platform, node, cancel)
    }
    fn inventory<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn NetworkVolumeInventoryPort + 'a>, RuntimeCapabilityError>>
    {
        Box::pin(async move {
            self.resolutions.fetch_add(1, Ordering::SeqCst);
            let inner = self.inner.inventory(platform, cancel).await?;
            Ok(Box::new(CountedInventory {
                inner,
                reads: self.reads.clone(),
            }) as Box<dyn NetworkVolumeInventoryPort>)
        })
    }
    fn tasks<'a>(
        &'a self,
        platform: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Box<dyn SwarmTaskRuntimePort + 'a>, RuntimeCapabilityError>> {
        self.inner.tasks(platform, cancel)
    }
    fn streams(&self, platform: Uuid, node: Option<&str>) -> Box<dyn ContainerStreamsPort> {
        self.inner.streams(platform, node)
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn coarse_events_share_resolution_and_reads_but_keep_actor_capabilities() {
    let f = fixture().await;
    let mut groups = reader(&f);
    let resolutions = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    groups.runtime = Arc::new(CountedProvider {
        inner: groups.runtime.clone(),
        resolutions: resolutions.clone(),
        reads: reads.clone(),
    });
    let actor = super::super::lookup::subject(&f).await;
    super::super::lookup::grant(
        &f,
        actor.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let hub = citadel_server::realtime::RealtimeHub::new(16, Arc::new(Metrics::default()));
    let mut changes = hub.subscribe();
    let daemon = Group::parse(&format!("docker-daemon:{}", f.platform_id)).unwrap();
    for (index, kind) in ["network", "volume"].into_iter().enumerate() {
        hub.publish_runtime_change(f.platform_id, kind, "update", "");
        let event = changes.recv().await.unwrap();
        for principal in [&actor, &f.administrator, &actor, &f.administrator] {
            let snapshot = groups.read(principal, &daemon, Some(&event)).await.unwrap();
            assert_eq!(snapshot.rows.len(), 1);
            for row in &snapshot.rows[0].rows {
                assert_eq!(
                    row["capabilities"]["canInspect"],
                    principal.is_administrator()
                );
            }
        }
        assert_eq!(resolutions.load(Ordering::SeqCst), index + 1);
        assert_eq!(reads.load(Ordering::SeqCst), index + 1);
    }
    cleanup(f).await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn swarm_image_replacements_keep_per_actor_inspect_capabilities() {
    let (f, _) = super::super::swarm_inventory::fixture_swarm().await;
    let groups = reader(&f);
    let actor = super::super::lookup::subject(&f).await;
    super::super::lookup::grant(
        &f,
        actor.actor_id.value(),
        ResourceType::Platform,
        f.platform_id,
        0,
    )
    .await;
    let hub = citadel_server::realtime::RealtimeHub::new(16, Arc::new(Metrics::default()));
    let mut changes = hub.subscribe();
    let daemon = Group::parse(&format!("docker-daemon:{}", f.platform_id)).unwrap();
    hub.publish_resource_observation(
        f.platform_id,
        None,
        "update",
        &jobs::ResourceDelta::Image {
            id: "image".into(),
            value: None,
        },
    );
    let event = changes.recv().await.unwrap();
    for principal in [&actor, &f.administrator] {
        let snapshot = groups.read(principal, &daemon, Some(&event)).await.unwrap();
        assert!(!snapshot.rows[0].rows.is_empty());
        for row in &snapshot.rows[0].rows {
            assert_eq!(
                row["capabilities"]["canInspect"],
                principal.is_administrator()
            );
        }
        let replacement = snapshot
            .events
            .iter()
            .find(|e| e.target == "SwarmNodeLocalResourcesUpdated")
            .unwrap();
        for row in replacement.arguments[0]["images"].as_array().unwrap() {
            assert_eq!(
                row["capabilities"]["canInspect"],
                principal.is_administrator()
            );
        }
    }
    cleanup(f).await;
}

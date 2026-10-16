//! Compare immutable workload baselines without changing their deployment identity.
use super::image_updates::ImageUpdateAdapter;
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind, RuntimeSwarmNode, RuntimeSwarmTask,
    image_updates::{ImagePlatform, ImageUpdateRegistry, ImageUpdateResolver},
    images::ImageInspectionPort,
};
use std::collections::BTreeSet;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(crate) struct WorkloadImageCheck<'a> {
    pub platform: Uuid,
    pub registry: Uuid,
    pub reference: &'a str,
    pub current: &'a str,
    pub remote: &'a str,
}

impl ImageUpdateAdapter {
    pub(crate) async fn workload_content_changed(
        &self,
        check: &WorkloadImageCheck<'_>,
        targets: &[ImagePlatform],
        cancel: &CancellationToken,
    ) -> Result<bool, RuntimeCapabilityError> {
        let registry = self.load_registry(check.registry).await?;
        let changed = compare_content(self, &registry, check, targets, cancel).await?;
        if self.load_registry(check.registry).await?.revision != registry.revision {
            return Err(unavailable(
                "Registry configuration changed during the check.",
            ));
        }
        Ok(changed)
    }

    pub(crate) async fn deployed_image_target(
        &self,
        check: &WorkloadImageCheck<'_>,
        image_id: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<ImagePlatform, RuntimeCapabilityError> {
        let registry = self.load_registry(check.registry).await?;
        let reference = pinned_reference(&registry.host, check.reference, check.current)?;
        let image = self
            .runtime
            .images(check.platform, None, cancel)
            .await?
            .inspect_image(image_id.unwrap_or(&reference), cancel)
            .await?;
        if image_id.is_some_and(|id| image.id != id) {
            return Err(unavailable(
                "The deployed image identity changed during the check.",
            ));
        }
        if image.os.is_empty()
            || image.os == "unknown"
            || image.architecture.is_empty()
            || image.architecture == "unknown"
        {
            return Err(unavailable("The deployed image's platform is unavailable."));
        }
        if self.load_registry(check.registry).await?.revision != registry.revision {
            return Err(unavailable(
                "Registry configuration changed during the check.",
            ));
        }
        Ok(ImagePlatform {
            os: image.os,
            architecture: image.architecture,
            variant: image.variant.unwrap_or_default(),
        })
    }
}

async fn compare_content(
    resolver: &dyn ImageUpdateResolver,
    registry: &ImageUpdateRegistry,
    check: &WorkloadImageCheck<'_>,
    targets: &[ImagePlatform],
    cancel: &CancellationToken,
) -> Result<bool, RuntimeCapabilityError> {
    if targets.is_empty() || targets.len() > 16 {
        return Err(unavailable(
            "The workload's image platforms are unavailable or exceed the check limit.",
        ));
    }
    let current = pinned_reference(&registry.host, check.reference, check.current)?;
    let remote = pinned_reference(&registry.host, check.reference, check.remote)?;
    let mut changed = false;
    // Complete every comparison: incomplete coverage must never trigger Apply.
    for target in targets.iter().collect::<BTreeSet<_>>() {
        let before = resolver
            .observe(check.platform, None, registry, &current, target, cancel)
            .await?;
        let after = resolver
            .observe(check.platform, None, registry, &remote, target, cancel)
            .await?;
        if before.index_digest != digest(check.current)?
            || after.index_digest != digest(check.remote)?
        {
            return Err(unavailable(
                "The Registry did not confirm the requested immutable image.",
            ));
        }
        changed |= before.config_digest != after.config_digest;
    }
    Ok(changed)
}

fn digest(value: &str) -> Result<&str, RuntimeCapabilityError> {
    let value = value.rsplit('@').next().unwrap_or(value);
    if !value
        .strip_prefix("sha256:")
        .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err(unavailable("The workload has no comparable image digest."));
    }
    Ok(value)
}
fn pinned_reference(
    host: &str,
    reference: &str,
    value: &str,
) -> Result<String, RuntimeCapabilityError> {
    let reference =
        crate::connectors::routing::deployments::qualify_image_reference(host, reference)
            .map_err(|_| unavailable("The image reference is unavailable."))?;
    let repository = reference.split('@').next().unwrap_or(&reference);
    let repository = repository
        .rsplit_once(':')
        .filter(|(_, tag)| !tag.contains('/'))
        .map_or(repository, |(repo, _)| repo);
    Ok(format!("{repository}@{}", digest(value)?))
}

pub(crate) fn swarm_targets(
    service: &str,
    nodes: &[RuntimeSwarmNode],
    tasks: &[RuntimeSwarmTask],
) -> Result<Vec<ImagePlatform>, RuntimeCapabilityError> {
    let mut targets = BTreeSet::new();
    for task in tasks
        .iter()
        .filter(|t| t.service_id == service && t.desired_state.eq_ignore_ascii_case("running"))
    {
        let node = nodes
            .iter()
            .find(|n| !task.node_id.is_empty() && n.id == task.node_id)
            .ok_or_else(|| unavailable("A workload task has no known node platform."))?;
        let (architecture, variant) = match node.architecture.as_str() {
            "x86_64" => ("amd64", ""),
            "aarch64" => ("arm64", ""),
            "armv7l" => ("arm", "v7"),
            "armv6l" => ("arm", "v6"),
            other => (other, ""),
        };
        if architecture.is_empty()
            || architecture == "unknown"
            || node.operating_system.is_empty()
            || node.operating_system == "unknown"
        {
            return Err(unavailable(
                "A workload node's image platform is unavailable.",
            ));
        }
        targets.insert(ImagePlatform {
            os: node.operating_system.clone(),
            architecture: architecture.into(),
            variant: variant.into(),
        });
    }
    if targets.is_empty() || targets.len() > 16 {
        return Err(unavailable(
            "The Service has no assigned tasks with comparable image platforms.",
        ));
    }
    Ok(targets.into_iter().collect())
}
fn unavailable(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Unavailable, message, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_platforms::image_updates::RegistryImageObservation;
    use futures_util::future::BoxFuture;
    use std::sync::Mutex;
    struct Resolver {
        host_changed: bool,
        wrong_identity: bool,
        fail_arm: bool,
        calls: Mutex<Vec<String>>,
    }
    impl ImageUpdateResolver for Resolver {
        fn registries(
            &self,
            _: citadel_primitives::ActorId,
            _: bool,
        ) -> BoxFuture<'_, Result<Vec<ImageUpdateRegistry>, RuntimeCapabilityError>> {
            unreachable!()
        }
        fn observe<'a>(
            &'a self,
            _: Uuid,
            node: Option<&'a str>,
            _: &'a ImageUpdateRegistry,
            reference: &'a str,
            target: &'a ImagePlatform,
            cancel: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<RegistryImageObservation, RuntimeCapabilityError>> {
            Box::pin(async move {
                assert!(
                    node.is_none(),
                    "registry metadata uses the manager, not a restricted node Agent"
                );
                self.calls.lock().unwrap().push(reference.into());
                if cancel.is_cancelled() || (self.fail_arm && target.architecture == "arm64") {
                    return Err(unavailable("unavailable"));
                }
                let latest = reference.ends_with(&"b".repeat(64));
                let config = if latest && (self.host_changed || target.architecture == "arm64") {
                    'd'
                } else {
                    'c'
                };
                Ok(RegistryImageObservation {
                    config_digest: hash(config),
                    manifest_digest: hash('e'),
                    index_digest: if self.wrong_identity {
                        hash('f')
                    } else {
                        reference.split_once('@').unwrap().1.into()
                    },
                    observed_at: chrono::Utc::now(),
                })
            })
        }
    }
    fn hash(c: char) -> String {
        format!("sha256:{}", c.to_string().repeat(64))
    }
    fn target(architecture: &str) -> ImagePlatform {
        ImagePlatform {
            os: "linux".into(),
            architecture: architecture.into(),
            variant: String::new(),
        }
    }
    #[tokio::test]
    async fn workload_content_ignores_unrelated_platforms_but_requires_complete_verified_coverage()
    {
        let registry = ImageUpdateRegistry {
            id: Uuid::now_v7(),
            host: "docker.io".into(),
            revision: "revision".into(),
        };
        let current = hash('a');
        let remote = hash('b');
        let check = WorkloadImageCheck {
            platform: Uuid::now_v7(),
            registry: registry.id,
            reference: "library/app:ReleaseA",
            current: &current,
            remote: &remote,
        };
        let mut resolver = Resolver {
            host_changed: false,
            wrong_identity: false,
            fail_arm: false,
            calls: Mutex::new(vec![]),
        };
        let cancel = CancellationToken::new();
        assert!(
            !compare_content(&resolver, &registry, &check, &[target("amd64")], &cancel)
                .await
                .unwrap()
        );
        assert!(
            compare_content(
                &resolver,
                &registry,
                &check,
                &[target("amd64"), target("arm64")],
                &cancel
            )
            .await
            .unwrap()
        );
        resolver.host_changed = true;
        assert!(
            compare_content(&resolver, &registry, &check, &[target("amd64")], &cancel)
                .await
                .unwrap()
        );
        resolver.fail_arm = true;
        assert!(
            compare_content(
                &resolver,
                &registry,
                &check,
                &[target("amd64"), target("arm64")],
                &cancel
            )
            .await
            .is_err()
        );
        resolver.wrong_identity = true;
        assert!(
            compare_content(&resolver, &registry, &check, &[target("amd64")], &cancel)
                .await
                .is_err()
        );
        assert!(
            compare_content(&resolver, &registry, &check, &[], &cancel)
                .await
                .is_err()
        );
        cancel.cancel();
        assert!(
            compare_content(&resolver, &registry, &check, &[target("amd64")], &cancel)
                .await
                .is_err()
        );
        assert!(
            resolver
                .calls
                .lock()
                .unwrap()
                .iter()
                .all(|r| r.contains('@') && !r.contains("ReleaseA"))
        );
    }
    #[test]
    fn swarm_targets_follow_assigned_workload_nodes_instead_of_the_manager() {
        let nodes = vec![
            RuntimeSwarmNode {
                id: "manager".into(),
                operating_system: "linux".into(),
                architecture: "aarch64".into(),
                ..Default::default()
            },
            RuntimeSwarmNode {
                id: "worker".into(),
                operating_system: "linux".into(),
                architecture: "x86_64".into(),
                ..Default::default()
            },
        ];
        let mut tasks = vec![
            RuntimeSwarmTask {
                service_id: "app".into(),
                node_id: "worker".into(),
                desired_state: "running".into(),
                ..Default::default()
            },
            RuntimeSwarmTask {
                service_id: "app".into(),
                node_id: "manager".into(),
                desired_state: "shutdown".into(),
                ..Default::default()
            },
        ];
        assert_eq!(
            swarm_targets("app", &nodes, &tasks).unwrap(),
            vec![target("amd64")]
        );
        tasks[1].desired_state = "running".into();
        assert_eq!(swarm_targets("app", &nodes, &tasks).unwrap().len(), 2);
        tasks[1].node_id.clear();
        assert!(swarm_targets("app", &nodes, &tasks).is_err());
        assert!(swarm_targets("scaled-to-zero", &nodes, &tasks).is_err());
    }
    #[test]
    fn pinned_baselines_preserve_registry_ports_and_cannot_follow_the_downloaded_tag() {
        assert_eq!(
            pinned_reference("registry.example:5000", "team/app:ReleaseA", &hash('a')).unwrap(),
            format!("registry.example:5000/team/app@{}", hash('a'))
        );
        assert!(pinned_reference("docker.io", "app:latest", "not-a-digest").is_err());
    }
}

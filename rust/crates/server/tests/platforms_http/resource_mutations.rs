use super::*;
use citadel_platforms::{
    CreateRuntimeNetwork, CreateRuntimeVolume, CreatedRuntimeNetwork, NetworkMutationPort,
    NetworkObservationPort, RuntimeCapabilityError, RuntimeErrorKind, VolumeMutationPort,
    resource_mutations::{self, NetworkDeletionError},
};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct Runtime {
    inspected: std::sync::Mutex<Vec<String>>,
    deleted: std::sync::Mutex<Vec<String>>,
    protected: Option<RuntimeNetworkSummary>,
}
impl NetworkObservationPort for Runtime {
    fn inspect_network<'a>(
        &'a self,
        id: &'a str,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeNetworkSummary, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.inspected.lock().unwrap().push(id.into());
            Ok(if id == "protected" {
                self.protected.clone().unwrap()
            } else {
                RuntimeNetworkSummary {
                    name: id.into(),
                    scope: "local".into(),
                    ..Default::default()
                }
            })
        })
    }
}
impl Runtime {
    fn remove(&self, id: &str) -> Result<(), RuntimeCapabilityError> {
        self.deleted.lock().unwrap().push(id.into());
        match id {
            "missing" => Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::NotFound,
                "gone",
                false,
            )),
            "fail" => Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Remote,
                "rejected",
                false,
            )),
            _ => Ok(()),
        }
    }
}
impl NetworkMutationPort for Runtime {
    fn create_network<'a>(
        &'a self,
        _: &'a CreateRuntimeNetwork,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<CreatedRuntimeNetwork, RuntimeCapabilityError>> {
        unreachable!()
    }
    fn delete_network<'a>(
        &'a self,
        id: &'a str,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move { self.remove(id) })
    }
}
impl VolumeMutationPort for Runtime {
    fn create_volume<'a>(
        &'a self,
        _: &'a CreateRuntimeVolume,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        unreachable!()
    }
    fn delete_volume<'a>(
        &'a self,
        name: &'a str,
        force: bool,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            assert!(force);
            self.remove(name)
        })
    }
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn resource_deletion_preflights_networks_and_reports_only_confirmed_removals() {
    let f = fixture().await;
    let reads = &f.lookup_state.platforms.platforms;
    let cancel = CancellationToken::new();
    for (labels, count, swarm, scope) in [
        (
            BTreeMap::from([("com.citadel.system".into(), "true".into())]),
            0,
            false,
            "local",
        ),
        (BTreeMap::new(), 1, false, "local"),
        (
            BTreeMap::from([("com.docker.stack.namespace".into(), "app".into())]),
            0,
            false,
            "local",
        ),
        (
            BTreeMap::from([("com.citadel.stack-id".into(), "app".into())]),
            0,
            false,
            "local",
        ),
        (BTreeMap::new(), 0, true, "local"),
        // No current Swarm projection: validation must fail before any deletion.
        (BTreeMap::new(), 0, true, "swarm"),
    ] {
        let runtime = Runtime {
            protected: Some(RuntimeNetworkSummary {
                name: "protected".into(),
                labels,
                container_count: count,
                scope: scope.into(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ids = if swarm {
            vec!["protected".into()]
        } else {
            vec!["first".into(), "protected".into()]
        };
        let result = resource_mutations::delete_networks(
            reads,
            &runtime,
            f.platform_id,
            swarm,
            &ids,
            &cancel,
            |_| panic!("must not publish a failed preflight"),
        )
        .await;
        assert!(result.is_err());
        assert!(
            runtime.deleted.lock().unwrap().is_empty(),
            "full preflight must precede deletion"
        );
    }
    let ids = vec![
        "first".into(),
        "missing".into(),
        "fail".into(),
        "unattempted".into(),
    ];
    for networks in [true, false] {
        let runtime = Runtime::default();
        let removed = std::sync::Mutex::new(Vec::new());
        let notify = |id: &str| removed.lock().unwrap().push(id.to_owned());
        let error = if networks {
            match resource_mutations::delete_networks(
                reads,
                &runtime,
                f.platform_id,
                false,
                &ids,
                &cancel,
                notify,
            )
            .await
            .unwrap_err()
            {
                NetworkDeletionError::Runtime(error) => error,
                error => panic!("unexpected error: {error}"),
            }
        } else {
            resource_mutations::delete_volumes(&runtime, &ids, true, &cancel, notify)
                .await
                .unwrap_err()
        };
        assert_eq!(error.kind, RuntimeErrorKind::Conflict);
        assert!(error.message.starts_with("Deleted 2 of 4 "));
        assert_eq!(*removed.lock().unwrap(), ["first", "missing"]);
        assert_eq!(
            *runtime.deleted.lock().unwrap(),
            ["first", "missing", "fail"]
        );
        if networks {
            assert_eq!(*runtime.inspected.lock().unwrap(), ids);
        }
    }
    f.docker_server.abort();
    f.pool.close().await;
    std::fs::remove_file(f.docker_socket).unwrap();
}

use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct Fixture {
    calls: Mutex<Vec<String>>,
    reject_claim: bool,
    fail_service: bool,
    fail_cleanup: bool,
    fail_finish: bool,
    cancel_during_delete: bool,
}
impl Fixture {
    fn record(&self, call: impl Into<String>) {
        self.calls.lock().unwrap().push(call.into());
    }
}
impl NodeAgentLifecycleStore for Fixture {
    fn claim_remove(
        &self,
        actor: ActorId,
        platform_id: Uuid,
        _: RuntimePlatformInfo,
    ) -> BoxFuture<'_, Result<NodeAgentRemovalClaim, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.record("claim");
            if self.reject_claim {
                return Err(failure("Already running"));
            }
            Ok(NodeAgentRemovalClaim {
                platform_id,
                actor,
                platform_name: "fixture".into(),
                operation_id: Uuid::now_v7(),
                cluster_id: "cluster".into(),
                manager_node_id: "manager".into(),
                manager_daemon_id: "daemon".into(),
                service_id: Some("agent-service".into()),
                secret_ids: vec!["bootstrap".into()],
                ca_config_id: Some("ca".into()),
            })
        })
    }
    fn revoke<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>> {
        Box::pin(async {
            self.record("revoke");
            Ok(vec!["worker".into()])
        })
    }
    fn finish<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        error: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.record(if error.is_some() {
                "finish:failed"
            } else {
                "finish:completed"
            });
            if self.fail_finish {
                Err(failure("database unavailable"))
            } else {
                Ok(())
            }
        })
    }
}
impl NodeAgentLifecycleRuntime for Fixture {
    fn info<'a>(
        &'a self,
        _: Uuid,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        Box::pin(async {
            self.record("info");
            Ok(RuntimePlatformInfo::default())
        })
    }
    fn delete_owned<'a>(
        &'a self,
        _: &'a NodeAgentRemovalClaim,
        kind: NodeAgentResource,
        _: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            self.record(format!("delete:{kind:?}"));
            if self.cancel_during_delete {
                cancellation.cancel();
                std::future::pending::<()>().await;
            }
            if (matches!(kind, NodeAgentResource::Service) && self.fail_service)
                || (!matches!(kind, NodeAgentResource::Service) && self.fail_cleanup)
            {
                Err(failure("Ownership mismatch"))
            } else {
                Ok(())
            }
        })
    }
    fn disconnect(&self, _: Uuid, nodes: &[String]) {
        assert_eq!(nodes, ["worker"]);
        self.record("disconnect");
    }
}
async fn run(fixture: Arc<Fixture>, capacity: usize) -> Vec<NodeAgentProgress> {
    let (sender, mut receiver) = mpsc::channel(capacity);
    let service = NodeAgentRemovalService {
        store: fixture.clone(),
        runtime: fixture,
        changed: Arc::new(|_| {}),
    };
    let worker = tokio::spawn(async move {
        service
            .remove(
                ActorId::new(Uuid::now_v7()),
                Uuid::now_v7(),
                sender,
                CancellationToken::new(),
            )
            .await;
    });
    let mut messages = Vec::new();
    while let Some(message) = receiver.recv().await {
        messages.push(message);
    }
    worker.await.unwrap();
    messages
}

#[tokio::test]
async fn removal_deletes_service_before_revoking_and_preserves_volume_state() {
    let fixture = Arc::new(Fixture::default());
    let messages = run(fixture.clone(), 32).await;
    assert_eq!(
        *fixture.calls.lock().unwrap(),
        [
            "info",
            "claim",
            "delete:Service",
            "revoke",
            "disconnect",
            "delete:Secret",
            "delete:Config",
            "finish:completed"
        ]
    );
    assert!(messages.last().unwrap().is_completed);
    assert!(messages.last().unwrap().error_message.is_none());
}

#[tokio::test]
async fn service_ownership_failure_does_not_revoke_credentials() {
    let fixture = Arc::new(Fixture {
        fail_service: true,
        ..Default::default()
    });
    let messages = run(fixture.clone(), 32).await;
    assert_eq!(
        *fixture.calls.lock().unwrap(),
        ["info", "claim", "delete:Service", "finish:failed"]
    );
    assert!(messages.last().unwrap().error_message.is_some());
}

#[tokio::test]
async fn cleanup_failures_report_warnings_without_restoring_revoked_credentials() {
    let messages = run(
        Arc::new(Fixture {
            fail_cleanup: true,
            ..Default::default()
        }),
        32,
    )
    .await;
    assert_eq!(messages.iter().filter(|p| p.is_warning).count(), 2);
    assert!(messages.last().unwrap().is_completed);
    assert!(messages.last().unwrap().error_message.is_none());
}

#[tokio::test]
async fn cancellation_persists_failure_instead_of_leaving_claim_running() {
    let fixture = Arc::new(Fixture {
        cancel_during_delete: true,
        ..Default::default()
    });
    let messages = run(fixture.clone(), 32).await;
    assert_eq!(
        *fixture.calls.lock().unwrap(),
        ["info", "claim", "delete:Service", "finish:failed"]
    );
    assert!(
        messages
            .last()
            .unwrap()
            .error_message
            .as_deref()
            .unwrap()
            .contains("canceled")
    );
}

#[tokio::test]
async fn rejected_claim_never_mutates_docker_or_persists_completion() {
    let fixture = Arc::new(Fixture {
        reject_claim: true,
        ..Default::default()
    });
    let messages = run(fixture.clone(), 32).await;
    assert_eq!(*fixture.calls.lock().unwrap(), ["info", "claim"]);
    assert!(messages.last().unwrap().is_completed);
    assert!(messages.last().unwrap().error_message.is_some());
}

#[tokio::test]
async fn persistence_failure_is_not_reported_as_success() {
    let messages = run(
        Arc::new(Fixture {
            fail_finish: true,
            ..Default::default()
        }),
        32,
    )
    .await;
    assert!(
        messages
            .last()
            .unwrap()
            .error_message
            .as_deref()
            .unwrap()
            .contains("persist")
    );
}

#[tokio::test]
async fn terminal_progress_is_not_dropped_when_channel_is_full() {
    let messages = run(Arc::new(Fixture::default()), 1).await;
    assert!(messages.last().unwrap().is_completed);
    assert_eq!(messages.last().unwrap().stage, "completed");
}

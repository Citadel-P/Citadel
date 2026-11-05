use super::*;
use crate::{
    CoreIdentity, InspectedRelease, ReleaseArtifactSource, ReleaseInspectionRuntime, UpdateError,
};
use crate::{INSTALLER_FORMAT, UPDATE_PROTOCOL, UpgradeDescriptor, VerifiedReleaseArtifact};
use futures_util::future::BoxFuture;
use std::sync::Arc;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

struct Ports {
    calls: Mutex<Vec<&'static str>>,
    reject_signature: bool,
    artifact_version: String,
    descriptor: UpgradeDescriptor,
    cancel_after_signature: bool,
    cancel_during_inspection: bool,
}

impl Ports {
    fn new() -> Self {
        Self {
            calls: Mutex::new(vec![]),
            reject_signature: false,
            artifact_version: "1.2.3".into(),
            cancel_after_signature: false,
            cancel_during_inspection: false,
            descriptor: UpgradeDescriptor {
                schema_version: 1,
                product: "citadel".into(),
                target_version: "1.2.3".into(),
                target_catalog_sha256: "a".repeat(64),
                installer_format: INSTALLER_FORMAT,
                update_protocol: UPDATE_PROTOCOL,
                sources: vec![],
            },
        }
    }
}

impl ReleaseArtifactSource for Ports {
    fn verify<'a>(
        &'a self,
        _: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<VerifiedReleaseArtifact, UpdateError>> {
        Box::pin(async move {
            self.calls.lock().unwrap().push("verify");
            if self.reject_signature {
                return Err(UpdateError::Unavailable("signature".into()));
            }
            if self.cancel_after_signature {
                cancellation.cancel();
            }
            Ok(VerifiedReleaseArtifact {
                version: self.artifact_version.clone(),
                image: format!("ghcr.io/citadel-p/citadel@sha256:{}", "b".repeat(64)),
            })
        })
    }
}

impl ReleaseInspectionRuntime for Ports {
    fn inspect<'a>(
        &'a self,
        _: &'a VerifiedReleaseArtifact,
        _: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<UpgradeDescriptor, UpdateError>> {
        Box::pin(async move {
            self.calls.lock().unwrap().push("inspect");
            if self.cancel_during_inspection {
                cancellation.cancel();
                tokio::task::yield_now().await;
            }
            self.calls.lock().unwrap().push("cleanup");
            Ok(self.descriptor.clone())
        })
    }
}

async fn inspect(ports: Arc<Ports>) -> Result<InspectedRelease, UpdateError> {
    ReleaseInspectionService::new(ports.clone(), ports)
        .inspect("1.2.3", "amd64", &CancellationToken::new())
        .await
}

#[tokio::test]
async fn verifies_before_execution_and_preserves_empty_eligibility() {
    let ports = Arc::new(Ports::new());
    let inspected = inspect(ports.clone()).await.unwrap();
    assert_eq!(
        *ports.calls.lock().unwrap(),
        ["verify", "inspect", "cleanup"]
    );
    assert_eq!(inspected.identity.version, "1.2.3");
    assert_eq!(inspected.identity.architecture, "amd64");
    assert!(inspected.descriptor.sources.is_empty());
    let source = CoreIdentity {
        version: "1.2.2".into(),
        ..inspected.identity.clone()
    };
    assert!(
        inspected
            .descriptor
            .validate_transition(&source, &inspected.identity, &[])
            .is_err()
    );
}

#[tokio::test]
async fn refuses_unverified_mismatched_or_cancelled_artifacts_before_execution() {
    for case in 0..3 {
        let mut ports = Ports::new();
        match case {
            0 => ports.reject_signature = true,
            1 => ports.artifact_version = "1.2.4".into(),
            _ => ports.cancel_after_signature = true,
        }
        let ports = Arc::new(ports);
        assert!(inspect(ports.clone()).await.is_err());
        assert_eq!(*ports.calls.lock().unwrap(), ["verify"]);
    }
}

#[tokio::test]
async fn validates_request_before_network_and_runtime() {
    let ports = Arc::new(Ports::new());
    let service = ReleaseInspectionService::new(ports.clone(), ports.clone());
    for (version, architecture) in [
        ("latest", "amd64"),
        ("1.2.3", "other"),
        ("1.2.3+dirty", "amd64"),
    ] {
        assert!(
            service
                .inspect(version, architecture, &CancellationToken::new())
                .await
                .is_err()
        );
    }
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert!(matches!(
        service.inspect("1.2.3", "amd64", &cancellation).await,
        Err(UpdateError::Cancelled)
    ));
    assert!(ports.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cancellation_waits_for_runtime_cleanup() {
    let mut ports = Ports::new();
    ports.cancel_during_inspection = true;
    let ports = Arc::new(ports);
    assert!(matches!(
        inspect(ports.clone()).await,
        Err(UpdateError::Cancelled)
    ));
    assert_eq!(
        *ports.calls.lock().unwrap(),
        ["verify", "inspect", "cleanup"]
    );
}

#[tokio::test]
async fn rejects_inconsistent_or_unsupported_embedded_descriptors() {
    for case in 0..6 {
        let mut ports = Ports::new();
        match case {
            0 => ports.descriptor.target_version = "1.2.4".into(),
            1 => ports.descriptor.product = "other".into(),
            2 => ports.descriptor.target_catalog_sha256 = "invalid".into(),
            3 => ports.descriptor.schema_version += 1,
            4 => ports.descriptor.installer_format += 1,
            _ => ports.descriptor.update_protocol += 1,
        }
        assert!(inspect(Arc::new(ports)).await.is_err());
    }
}

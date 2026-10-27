//! A data item retains its host budget between helper commands and reconnects.
use super::{Runtime, docker_error};
use citadel_contracts::citadel::images::v1::{
    BuildAdmissionResult as Admission, BuildCleanupState as Cleanup, DataOperationRequest,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tonic::Status;
use uuid::Uuid;

pub(super) const OPERATION: &str = "citadel.data-operation";
pub(super) const AUTHORITY: &str = "citadel.data-authority";
#[derive(Clone)]
pub(super) struct DataAdmission(Arc<Mutex<State>>);
struct State {
    limit: usize,
    heavy: Arc<Semaphore>,
    operations: HashMap<Uuid, Operation>,
}
struct Operation {
    request: DataOperationRequest,
    helpers: Vec<(String, bool)>,
    active: usize,
    closing: bool,
    _heavy: OwnedSemaphorePermit,
}
pub(super) struct CommandGuard {
    ledger: DataAdmission,
    id: Uuid,
    helper: Option<String>,
}
impl CommandGuard {
    /// A definitive create response proves the daemon finished processing it.
    pub(super) fn created(&self) {
        if let Some(name) = &self.helper
            && let Ok(mut state) = self.ledger.0.lock()
            && let Some(operation) = state.operations.get_mut(&self.id)
            && let Some((_, settled)) = operation
                .helpers
                .iter_mut()
                .find(|(helper, _)| helper == name)
        {
            *settled = true;
        }
    }
}
impl Drop for CommandGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.ledger.0.lock()
            && let Some(operation) = state.operations.get_mut(&self.id)
        {
            operation.active = operation.active.saturating_sub(1);
        }
    }
}
fn id(value: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(value)
        .ok()
        .filter(|id| !id.is_nil())
        .ok_or_else(|| Status::invalid_argument("Data operation identity is required"))
}
impl DataAdmission {
    pub(super) fn new(limit: usize, heavy: Arc<Semaphore>) -> Self {
        Self(Arc::new(Mutex::new(State {
            limit,
            heavy,
            operations: HashMap::new(),
        })))
    }
    pub(super) fn capacity(&self) -> (usize, usize) {
        self.0
            .lock()
            .map(|s| (s.limit, s.operations.len()))
            .unwrap_or((0, 0))
    }
    pub(super) fn reserve(&self, request: &DataOperationRequest) -> Result<Admission, Status> {
        let operation_id = id(&request.operation_id)?;
        id(&request.run_id)?;
        id(&request.attempt_id)?;
        let mut state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Data admission unavailable"))?;
        if state.operations.contains_key(&operation_id) {
            return Err(Status::already_exists("Data operation is already admitted"));
        }
        if state.operations.len() >= state.limit {
            return Ok(Admission::BuildAdmissionCapacityBusyNotStarted);
        }
        let Ok(heavy) = state.heavy.clone().try_acquire_owned() else {
            return Ok(Admission::BuildAdmissionCapacityBusyNotStarted);
        };
        state.operations.insert(
            operation_id,
            Operation {
                request: request.clone(),
                helpers: vec![],
                active: 0,
                closing: false,
                _heavy: heavy,
            },
        );
        Ok(Admission::BuildAdmissionAccepted)
    }
    pub(super) fn command(
        &self,
        operation: &str,
        authority: &str,
        helper: Option<&str>,
    ) -> Result<CommandGuard, Status> {
        let id = id(operation)?;
        let mut state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Data admission unavailable"))?;
        let entry = state
            .operations
            .get_mut(&id)
            .filter(|entry| !entry.closing && entry.request.admission_authority_id == authority)
            .ok_or_else(|| Status::failed_precondition("Data operation is not admitted"))?;
        if let Some(name) = helper {
            if entry.helpers.len() >= 16
                || entry.helpers.iter().any(|(existing, _)| existing == name)
            {
                return Err(Status::failed_precondition(
                    "Data helper was already submitted or helper limit reached",
                ));
            }
            entry.helpers.push((name.into(), false)); // before Docker may observe the create
        }
        entry.active += 1;
        Ok(CommandGuard {
            ledger: self.clone(),
            id,
            helper: helper.map(str::to_owned),
        })
    }
    fn cleanup(
        &self,
        request: &DataOperationRequest,
    ) -> Result<Option<Vec<(String, bool)>>, Status> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Data admission unavailable"))?;
        let Some(entry) = state.operations.get_mut(&id(&request.operation_id)?) else {
            return Ok(Some(vec![]));
        };
        if entry.request != *request {
            return Err(Status::failed_precondition(
                "Data operation identity changed",
            ));
        }
        if entry.active > 0 {
            return Ok(None);
        }
        entry.closing = true;
        Ok(Some(entry.helpers.clone()))
    }
    fn complete(&self, request: &DataOperationRequest) -> Result<(), Status> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Data admission unavailable"))?;
        let operation_id = id(&request.operation_id)?;
        if state
            .operations
            .get(&operation_id)
            .is_some_and(|entry| entry.request != *request)
        {
            return Err(Status::failed_precondition(
                "Data operation identity changed",
            ));
        }
        state.operations.remove(&operation_id);
        Ok(())
    }
}
impl Runtime {
    pub(super) async fn validate_data_host(
        &self,
        request: &DataOperationRequest,
    ) -> Result<(), Status> {
        if request.admission_authority_id != self.builds.authority()?
            || request.admission_generation == 0
            || request.docker_daemon_id != self.docker.info().await.map_err(docker_error)?.id
        {
            return Err(Status::failed_precondition(
                "Data admission host identity changed",
            ));
        }
        Ok(())
    }
    pub(super) async fn data_command(
        &self,
        container: &str,
    ) -> Result<Option<CommandGuard>, Status> {
        let document = self
            .docker
            .inspect_container_document(container)
            .await
            .map_err(docker_error)?;
        let labels = &document["Config"]["Labels"];
        match labels[OPERATION].as_str() {
            Some(operation) => self
                .data
                .command(
                    operation,
                    labels[AUTHORITY].as_str().unwrap_or_default(),
                    None,
                )
                .map(Some),
            None => Ok(None),
        }
    }
    pub(super) async fn reconcile_data_operation(
        &self,
        request: DataOperationRequest,
    ) -> Result<Cleanup, Status> {
        self.validate_data_host(&request).await?;
        self.builds.fence_generation(request.admission_generation)?;
        let Some(helpers) = self.data.cleanup(&request)? else {
            return Ok(Cleanup::BuildCleanupActive);
        };
        for (name, create_settled) in helpers {
            let document = match self.docker.inspect_container_document(&name).await {
                Ok(document) => document,
                Err(citadel_docker::client::DockerError::Api { status, .. })
                    if status.as_u16() == 404 =>
                {
                    // A timed-out create can still arrive at Docker later. An
                    // absent name is proof only after a definitive create response.
                    if !create_settled {
                        return Ok(Cleanup::BuildCleanupUncertain);
                    }
                    continue;
                }
                Err(_) => return Ok(Cleanup::BuildCleanupUncertain),
            };
            if document["Config"]["Labels"][OPERATION].as_str()
                != Some(request.operation_id.as_str())
                || document["Config"]["Labels"][AUTHORITY].as_str()
                    != Some(request.admission_authority_id.as_str())
            {
                return Ok(Cleanup::BuildCleanupUncertain);
            }
            let Some(id) = document["Id"].as_str() else {
                return Ok(Cleanup::BuildCleanupUncertain);
            };
            match self
                .docker
                .delete_container_with_options(id, true, true, false)
                .await
            {
                Ok(()) => {}
                Err(citadel_docker::client::DockerError::Api { status, .. })
                    if status.as_u16() == 404 => {}
                Err(_) => return Ok(Cleanup::BuildCleanupUncertain),
            }
        }
        self.data.complete(&request)?;
        Ok(Cleanup::BuildCleanupConfirmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> DataOperationRequest {
        DataOperationRequest {
            run_id: Uuid::now_v7().to_string(),
            attempt_id: Uuid::now_v7().to_string(),
            operation_id: Uuid::now_v7().to_string(),
            admission_authority_id: Uuid::now_v7().to_string(),
            docker_daemon_id: "daemon".into(),
            admission_generation: 1,
        }
    }
    #[test]
    fn data_reservation_holds_shared_capacity_across_helpers_and_command_drop() {
        let heavy = Arc::new(Semaphore::new(2));
        let data = DataAdmission::new(2, heavy.clone());
        let builds = super::super::build_admission::BuildAdmission::with_heavy(2, heavy.clone());
        let first = request();
        let second = request();
        assert_eq!(
            data.reserve(&first).unwrap(),
            Admission::BuildAdmissionAccepted
        );
        assert_eq!(
            data.reserve(&second).unwrap(),
            Admission::BuildAdmissionAccepted
        );
        assert_eq!(
            data.reserve(&request()).unwrap(),
            Admission::BuildAdmissionCapacityBusyNotStarted
        );
        assert!(matches!(
            builds
                .build(
                    &Uuid::now_v7().to_string(),
                    &Uuid::now_v7().to_string(),
                    vec![],
                    &builds.authority().unwrap(),
                    1
                )
                .unwrap(),
            Err(super::super::build_admission::AdmissionRefusal::CapacityBusy)
        ));
        let command = data
            .command(
                &first.operation_id,
                &first.admission_authority_id,
                Some("owned-helper"),
            )
            .unwrap();
        assert!(data.cleanup(&first).unwrap().is_none());
        drop(command);
        assert_eq!(
            heavy.available_permits(),
            0,
            "dropping a command is not cleanup proof"
        );
        assert_eq!(
            data.cleanup(&first).unwrap().unwrap(),
            [("owned-helper".into(), false)]
        );
        assert!(
            data.command(&first.operation_id, &first.admission_authority_id, None)
                .is_err(),
            "closing fences late execution"
        );
        data.complete(&first).unwrap();
        assert_eq!(heavy.available_permits(), 1);
        assert!(
            builds
                .build(
                    &Uuid::now_v7().to_string(),
                    &Uuid::now_v7().to_string(),
                    vec![],
                    &builds.authority().unwrap(),
                    1
                )
                .unwrap()
                .is_ok()
        );
    }
    #[test]
    fn duplicate_admission_or_changed_identity_never_grants_replay() {
        let data = DataAdmission::new(2, Arc::new(Semaphore::new(2)));
        let original = request();
        data.reserve(&original).unwrap();
        assert_eq!(
            data.reserve(&original).unwrap_err().code(),
            tonic::Code::AlreadyExists
        );
        let mut stale = original.clone();
        stale.attempt_id = Uuid::now_v7().to_string();
        assert!(data.cleanup(&stale).is_err());
        assert!(data.complete(&stale).is_err());
        assert!(
            data.command(&original.operation_id, "other-authority", None)
                .is_err()
        );
        assert_eq!(data.capacity(), (2, 1));
    }

    #[test]
    fn reconciliation_fences_concurrent_data_admission() {
        for _ in 0..100 {
            let heavy = Arc::new(Semaphore::new(2));
            let data = DataAdmission::new(2, heavy.clone());
            let builds = super::super::build_admission::BuildAdmission::with_heavy(2, heavy);
            let mut input = request();
            input.admission_authority_id = builds.authority().unwrap();
            let barrier = Arc::new(std::sync::Barrier::new(2));
            std::thread::scope(|scope| {
                scope.spawn(|| {
                    barrier.wait();
                    let result = builds.reserve_data(&data, &input).unwrap();
                    assert!(matches!(
                        result,
                        Admission::BuildAdmissionAccepted
                            | Admission::BuildAdmissionFencedNotStarted
                    ));
                });
                barrier.wait();
                builds.fence_generation(input.admission_generation).unwrap();
                assert!(data.cleanup(&input).unwrap().unwrap().is_empty());
                data.complete(&input).unwrap();
            });
            assert_eq!(
                data.capacity().1,
                0,
                "cleanup cannot be followed by a late admission"
            );
        }
    }
}

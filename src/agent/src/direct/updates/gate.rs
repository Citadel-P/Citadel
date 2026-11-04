use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use tonic::Status;

#[derive(Clone, Default)]
pub(crate) struct Gate {
    closed: Arc<AtomicBool>,
    owner: Arc<std::sync::Mutex<Option<(uuid::Uuid, uuid::Uuid)>>>,
    active: Arc<RwLock<()>>,
}
pub(super) struct Drain {
    gate: Gate,
    _guard: Option<OwnedRwLockWriteGuard<()>>,
    retained: bool,
    token: uuid::Uuid,
}
impl Drain {
    pub(super) fn retain(&mut self) -> Result<(), Status> {
        let owner = self.gate.owner.lock().expect("update owner");
        if owner.as_ref().is_none_or(|(_, token)| *token != self.token) {
            return Err(Status::cancelled("Agent drain was released"));
        }
        self.retained = true;
        Ok(())
    }
}
impl Drop for Drain {
    fn drop(&mut self) {
        if !self.retained {
            let mut owner = self.gate.owner.lock().expect("update owner");
            if owner
                .as_ref()
                .is_some_and(|(_, token)| *token == self.token)
            {
                *owner = None;
                self.gate.closed.store(false, Ordering::Release);
            }
        }
    }
}
impl Gate {
    pub(super) fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
    pub(crate) fn enter(&self) -> Result<OwnedRwLockReadGuard<()>, Status> {
        let _owner = self.owner.lock().expect("update owner");
        if self.closed.load(Ordering::Acquire) {
            return Err(Status::unavailable("Agent update is draining work"));
        }
        self.active
            .clone()
            .try_read_owned()
            .map_err(|_| Status::unavailable("Agent update is draining work"))
    }
    pub(super) async fn drain(&self, operation: uuid::Uuid) -> Result<Drain, Status> {
        let token = uuid::Uuid::now_v7();
        {
            let mut owner = self.owner.lock().expect("update owner");
            if self.closed.swap(true, Ordering::AcqRel) {
                return Err(Status::already_exists("An Agent update is already active"));
            }
            *owner = Some((operation, token));
        }
        let mut drain = Drain {
            gate: self.clone(),
            _guard: None,
            retained: false,
            token,
        };
        drain._guard = Some(
            tokio::time::timeout(
                std::time::Duration::from_secs(120),
                self.active.clone().write_owned(),
            )
            .await
            .map_err(|_| {
                Status::deadline_exceeded("Agent work did not drain before the update deadline")
            })?,
        );
        Ok(drain)
    }
    pub(super) fn resume(&self, operation: uuid::Uuid) -> Result<(), Status> {
        let mut owner = self.owner.lock().expect("update owner");
        if owner.is_some_and(|(id, _)| id != operation) {
            return Err(Status::failed_precondition(
                "A different Agent update owns admission",
            ));
        }
        *owner = None;
        self.closed.store(false, Ordering::Release);
        Ok(())
    }
}

pub(crate) fn observes(method: &str) -> bool {
    matches!(
        method.rsplit('/').next(),
        Some(
            "SetAgentUpdateDrain"
                | "GetAgentUpdateInfo"
                | "UpdateAgent"
                | "CheckHealth"
                | "GetPlatformInfo"
                | "StreamPlatformStats"
                | "StreamDaemonEvent"
                | "List"
                | "Inspect"
                | "Logs"
                | "StreamStats"
                | "StreamContainersStats"
                | "Processes"
                // These calls release already-admitted work, with identity checks in the handlers.
                | "ReconcileData"
                | "ReconcileBuild"
                | "Reconcile"
        )
    )
}

pub(crate) async fn admission(
    axum::extract::State(gate): axum::extract::State<Gate>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let permit = if observes(request.uri().path()) {
        None
    } else {
        match gate.enter() {
            Ok(permit) => Some(permit),
            Err(error) => return error.into_http(),
        }
    };
    let response = next.run(request).await;
    let (parts, body) = response.into_parts();
    axum::response::Response::from_parts(
        parts,
        axum::body::Body::from_stream(async_stream::stream! {
            let _permit=permit;
            let mut body=body.into_data_stream();
            use futures_util::StreamExt;
            while let Some(chunk)=body.next().await { yield chunk; }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn drain_waits_for_existing_work_and_timeout_reopens() {
        let gate = Gate::default();
        let active = gate.enter().unwrap();
        let result = gate.drain(uuid::Uuid::now_v7()).await;
        assert!(result.is_err());
        assert!(!gate.is_closed());
        drop(active);
        let drain = gate.drain(uuid::Uuid::now_v7()).await.unwrap();
        assert!(gate.enter().is_err());
        drop(drain);
        assert!(gate.enter().is_ok());
    }
    #[tokio::test]
    async fn released_drain_cannot_close_or_reopen_a_new_owner() {
        let gate = Gate::default();
        let id = uuid::Uuid::now_v7();
        let mut first = gate.drain(id).await.unwrap();
        gate.resume(id).unwrap();
        assert!(first.retain().is_err());
        drop(first);
        let mut second = gate.drain(uuid::Uuid::now_v7()).await.unwrap();
        second.retain().unwrap();
        drop(second);
        assert!(gate.resume(id).is_err());
        assert!(gate.enter().is_err());
    }
    #[tokio::test]
    async fn handoff_keeps_admission_closed_after_rpc_returns() {
        let gate = Gate::default();
        let mut drain = gate.drain(uuid::Uuid::now_v7()).await.unwrap();
        drain.retain().unwrap();
        drop(drain);
        assert!(gate.enter().is_err());
    }
}

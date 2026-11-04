use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use tonic::Status;

#[derive(Clone, Default)]
pub(crate) struct Gate {
    closed: Arc<AtomicBool>,
    owner: Arc<std::sync::Mutex<Option<Owner>>>,
    active: Arc<RwLock<()>>,
}
struct Owner {
    operation: uuid::Uuid,
    token: uuid::Uuid,
    failed_preparation: bool,
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
        if owner
            .as_ref()
            .is_none_or(|owner| owner.token != self.token || owner.failed_preparation)
        {
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
                .is_some_and(|owner| owner.token == self.token)
            {
                // Keep one fenced failure receipt until the next operation. A lost
                // response can be settled, but this operation cannot be dispatched again.
                owner.as_mut().expect("matching owner").failed_preparation = true;
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
            if owner
                .as_ref()
                .is_some_and(|owner| owner.operation == operation)
                || self.closed.swap(true, Ordering::AcqRel)
            {
                return Err(Status::already_exists(
                    "Agent update operation is already active or settled",
                ));
            }
            *owner = Some(Owner {
                operation,
                token,
                failed_preparation: false,
            });
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
    pub(super) fn settle_preparation(&self, operation: uuid::Uuid) -> Result<(), Status> {
        let owner = self.owner.lock().expect("update owner");
        if owner
            .as_ref()
            .is_some_and(|owner| owner.operation == operation && owner.failed_preparation)
        {
            return Ok(());
        }
        Err(Status::failed_precondition(
            "Agent update recovery required: no completed pre-handoff failure is recorded for this operation",
        ))
    }
    pub(super) fn resume(&self, operation: uuid::Uuid) -> Result<(), Status> {
        let mut owner = self.owner.lock().expect("update owner");
        if owner
            .as_ref()
            .is_some_and(|owner| owner.operation != operation)
        {
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
                | "StreamContainerLogs"
                | "StreamContainerStats"
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
    if permit.is_none() {
        return response;
    }
    let (parts, mut body) = response.into_parts();
    axum::response::Response::from_parts(
        parts,
        axum::body::Body::new(http_body_util::StreamBody::new(async_stream::stream! {
            let _permit = permit;
            use http_body_util::BodyExt;
            // gRPC completes with trailers, including its success/error status.
            while let Some(frame) = body.frame().await {
                yield frame;
            }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use tonic::codegen::Service;

    #[tokio::test]
    async fn admission_preserves_grpc_trailers_and_holds_work_until_body_finishes() {
        for status in ["0", "13"] {
            let gate = Gate::default();
            let mut app = axum::Router::new()
                .route(
                    "/mutate",
                    axum::routing::post(move || async move {
                        let mut trailers = axum::http::HeaderMap::new();
                        trailers.insert("grpc-status", status.parse().unwrap());
                        let frames = futures_util::stream::iter([
                            Ok::<_, std::convert::Infallible>(http_body::Frame::data(
                                axum::body::Bytes::from_static(b"response"),
                            )),
                            Ok(http_body::Frame::trailers(trailers)),
                        ]);
                        axum::body::Body::new(http_body_util::StreamBody::new(frames))
                    }),
                )
                .layer(axum::middleware::from_fn_with_state(
                    gate.clone(),
                    admission,
                ));
            let response = app
                .call(
                    axum::http::Request::post("/mutate")
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let mut body = response.into_body();
            assert!(gate.active.clone().try_write_owned().is_err());
            assert_eq!(
                body.frame().await.unwrap().unwrap().into_data().unwrap(),
                "response"
            );
            let trailers = body
                .frame()
                .await
                .expect("gRPC status trailers")
                .unwrap()
                .into_trailers()
                .unwrap();
            assert_eq!(trailers["grpc-status"], status);
            assert!(gate.active.clone().try_write_owned().is_err());
            assert!(body.frame().await.is_none());
            assert!(gate.active.clone().try_write_owned().is_ok());
        }
    }

    #[tokio::test]
    async fn passive_streams_do_not_hold_drain_but_exec_holds_it_until_disconnect() {
        for method in ["StreamContainerLogs", "StreamContainerStats", "Exec"] {
            let gate = Gate::default();
            let mut app = axum::Router::new()
                .route(
                    "/{*method}",
                    axum::routing::post(|| async {
                        axum::body::Body::from_stream(futures_util::stream::pending::<
                            Result<axum::body::Bytes, std::io::Error>,
                        >())
                    }),
                )
                .layer(axum::middleware::from_fn_with_state(
                    gate.clone(),
                    admission,
                ));
            let response = app
                .call(
                    axum::http::Request::post(format!("/{method}"))
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let draining = gate.clone();
            let task = tokio::spawn(async move { draining.drain(uuid::Uuid::now_v7()).await });
            while !gate.is_closed() {
                tokio::task::yield_now().await;
            }
            if method == "Exec" {
                assert!(!task.is_finished());
                drop(response);
                task.await.unwrap().unwrap();
            } else {
                let drain = task.await.unwrap().unwrap();
                assert!(gate.enter().is_err());
                drop(response);
                drop(drain);
            }
        }
    }

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
    #[tokio::test(start_paused = true)]
    async fn only_completed_preparation_failure_can_settle_and_cannot_replay() {
        let gate = Gate::default();
        let operation = uuid::Uuid::now_v7();
        let active = gate.enter().unwrap();
        assert!(gate.drain(operation).await.is_err());
        assert!(gate.settle_preparation(operation).is_ok());
        assert!(gate.settle_preparation(operation).is_ok());
        assert!(gate.drain(operation).await.is_err());
        drop(active);
        let next = uuid::Uuid::now_v7();
        let mut drain = gate.drain(next).await.unwrap();
        assert!(gate.settle_preparation(operation).is_err());
        assert!(gate.settle_preparation(next).is_err());
        drain.retain().unwrap();
        drop(drain);
        assert!(gate.settle_preparation(next).is_err());
        assert!(gate.enter().is_err());
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

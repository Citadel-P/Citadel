use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use citadel_contracts::citadel::edge::v1::{
    CancelCommand, CoreEnvelope, EdgeCommand, EdgeCommandKind, core_envelope,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use tonic::{Code, Status};
use uuid::Uuid;

pub const MAX_PAYLOAD: usize = 16 * 1024 * 1024;
const MAX_COMMANDS: usize = 16;
const MAX_STREAMS: usize = 8;
const OUTBOUND_CAPACITY: usize = 32;
// Account for queue nodes and permits even when a frame has an empty payload.
const OUTPUT_FRAME_OVERHEAD: usize = 256;

#[derive(Clone, Debug, thiserror::Error)]
#[error("{message}")]
pub struct EdgeError {
    code: Code,
    message: &'static str,
}

impl EdgeError {
    fn new(code: Code, message: &'static str) -> Self {
        Self { code, message }
    }

    fn remote(code: &str) -> Self {
        // Preserve error types without exposing remote messages that may contain
        // resolved credentials or other sensitive command input.
        let (code, message) = match code {
            "cancelled" => (Code::Cancelled, "Edge command canceled."),
            "deadline_exceeded" => (Code::DeadlineExceeded, "Edge command timed out."),
            "resource_exhausted" => (
                Code::ResourceExhausted,
                "Edge command resource limit exceeded.",
            ),
            "invalid_argument" => (Code::InvalidArgument, "Invalid Edge command request."),
            "permission_denied" => (Code::PermissionDenied, "Edge command permission denied."),
            "unauthenticated" => (Code::Unauthenticated, "Edge command authentication failed."),
            "not_found" => (Code::NotFound, "The requested Edge resource was not found."),
            "already_exists" => (Code::AlreadyExists, "The Edge resource already exists."),
            "aborted" => (
                Code::Aborted,
                "Edge command conflicted with another operation.",
            ),
            "failed_precondition" => (
                Code::FailedPrecondition,
                "Edge command precondition failed.",
            ),
            "out_of_range" => (Code::OutOfRange, "Edge command argument is out of range."),
            "unavailable" => (Code::Unavailable, "Edge runtime is unavailable."),
            "unimplemented" => (Code::Unimplemented, "Edge command is not supported."),
            "data_loss" => (Code::DataLoss, "Edge command returned invalid data."),
            _ => (Code::Unknown, "Edge command failed."),
        };
        Self::new(code, message)
    }

    pub(crate) fn status(&self) -> Status {
        Status::new(self.code, self.message)
    }

    pub(crate) fn runtime(self) -> citadel_platforms::RuntimeCapabilityError {
        if self.code == Code::Cancelled {
            return citadel_platforms::RuntimeCapabilityError::new(
                citadel_platforms::RuntimeErrorKind::Cancelled,
                self.message,
                false,
            );
        }
        crate::connectors::agent::client::normalize_status(self.status())
    }
}

pub use citadel_platforms::edge_management::EdgeTarget;

#[derive(Clone)]
pub struct EdgeRegistry {
    sessions: Arc<Mutex<HashMap<EdgeTarget, Arc<EdgeSession>>>>,
    // Across all sessions, not a per-node multiplier. Slow consumers fail
    // explicitly instead of retaining an unbounded log/backup output backlog.
    output_budget: Arc<Semaphore>,
}
impl Default for EdgeRegistry {
    fn default() -> Self {
        Self {
            sessions: Arc::default(),
            output_budget: Arc::new(Semaphore::new(32 * 1024 * 1024)),
        }
    }
}
impl EdgeRegistry {
    pub fn disconnect_platform(&self, id: Uuid) {
        self.sessions
            .lock()
            .expect("Edge registry lock poisoned")
            .retain(|target, session| {
                if target.resource_type == 0 && target.platform_id == id {
                    session.close();
                    false
                } else {
                    true
                }
            });
    }

    pub fn current_sessions(&self) -> Vec<Arc<EdgeSession>> {
        self.sessions
            .lock()
            .expect("Edge registry lock poisoned")
            .values()
            .filter(|session| !session.is_closed())
            .cloned()
            .collect()
    }
    pub fn register(
        &self,
        target: EdgeTarget,
        agent_id: Uuid,
    ) -> Result<(Arc<EdgeSession>, mpsc::Receiver<CoreEnvelope>), EdgeError> {
        let mut sessions = self.sessions.lock().expect("Edge registry lock poisoned");
        if let Some(previous) = sessions.get(&target)
            && target.node_id.is_some()
            && previous.agent_id != agent_id
            && !previous.is_closed()
        {
            return Err(EdgeError::new(
                Code::AlreadyExists,
                "Node already has an authenticated Agent session.",
            ));
        }
        let (outbound, receiver) = mpsc::channel(OUTBOUND_CAPACITY);
        let connected_micros = sessions.get(&target).map_or_else(
            || chrono::Utc::now().timestamp_micros(),
            |previous| {
                chrono::Utc::now()
                    .timestamp_micros()
                    .max(previous.connected_at.timestamp_micros() + 1)
            },
        );
        let session = Arc::new(EdgeSession {
            target: target.clone(),
            agent_id,
            id: Uuid::now_v7(),
            connected_at: chrono::DateTime::from_timestamp_micros(connected_micros)
                .expect("current UTC timestamp"),
            outbound,
            closed: CancellationToken::new(),
            pending: Mutex::new(HashMap::new()),
            output_budget: self.output_budget.clone(),
        });
        if let Some(previous) = sessions.insert(target, session.clone()) {
            previous.close();
        }
        Ok((session, receiver))
    }
    pub fn get(&self, target: &EdgeTarget) -> Result<Arc<EdgeSession>, EdgeError> {
        self.sessions
            .lock()
            .expect("Edge registry lock poisoned")
            .get(target)
            .filter(|session| !session.is_closed())
            .cloned()
            .ok_or(EdgeError::new(
                Code::Unavailable,
                "The selected Edge Agent is disconnected or unavailable.",
            ))
    }
    pub fn remove(&self, session: &Arc<EdgeSession>) -> bool {
        let mut sessions = self.sessions.lock().expect("Edge registry lock poisoned");
        if sessions
            .get(&session.target)
            .is_some_and(|current| Arc::ptr_eq(current, session))
        {
            sessions.remove(&session.target);
            session.close();
            true
        } else {
            false
        }
    }
    pub fn disconnect(&self, target: &EdgeTarget) {
        if let Some(session) = self
            .sessions
            .lock()
            .expect("Edge registry lock poisoned")
            .remove(target)
        {
            session.close();
        }
    }
}

struct QueuedOutput {
    payload: Vec<u8>,
    _permit: OwnedSemaphorePermit,
}
struct Pending {
    output: mpsc::UnboundedSender<QueuedOutput>,
    failed: CancellationToken,
    failure: Arc<Mutex<Option<EdgeError>>>,
    streaming: bool,
    interactive: bool,
    _request_budget: OwnedSemaphorePermit,
}

pub struct EdgeSession {
    pub target: EdgeTarget,
    pub agent_id: Uuid,
    pub id: Uuid,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    outbound: mpsc::Sender<CoreEnvelope>,
    closed: CancellationToken,
    pending: Mutex<HashMap<Uuid, Pending>>,
    output_budget: Arc<Semaphore>,
}
impl EdgeSession {
    /// Buffered observations must not outlive the authenticated session.
    pub fn observation_token(&self) -> CancellationToken {
        self.closed.clone()
    }

    pub fn is_closed(&self) -> bool {
        self.closed.is_cancelled()
    }
    pub async fn closed(&self) {
        self.closed.cancelled().await;
    }
    pub fn close(&self) {
        self.closed.cancel();
        for (_, pending) in self
            .pending
            .lock()
            .expect("Edge pending lock poisoned")
            .drain()
        {
            pending.failed.cancel();
        }
    }
    pub fn pending_count(&self) -> usize {
        self.pending
            .lock()
            .expect("Edge pending lock poisoned")
            .len()
    }

    pub fn command(
        self: &Arc<Self>,
        kind: EdgeCommandKind,
        payload: Vec<u8>,
        timeout: Duration,
        streaming: bool,
    ) -> Result<EdgeCommandStream, EdgeError> {
        if payload.len() > MAX_PAYLOAD {
            return Err(EdgeError::new(
                Code::ResourceExhausted,
                "Edge command exceeds maximum payload size.",
            ));
        }
        if timeout.is_zero() || timeout > Duration::from_secs(24 * 60 * 60) {
            return Err(EdgeError::new(
                Code::InvalidArgument,
                "Edge command timeout is invalid.",
            ));
        }
        if self.target.node_id.is_some() && !node_command_allowed(kind) {
            return Err(EdgeError::new(
                Code::PermissionDenied,
                "Command is not allowed for a Swarm Node Agent.",
            ));
        }
        let mut pending = self.pending.lock().expect("Edge pending lock poisoned");
        if self.is_closed() {
            return Err(EdgeError::new(
                Code::Unavailable,
                "Edge session disconnected.",
            ));
        }
        if pending.len() >= MAX_COMMANDS
            || (streaming && pending.values().filter(|p| p.streaming).count() >= MAX_STREAMS)
        {
            return Err(EdgeError::new(
                Code::ResourceExhausted,
                "Edge session concurrent command limit reached.",
            ));
        }
        let id = Uuid::now_v7();
        let request_budget = self
            .output_budget
            .clone()
            .try_acquire_many_owned(payload.len().max(1) as u32)
            .map_err(|_| {
                EdgeError::new(
                    Code::ResourceExhausted,
                    "Edge queued payload byte budget is exhausted.",
                )
            })?;
        // The shared byte budget bounds this queue. A frame-count limit rejects
        // normal log history bursts regardless of how little memory they use.
        let (output, receiver) = mpsc::unbounded_channel();
        let failed = CancellationToken::new();
        let failure = Arc::new(Mutex::new(None));
        let envelope = self.envelope(
            id,
            core_envelope::Body::Command(EdgeCommand {
                command_id: id.to_string(),
                platform_id: self.target.platform_id.to_string(),
                kind: kind as i32,
                payload,
                payload_schema_version: 1,
                timeout_ms: timeout.as_millis().min(i32::MAX as u128) as i32,
                correlation_id: id.to_string(),
                expects_stream: streaming,
                resource_type: self.target.resource_type,
                resource_id: self.target.resource_id.to_string(),
                node_id: self.target.node_id.clone().unwrap_or_default(),
            }),
        );
        pending.insert(
            id,
            Pending {
                output,
                failed: failed.clone(),
                failure: failure.clone(),
                streaming,
                interactive: kind == EdgeCommandKind::ContainerExec,
                _request_budget: request_budget,
            },
        );
        if self.outbound.try_send(envelope).is_err() {
            pending.remove(&id);
            return Err(EdgeError::new(
                Code::Unavailable,
                "Edge outbound queue is unavailable.",
            ));
        }
        Ok(EdgeCommandStream {
            session: self.clone(),
            id,
            receiver,
            failed,
            failure,
            deadline: tokio::time::Instant::now() + timeout,
            finished: false,
        })
    }
    fn envelope(&self, id: Uuid, body: core_envelope::Body) -> CoreEnvelope {
        CoreEnvelope {
            envelope_id: Uuid::now_v7().to_string(),
            session_id: self.id.to_string(),
            command_id: id.to_string(),
            body: Some(body),
        }
    }
    fn cancel(&self, id: Uuid) {
        let removed = self
            .pending
            .lock()
            .expect("Edge pending lock poisoned")
            .remove(&id);
        if let Some(pending) = removed {
            pending.failed.cancel();
            if self
                .outbound
                .try_send(self.envelope(
                    id,
                    core_envelope::Body::CancelCommand(CancelCommand {
                        reason: "Core stopped waiting for this command.".into(),
                    }),
                ))
                .is_err()
            {
                // A dropped cancel must not leave remote mutations running while
                // their local concurrency slots are reused.
                self.close();
            }
        }
    }
    pub fn output(&self, id: Uuid, payload: Vec<u8>) {
        let delivered = {
            let pending = self.pending.lock().expect("Edge pending lock poisoned");
            let Some(command) = pending.get(&id) else {
                return;
            };
            payload.len() <= MAX_PAYLOAD
                && self
                    .output_budget
                    .clone()
                    .try_acquire_many_owned((payload.len() + OUTPUT_FRAME_OVERHEAD) as u32)
                    .is_ok_and(|permit| {
                        command
                            .output
                            .send(QueuedOutput {
                                payload,
                                _permit: permit,
                            })
                            .is_ok()
                    })
        };
        if !delivered {
            self.cancel(id);
        }
    }
    pub fn complete(&self, id: Uuid, succeeded: bool) {
        self.finish(
            id,
            (!succeeded).then(|| EdgeError::remote("command_failed")),
        );
    }
    pub fn fail(&self, id: Uuid, code: &str) {
        self.finish(id, Some(EdgeError::remote(code)));
    }
    fn finish(&self, id: Uuid, error: Option<EdgeError>) {
        if let Some(pending) = self
            .pending
            .lock()
            .expect("Edge pending lock poisoned")
            .remove(&id)
            && let Some(error) = error
        {
            *pending.failure.lock().expect("Edge failure lock poisoned") = Some(error);
            pending.failed.cancel();
        }
    }
}

pub struct EdgeCommandStream {
    session: Arc<EdgeSession>,
    id: Uuid,
    receiver: mpsc::UnboundedReceiver<QueuedOutput>,
    failed: CancellationToken,
    failure: Arc<Mutex<Option<EdgeError>>>,
    deadline: tokio::time::Instant,
    finished: bool,
}
impl EdgeCommandStream {
    pub(crate) fn send_input(&self, payload: Vec<u8>) -> Result<(), EdgeError> {
        // Bound protobuf overhead as well as stdin. No user-supplied command/session IDs.
        if payload.len() > citadel_platforms::terminal::MAX_TERMINAL_INPUT + 32 {
            return Err(EdgeError::new(
                Code::ResourceExhausted,
                "Terminal input exceeds the limit.",
            ));
        }
        let pending = self
            .session
            .pending
            .lock()
            .expect("Edge pending lock poisoned");
        if self.finished
            || self.failed.is_cancelled()
            || self.session.is_closed()
            || tokio::time::Instant::now() >= self.deadline
            || !pending.get(&self.id).is_some_and(|p| p.interactive)
        {
            return Err(EdgeError::new(
                Code::Unavailable,
                "Interactive command is no longer active.",
            ));
        }
        self.session
            .outbound
            .try_send(self.session.envelope(
                self.id,
                core_envelope::Body::StreamInput(
                    citadel_contracts::citadel::edge::v1::StreamInput { payload },
                ),
            ))
            .map_err(|_| {
                EdgeError::new(
                    Code::Unavailable,
                    "Edge terminal input queue is full or disconnected.",
                )
            })
    }
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub async fn next(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<Option<Vec<u8>>, EdgeError> {
        if self.finished {
            return Ok(None);
        }
        let result = tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(EdgeError::new(Code::Cancelled, "Edge command canceled.")),
            () = self.session.closed() => Err(EdgeError::new(Code::Unavailable, "Edge session disconnected.")),
            () = self.failed.cancelled() => Err(self.failure.lock().expect("Edge failure lock poisoned").clone()
                .unwrap_or_else(|| EdgeError::new(Code::ResourceExhausted, "Edge command output limit was exceeded."))),
            () = tokio::time::sleep_until(self.deadline) => Err(EdgeError::new(Code::DeadlineExceeded, "Edge command timed out.")),
            item = self.receiver.recv() => Ok(item.map(|output| output.payload)),
        };
        if !matches!(result, Ok(Some(_))) {
            self.finished = true;
            self.session.cancel(self.id);
            self.receiver.close();
            while self.receiver.try_recv().is_ok() {}
        }
        result
    }
}
impl Drop for EdgeCommandStream {
    fn drop(&mut self) {
        if !self.finished {
            self.session.cancel(self.id);
        }
    }
}

fn node_command_allowed(kind: EdgeCommandKind) -> bool {
    use EdgeCommandKind::*;
    matches!(
        kind,
        PlatformCheckHealth
            | PlatformGetInfo
            | PlatformStatsStream
            | PlatformDaemonEventsStream
            | ContainerList
            | ContainerLogsStream
            | ContainerInspect
            | ContainerCreate
            | ContainerStart
            | ContainerStop
            | ContainerPause
            | ContainerUnpause
            | ContainerRestart
            | ContainerDelete
            | ContainerStatsStream
            | ContainersStatsStream
            | ContainerExec
            | ContainerExecBinary
            | ImageGet
            | ImageList
            | ImageInspect
            | ImageExposedPorts
            | VolumeList
            | VolumeInspect
            | VolumeCreate
            | VolumeDelete
            | NetworkList
            | NetworkInspect
    )
}

impl citadel_platforms::edge_management::EdgeSessionControl for EdgeRegistry {
    fn disconnect(&self, target: &EdgeTarget) {
        self.disconnect(target);
    }
    fn disconnect_platform(&self, id: Uuid) {
        self.disconnect_platform(id);
    }
}

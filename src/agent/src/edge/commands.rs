//! Session-owned command futures, bounded input and exactly one terminal envelope.
use super::{
    AgentBody, EdgeCommand, EdgeCommandKind, MAX_ENVELOPE, StreamInput, dispatch, envelope,
    identity::{Identity, target_id},
    outgoing::Outgoing,
};
use crate::{config::EdgeProfile, direct::Runtime};
use citadel_adapters::connectors::docker::DockerClient;
use citadel_contracts::citadel::{
    containers::v1::ExecClientMessage,
    edge::v1::{CommandCompleted, CommandFailed, CommandOutput},
};
use futures_util::{StreamExt, future::BoxFuture, stream::FuturesUnordered};
use prost::Message;
use std::{collections::HashMap, io, sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use tonic::Status;
use uuid::Uuid;

const MAX_COMMANDS: usize = 16;
const INPUT_CAPACITY: usize = 256;
const INPUT_BYTES: usize = 2 * MAX_ENVELOPE;

struct Input {
    message: ExecClientMessage,
    _budget: OwnedSemaphorePermit,
}
struct Active {
    cancel: CancellationToken,
    input: Option<mpsc::Sender<Input>>,
}
pub(super) struct Commands {
    active: HashMap<Uuid, Active>,
    futures: FuturesUnordered<BoxFuture<'static, (Uuid, io::Result<()>)>>,
    cancel: CancellationToken,
    budget: Arc<Semaphore>,
    docker: DockerClient,
    runtime_container: Option<String>,
    identity: Identity,
    profile: EdgeProfile,
    session: String,
    outgoing: Outgoing,
}
impl Drop for Commands {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
impl Commands {
    pub fn new(
        docker: DockerClient,
        runtime_container: Option<String>,
        identity: Identity,
        profile: EdgeProfile,
        session: String,
        outgoing: Outgoing,
    ) -> Self {
        Self {
            active: HashMap::new(),
            futures: FuturesUnordered::new(),
            cancel: CancellationToken::new(),
            budget: Arc::new(Semaphore::new(INPUT_BYTES)),
            docker,
            runtime_container,
            identity,
            profile,
            session,
            outgoing,
        }
    }
    pub fn running(&self) -> bool {
        !self.futures.is_empty()
    }
    pub async fn next(&mut self) -> io::Result<()> {
        if let Some((id, result)) = self.futures.next().await {
            self.active.remove(&id);
            result?;
        }
        Ok(())
    }
    fn output(&self, command: &str) -> Output {
        Output {
            outgoing: self.outgoing.clone(),
            session: self.session.clone(),
            command: command.into(),
        }
    }
    pub async fn start(
        &mut self,
        envelope_command_id: &str,
        command: EdgeCommand,
    ) -> io::Result<()> {
        let id = Uuid::parse_str(&command.command_id)
            .ok()
            .filter(|id| !id.is_nil());
        if id.is_none() || envelope_command_id != command.command_id {
            return Err(io::Error::other(
                "Invalid or conflicting Edge command identifiers",
            ));
        }
        let id = id.expect("validated command identifier");
        // A duplicate must not complete or replace the original command under the same ID.
        if self.active.contains_key(&id) {
            return Err(io::Error::other("Duplicate active Edge command identifier"));
        }
        let output = self.output(&command.command_id);
        let validated = self.validate(&command);
        let kind = match validated {
            Ok(kind) => kind,
            Err(error) => return output.finish(Err(error)).await,
        };
        let permit = match self
            .budget
            .clone()
            .try_acquire_many_owned(command.payload.len().max(1) as u32)
        {
            Ok(permit) => permit,
            Err(_) => {
                return output
                    .finish(Err(Status::resource_exhausted(
                        "Edge input byte budget exhausted",
                    )))
                    .await;
            }
        };
        let cancel = self.cancel.child_token();
        let guard = cancel.clone().drop_guard();
        let runtime = Runtime::new(
            self.docker.clone(),
            cancel.clone(),
            self.runtime_container.clone(),
        );
        let (sender, receiver) = mpsc::channel::<Input>(INPUT_CAPACITY);
        let input = futures_util::stream::unfold(receiver, |mut receiver| async move {
            receiver
                .recv()
                .await
                .map(|item| (Ok(item.message), receiver))
        });
        self.active.insert(
            id,
            Active {
                cancel: cancel.clone(),
                input: (kind == EdgeCommandKind::ContainerExec).then_some(sender),
            },
        );
        let deadline = (command.timeout_ms > 0).then(|| {
            tokio::time::Instant::now() + Duration::from_millis(command.timeout_ms as u64)
        });
        let profile = self.profile.clone();
        let docker = self.docker.clone();
        self.futures.push(Box::pin(async move {
            let _guard = guard;
            let _permit = permit;
            let result = tokio::select! {
                biased;
                () = cancel.cancelled() => Err(Status::cancelled("Edge command cancelled")),
                () = async { if let Some(deadline) = deadline { tokio::time::sleep_until(deadline).await } else { std::future::pending::<()>().await } } => Err(Status::deadline_exceeded("Edge command timed out")),
                result = async {
                    let mut payload = command.payload;
                    if let EdgeProfile::SwarmNode(node) = &profile {
                        super::swarm_guard::authorize(&docker, node, kind, &mut payload).await?;
                    }
                    dispatch::execute(runtime, kind, &payload, Box::pin(input), output.clone()).await
                } => result,
            };
            // Drop operation/streams first, then publish the single terminal outcome.
            cancel.cancel();
            (id, output.finish(result).await)
        }));
        Ok(())
    }
    fn validate(&self, command: &EdgeCommand) -> Result<EdgeCommandKind, Status> {
        let kind = EdgeCommandKind::try_from(command.kind)
            .map_err(|_| Status::invalid_argument("Unknown Edge command kind"))?;
        if kind == EdgeCommandKind::Unspecified || command.payload_schema_version != 1 {
            return Err(Status::invalid_argument(
                "Unsupported Edge command kind or payload schema",
            ));
        }
        if command.payload.len() > MAX_ENVELOPE {
            return Err(Status::resource_exhausted(
                "Edge command payload exceeds 16 MiB",
            ));
        }
        if Uuid::parse_str(&command.platform_id).ok() != Some(self.identity.platform_id)
            || command.resource_type != i32::from(self.identity.resource_type == "BuildAgentPool")
            || Uuid::parse_str(if command.resource_id.is_empty() {
                &command.platform_id
            } else {
                &command.resource_id
            })
            .ok()
                != Some(target_id(&self.identity))
        {
            return Err(Status::permission_denied(
                "Edge command targets a different resource",
            ));
        }
        match &self.profile {
            EdgeProfile::SwarmNode(node)
                if command.node_id != node.node_id || !super::swarm_policy::allowed(kind) =>
            {
                return Err(Status::permission_denied(
                    "Command is not allowed for this Swarm node",
                ));
            }
            EdgeProfile::SwarmNode(_) => {}
            _ if !command.node_id.is_empty() => {
                return Err(Status::permission_denied(
                    "Node command sent to an ordinary Edge Agent",
                ));
            }
            _ => {}
        }
        if self.active.len() >= MAX_COMMANDS {
            return Err(Status::resource_exhausted(
                "Edge concurrent command limit reached",
            ));
        }
        Ok(kind)
    }
    pub fn input(&mut self, id: &str, input: StreamInput) {
        let Some(active) = Uuid::parse_str(id)
            .ok()
            .and_then(|id| self.active.get_mut(&id))
        else {
            return;
        };
        let queue = || {
            if input.payload.len() > MAX_ENVELOPE {
                return None;
            }
            let sender = active.input.as_ref()?;
            let budget = self
                .budget
                .clone()
                .try_acquire_many_owned(input.payload.len().max(1) as u32)
                .ok()?;
            let message = dispatch::decode(&input.payload).ok()?;
            sender
                .try_send(Input {
                    message,
                    _budget: budget,
                })
                .ok()
        };
        if queue().is_none() {
            active.input.take();
            active.cancel.cancel();
        }
    }
    pub fn cancel(&mut self, id: &str) {
        if let Some(active) = Uuid::parse_str(id)
            .ok()
            .and_then(|id| self.active.get_mut(&id))
        {
            active.input.take();
            active.cancel.cancel();
        }
    }
}

#[derive(Clone)]
pub(super) struct Output {
    pub outgoing: Outgoing,
    pub session: String,
    pub command: String,
}
impl Output {
    pub async fn message(&self, value: impl Message) -> Result<(), Status> {
        if value.encoded_len() > MAX_ENVELOPE {
            return Err(Status::resource_exhausted(
                "Edge command output exceeds 16 MiB",
            ));
        }
        self.outgoing
            .send(envelope(
                &self.session,
                &self.command,
                AgentBody::CommandOutput(CommandOutput {
                    payload: value.encode_to_vec(),
                }),
            ))
            .await
            .map_err(|error| {
                if error.kind() == io::ErrorKind::InvalidInput {
                    Status::resource_exhausted("Edge output envelope exceeds 16 MiB")
                } else {
                    Status::unavailable("Edge output stream unavailable")
                }
            })
    }
    async fn finish(&self, result: Result<(), Status>) -> io::Result<()> {
        let body = match result {
            Ok(()) => AgentBody::CommandCompleted(CommandCompleted { status_code: 0 }),
            Err(error) => AgentBody::CommandFailed(CommandFailed {
                code: match error.code() {
                    tonic::Code::Cancelled => "cancelled",
                    tonic::Code::DeadlineExceeded => "deadline_exceeded",
                    tonic::Code::ResourceExhausted => "resource_exhausted",
                    tonic::Code::InvalidArgument => "invalid_argument",
                    tonic::Code::PermissionDenied => "permission_denied",
                    tonic::Code::Unauthenticated => "unauthenticated",
                    tonic::Code::NotFound => "not_found",
                    tonic::Code::AlreadyExists => "already_exists",
                    tonic::Code::Aborted => "aborted",
                    tonic::Code::FailedPrecondition => "failed_precondition",
                    tonic::Code::OutOfRange => "out_of_range",
                    tonic::Code::Unavailable => "unavailable",
                    tonic::Code::Unimplemented => "unimplemented",
                    tonic::Code::DataLoss => "data_loss",
                    _ => "command_failed",
                }
                .into(),
                message: error.message().into(),
            }),
        };
        self.outgoing
            .send(envelope(&self.session, &self.command, body))
            .await
    }
}

#[cfg(test)]
mod tests;

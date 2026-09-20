use std::{pin::Pin, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use citadel_contracts::citadel::edge::v1::{
    AgentEnvelope, AuthChallenge, CoreEnvelope, SessionAccepted, SessionRejected, agent_envelope,
    core_envelope, edge_agent_service_server::EdgeAgentService,
};
use ed25519_dalek::Signature;
use futures_util::Stream;
use tokio::sync::Semaphore;
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;

use super::{EdgeBinding, EdgeRegistry, EdgeSession, PostgresEdgeStore};

#[derive(Clone)]
pub struct EdgeIntake {
    store: PostgresEdgeStore,
    registry: EdgeRegistry,
    connections: Arc<Semaphore>,
}
impl EdgeIntake {
    pub fn new(store: PostgresEdgeStore, registry: EdgeRegistry) -> Self {
        Self {
            store,
            registry,
            connections: Arc::new(Semaphore::new(512)),
        }
    }
}

#[tonic::async_trait]
impl EdgeAgentService for EdgeIntake {
    type ConnectStream = Pin<Box<dyn Stream<Item = Result<CoreEnvelope, Status>> + Send>>;
    async fn connect(
        &self,
        request: Request<Streaming<AgentEnvelope>>,
    ) -> Result<Response<Self::ConnectStream>, Status> {
        let permit = self
            .connections
            .clone()
            .try_acquire_owned()
            .map_err(|_| Status::resource_exhausted("Edge connection limit reached."))?;
        let mut inbound = request.into_inner();
        let store = self.store.clone();
        let registry = self.registry.clone();
        let stream = async_stream::try_stream! {
            let _permit = permit;
            let first = read_handshake(&mut inbound).await?;
            let authentication = match first.body {
                Some(agent_envelope::Body::EnrollmentRequest(request)) => {
                    tokio::time::timeout(Duration::from_secs(15), store.enroll(&request)).await
                        .map_err(|_| Status::deadline_exceeded("Enrollment timed out."))?
                }
                Some(agent_envelope::Body::Hello(hello)) => {
                    match tokio::time::timeout(Duration::from_secs(15), store.reconnect(&hello)).await
                        .map_err(|_| Status::deadline_exceeded("Authentication timed out."))? {
                        Ok(binding) => {
                            let mut nonce = [0u8; 32];
                            getrandom::fill(&mut nonce).map_err(|_| Status::internal("Challenge generation failed."))?;
                            let timestamp = Utc::now().timestamp();
                            yield envelope(core_envelope::Body::AuthChallenge(AuthChallenge { nonce: nonce.to_vec(), timestamp_unix_seconds: timestamp }));
                            let response = read_handshake(&mut inbound).await?;
                            let verified = match response.body {
                                Some(agent_envelope::Body::AuthChallengeResponse(response)) => {
                                    let mut payload = [0u8; 40];
                                    payload[..8].copy_from_slice(&timestamp.to_le_bytes());
                                    payload[8..].copy_from_slice(&nonce);
                                    response.nonce == nonce && response.timestamp_unix_seconds == timestamp
                                        && Signature::from_slice(&response.signature).is_ok_and(|signature| binding.public_key.verify_strict(&payload, &signature).is_ok())
                                }
                                _ => false,
                            };
                            if verified { Ok(binding) } else { Err(super::EdgeStoreError::Unauthorized) }
                        }
                        Err(error) => Err(error),
                    }
                }
                _ => Err(super::EdgeStoreError::Unauthorized),
            };
            let binding = match authentication {
                Ok(binding) => binding,
                Err(error) => {
                    if let super::EdgeStoreError::Storage(source) = error { tracing::warn!(%source, "Edge authentication persistence failed"); }
                    yield envelope(core_envelope::Body::SessionRejected(SessionRejected { reason: "Edge Agent authentication or enrollment was rejected.".into() }));
                    return;
                }
            };
            let (session, mut outbound) = registry.register(binding.target.clone(), binding.agent_id).map_err(|error| Status::already_exists(error.to_string()))?;
            let at = session.connected_at;
            let _lease = SessionLease { registry, session: session.clone(), store: store.clone(), at };
            // Recheck revocation after registering, closing the authenticate /
            // revoke race. Revocation commits before disconnecting the registry.
            store.connected(&binding, at).await.map_err(|_| Status::unauthenticated("Edge binding is no longer active."))?;
            yield CoreEnvelope {
                envelope_id: Uuid::now_v7().to_string(), session_id: session.id.to_string(), command_id: String::new(),
                body: Some(core_envelope::Body::SessionAccepted(SessionAccepted {
                    platform_id: binding.target.platform_id.to_string(), agent_id: binding.agent_id.to_string(), session_id: session.id.to_string(),
                    resource_type: binding.target.resource_type, resource_id: binding.target.resource_id.to_string(), node_id: binding.target.node_id.clone().unwrap_or_default(),
                })),
            };
            let mut heartbeat_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
            loop {
                enum Event { Closed, Incoming(Result<Option<AgentEnvelope>, Status>), Outgoing(Option<CoreEnvelope>) }
                let event = tokio::select! {
                    biased;
                    () = session.closed() => Event::Closed,
                    () = tokio::time::sleep_until(heartbeat_deadline) => Event::Closed,
                    incoming = inbound.message() => Event::Incoming(incoming),
                    outgoing = outbound.recv() => Event::Outgoing(outgoing),
                };
                match event {
                    Event::Closed => break,
                    Event::Incoming(incoming) => {
                        let Some(incoming) = incoming? else { break; };
                        if incoming.session_id != session.id.to_string() { Err(Status::permission_denied("Edge session identity mismatch."))?; }
                        match incoming.body {
                            Some(agent_envelope::Body::Heartbeat(heartbeat)) => {
                                if !heartbeat_identity_matches(&binding, &heartbeat) { Err(Status::permission_denied("Edge Docker identity changed."))?; }
                                store.heartbeat(binding.agent_id, at).await.map_err(|_| Status::unauthenticated("Edge binding is no longer active."))?;
                                heartbeat_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
                            }
                            Some(agent_envelope::Body::CommandOutput(output)) => {
                                if let Ok(id) = Uuid::parse_str(&incoming.command_id) { session.output(id, output.payload); }
                            }
                            Some(agent_envelope::Body::CommandCompleted(completed)) => {
                                if let Ok(id) = Uuid::parse_str(&incoming.command_id) { session.complete(id, completed.status_code == 0); }
                            }
                            Some(agent_envelope::Body::CommandFailed(_)) => {
                                if let Ok(id) = Uuid::parse_str(&incoming.command_id) { session.complete(id, false); }
                            }
                            _ => Err(Status::invalid_argument("Unexpected Edge message after authentication."))?,
                        }
                    }
                    Event::Outgoing(outgoing) => { match outgoing { Some(value) => yield value, None => break } }
                }
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }
}

fn envelope(body: core_envelope::Body) -> CoreEnvelope {
    CoreEnvelope {
        envelope_id: Uuid::now_v7().to_string(),
        body: Some(body),
        ..Default::default()
    }
}
async fn read_handshake(stream: &mut Streaming<AgentEnvelope>) -> Result<AgentEnvelope, Status> {
    tokio::time::timeout(Duration::from_secs(15), stream.message())
        .await
        .map_err(|_| Status::deadline_exceeded("Edge authentication timed out."))??
        .ok_or_else(|| Status::unauthenticated("Edge authentication ended early."))
}
fn heartbeat_identity_matches(
    binding: &EdgeBinding,
    heartbeat: &citadel_contracts::citadel::edge::v1::AgentHeartbeat,
) -> bool {
    heartbeat.daemon_id == binding.daemon_id
        && (binding.target.node_id.is_none()
            || (Some(heartbeat.node_id.as_str()) == binding.target.node_id.as_deref()
                && Some(heartbeat.cluster_id.as_str()) == binding.cluster_id.as_deref()))
}

struct SessionLease {
    registry: EdgeRegistry,
    session: Arc<EdgeSession>,
    store: PostgresEdgeStore,
    at: DateTime<Utc>,
}
impl Drop for SessionLease {
    fn drop(&mut self) {
        if self.registry.remove(&self.session) {
            let store = self.store.clone();
            let id = self.session.agent_id;
            let at = self.at;
            tokio::spawn(async move {
                let _ =
                    tokio::time::timeout(Duration::from_secs(3), store.disconnected(id, at)).await;
            });
        }
    }
}

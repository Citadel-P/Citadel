//! Outbound Edge connection, durable identity and session-owned command execution.
mod commands;
mod dispatch;
mod identity;
mod observation;
mod outgoing;
mod swarm_guard;
mod swarm_policy;
#[cfg(test)]
mod tests;

use crate::config::{EdgeConfig, EdgeProfile};
use citadel_adapters::connectors::docker::DockerClient;
use citadel_contracts::citadel::edge::v1::{
    agent_envelope::Body as AgentBody, core_envelope::Body as CoreBody,
    edge_agent_service_client::EdgeAgentServiceClient, *,
};
use commands::Commands;
use ed25519_dalek::Signer;
use identity::{State, invalid};
use observation::Observation;
use outgoing::Outgoing;
use std::{io, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tonic::transport::{Certificate, ClientTlsConfig, Endpoint};
use uuid::Uuid;

const MAX_ENVELOPE: usize = 16 * 1024 * 1024;
const OUTGOING_CAPACITY: usize = 512;
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) struct EdgeAgent {
    config: EdgeConfig,
    state: State,
    endpoint: Endpoint,
    docker: DockerClient,
    heartbeat_interval: Duration,
    runtime_container: Option<String>,
}
impl EdgeAgent {
    pub async fn new(config: EdgeConfig, docker: DockerClient) -> io::Result<Self> {
        let mut endpoint = Endpoint::from_shared(config.core_url.to_string())
            .map_err(|_| invalid("Invalid Core endpoint"))?
            .connect_timeout(Duration::from_secs(10))
            .http2_keep_alive_interval(Duration::from_secs(30))
            .keep_alive_timeout(Duration::from_secs(10))
            .keep_alive_while_idle(true);
        if config.core_url.scheme() == "https" {
            let mut tls = ClientTlsConfig::new().with_native_roots();
            if let Some(path) = &config.core_ca_path {
                let pem = tokio::fs::read(path)
                    .await
                    .map_err(|_| invalid("Could not read CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH"))?;
                use rustls::pki_types::{CertificateDer, pem::PemObject};
                let certificates = CertificateDer::pem_slice_iter(&pem)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| invalid("Invalid Core CA certificate PEM"))?;
                if certificates.is_empty() {
                    return Err(invalid("Core CA file contains no certificates"));
                }
                let mut roots = rustls::RootCertStore::empty();
                for cert in certificates {
                    roots
                        .add(cert)
                        .map_err(|_| invalid("Invalid Core CA certificate"))?;
                }
                tls = tls.ca_certificate(Certificate::from_pem(pem));
            }
            endpoint = endpoint
                .tls_config(tls)
                .map_err(|_| invalid("Could not configure Core TLS"))?;
        }
        let state = State::load(&config)?;
        Ok(Self {
            config,
            state,
            endpoint,
            docker,
            heartbeat_interval: HEARTBEAT_INTERVAL,
            runtime_container: None,
        })
    }

    pub fn with_runtime_container(mut self, runtime_container: Option<String>) -> Self {
        self.runtime_container = runtime_container;
        self
    }

    pub async fn run(mut self, stop: CancellationToken) -> io::Result<()> {
        if self.config.core_url.scheme() == "http" {
            tracing::warn!("Edge Core connection uses HTTP without TLS");
        }
        let mut backoff = Duration::from_secs(1);
        loop {
            let mut accepted = false;
            let result = tokio::select! {
                biased;
                ()=stop.cancelled()=>return Ok(()),
                result=self.connection(&mut accepted)=>result,
            };
            match result {
                // Durable-state and protocol errors need intervention; retrying cannot repair them.
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::InvalidData | io::ErrorKind::PermissionDenied
                    ) =>
                {
                    return Err(error);
                }
                Err(error) => tracing::warn!(%error, "Edge connection ended; reconnecting"),
                Ok(()) => tracing::info!("Edge session disconnected; reconnecting"),
            }
            if accepted {
                backoff = Duration::from_secs(1);
            }
            let delay = jitter(backoff);
            tokio::select! { ()=stop.cancelled()=>return Ok(()), ()=tokio::time::sleep(delay)=>{} }
            backoff = (backoff * 2).min(Duration::from_secs(60));
        }
    }

    async fn connection(&mut self, was_accepted: &mut bool) -> io::Result<()> {
        // Refresh negotiation and identity on every new connection, never reuse a replaced daemon.
        self.docker.invalidate_daemon().await;
        let initial = Observation::read(&self.docker, &self.config).await?;
        let first = self.opening(&initial)?;
        let channel = self
            .endpoint
            .connect()
            .await
            .map_err(|_| io::Error::other("Could not connect to Core"))?;
        let mut client = EdgeAgentServiceClient::new(channel)
            .max_decoding_message_size(MAX_ENVELOPE)
            .max_encoding_message_size(MAX_ENVELOPE);
        let (outgoing, receiver) = Outgoing::channel();
        outgoing.send(first).await?;
        let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
            receiver.recv().await.map(|value| (value.message, receiver))
        });
        let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
        let mut incoming = tokio::time::timeout_at(deadline, client.connect(stream))
            .await
            .map_err(|_| io::Error::other("Edge handshake timed out"))?
            .map_err(|_| io::Error::other("Core rejected the Edge connection"))?
            .into_inner();
        let mut session = String::new();
        let mut challenged = false;
        let mut commands: Option<Commands> = None;
        let mut heartbeat = tokio::time::interval(self.heartbeat_interval);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // A single owned heartbeat future keeps Docker I/O from blocking Core input.
        let mut probing = false;
        let probe = Observation::read(&self.docker, &self.config);
        tokio::pin!(probe);
        loop {
            enum Event {
                Message(Option<CoreEnvelope>),
                Tick,
                CommandFinished(io::Result<()>),
                Probe(io::Result<Observation>),
            }
            let event = tokio::select! {
                ()=tokio::time::sleep_until(deadline), if session.is_empty()=>return Err(io::Error::other("Edge handshake timed out")),
                value=async { commands.as_mut().expect("accepted session").next().await }, if commands.as_ref().is_some_and(Commands::running)=>Event::CommandFinished(value),
                value=incoming.message()=>Event::Message(value.map_err(|_|io::Error::other("Edge stream failed"))?),
                _=heartbeat.tick(), if !session.is_empty() && !probing=>Event::Tick,
                value=&mut probe, if probing=>Event::Probe(value),
            };
            match event {
                Event::CommandFinished(result) => result?,
                Event::Tick => {
                    probe.set(Observation::read(&self.docker, &self.config));
                    probing = true;
                }
                Event::Probe(value) => {
                    probing = false;
                    let observed = match value {
                        Ok(value)
                            if value.daemon_id != initial.daemon_id
                                || value.node_id != initial.node_id
                                || value.cluster_id != initial.cluster_id =>
                        {
                            return Err(io::Error::other(
                                "Docker identity changed during the Edge session",
                            ));
                        }
                        Ok(value) => value.heartbeat(&self.config, true),
                        Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                            return Err(error);
                        }
                        Err(_) => initial.heartbeat(&self.config, false),
                    };
                    send(
                        &outgoing,
                        envelope(&session, "", AgentBody::Heartbeat(observed)),
                    )
                    .await?;
                }
                Event::Message(None) => return Ok(()),
                Event::Message(Some(message)) => {
                    if !session.is_empty() && message.session_id != session {
                        return Err(invalid("Core message belongs to a different Edge session"));
                    }
                    match message.body {
                        Some(CoreBody::AuthChallenge(challenge))
                            if session.is_empty()
                                && self.state.identity.is_some()
                                && !challenged =>
                        {
                            if challenge.nonce.len() != 32 {
                                return Err(invalid("Invalid Edge authentication challenge"));
                            }
                            challenged = true;
                            let payload = [
                                &challenge.timestamp_unix_seconds.to_le_bytes()[..],
                                &challenge.nonce,
                            ]
                            .concat();
                            let signature = self.state.key.sign(&payload).to_bytes().to_vec();
                            send(
                                &outgoing,
                                envelope(
                                    "",
                                    "",
                                    AgentBody::AuthChallengeResponse(AuthChallengeResponse {
                                        nonce: challenge.nonce,
                                        timestamp_unix_seconds: challenge.timestamp_unix_seconds,
                                        signature,
                                    }),
                                ),
                            )
                            .await?;
                        }
                        Some(CoreBody::SessionAccepted(accepted)) if session.is_empty() => {
                            if self.state.identity.is_some() && !challenged {
                                return Err(invalid(
                                    "Core accepted reconnect without an authentication challenge",
                                ));
                            }
                            if message.session_id != accepted.session_id {
                                return Err(invalid(
                                    "Core accepted conflicting Edge session identifiers",
                                ));
                            }
                            self.state
                                .accept(&self.config, &accepted)
                                .map_err(|error| {
                                    io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        format!(
                                            "Could not validate or persist Edge identity: {error}"
                                        ),
                                    )
                                })?;
                            session = accepted.session_id;
                            commands = Some(Commands::new(
                                self.docker.clone(),
                                self.runtime_container.clone(),
                                self.state.identity.clone().expect("accepted identity"),
                                self.config.profile.clone(),
                                session.clone(),
                                outgoing.clone(),
                            ));
                            *was_accepted = true;
                            tracing::info!(%session, "Edge session accepted");
                        }
                        Some(CoreBody::SessionRejected(rejected)) if session.is_empty() => {
                            self.state
                                .rejected(&self.config, &rejected.reason)
                                .map_err(|error| {
                                    io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        format!("Could not reset rejected Edge identity: {error}"),
                                    )
                                })?;
                            return Ok(());
                        }
                        Some(CoreBody::Disconnect(_)) => return Ok(()),
                        Some(CoreBody::Command(command)) if !session.is_empty() => {
                            commands
                                .as_mut()
                                .expect("accepted session")
                                .start(&message.command_id, command)
                                .await?;
                        }
                        Some(CoreBody::StreamInput(input)) if !session.is_empty() => {
                            commands
                                .as_mut()
                                .expect("accepted session")
                                .input(&message.command_id, input);
                        }
                        Some(CoreBody::CancelCommand(_)) if !session.is_empty() => {
                            commands
                                .as_mut()
                                .expect("accepted session")
                                .cancel(&message.command_id);
                        }
                        _ => return Err(invalid("Unexpected Edge handshake or session message")),
                    }
                }
            }
        }
    }

    fn opening(&mut self, observed: &Observation) -> io::Result<AgentEnvelope> {
        let h = observed.heartbeat(&self.config, true);
        let profile = i32::from(matches!(self.config.profile, EdgeProfile::SwarmNode(_)));
        let body = if let Some(identity) = &self.state.identity {
            AgentBody::Hello(AgentHello {
                platform_id: identity.platform_id.to_string(),
                resource_type: i32::from(identity.resource_type == "BuildAgentPool"),
                resource_id: identity::target_id(identity).to_string(),
                agent_id: identity.agent_id.to_string(),
                agent_fingerprint: identity::fingerprint(self.state.key.verifying_key().as_bytes()),
                hostname: h.hostname,
                agent_version: h.agent_version,
                protocol_version: citadel_contracts::EDGE_AGENT_PROTOCOL_VERSION,
                capabilities_json: h.capabilities_json,
                daemon_id: h.daemon_id,
                profile,
                cluster_id: h.cluster_id,
                node_id: h.node_id,
                docker_hostname: h.docker_hostname,
                swarm_role: h.swarm_role,
                service_id: h.service_id,
                task_id: h.task_id,
            })
        } else {
            let token = identity::credential(&self.config)?
                .ok_or_else(|| invalid("Edge enrollment credential is missing"))?;
            self.state.token_fingerprint = Some(identity::fingerprint(token.as_bytes()));
            AgentBody::EnrollmentRequest(EnrollmentRequest {
                enrollment_token: token.to_string(),
                public_key: self.state.key.verifying_key().as_bytes().to_vec(),
                hostname: h.hostname,
                agent_version: h.agent_version,
                protocol_version: citadel_contracts::EDGE_AGENT_PROTOCOL_VERSION,
                capabilities_json: h.capabilities_json,
                daemon_id: h.daemon_id,
                profile,
                cluster_id: h.cluster_id,
                node_id: h.node_id,
                docker_hostname: h.docker_hostname,
                swarm_role: h.swarm_role,
                service_id: h.service_id,
                task_id: h.task_id,
            })
        };
        Ok(envelope("", "", body))
    }
}
fn envelope(session: &str, command: &str, body: AgentBody) -> AgentEnvelope {
    AgentEnvelope {
        envelope_id: Uuid::now_v7().to_string(),
        session_id: session.into(),
        command_id: command.into(),
        body: Some(body),
    }
}
async fn send(sender: &Outgoing, message: AgentEnvelope) -> io::Result<()> {
    sender.send(message).await
}

fn jitter(delay: Duration) -> Duration {
    let mut bytes = [0; 8];
    let factor = if getrandom::fill(&mut bytes).is_ok() {
        0.8 + 0.4 * (u64::from_le_bytes(bytes) as f64 / u64::MAX as f64)
    } else {
        1.0
    };
    delay.mul_f64(factor)
}
fn version() -> String {
    crate::VERSION.to_owned()
}

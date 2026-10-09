use super::*;
use citadel_contracts::citadel::edge::v1::{
    agent_envelope::Body as AgentBody,
    core_envelope::Body as CoreBody,
    edge_agent_service_server::{EdgeAgentService, EdgeAgentServiceServer},
    *,
};
use ed25519_dalek::{Signature, VerifyingKey};
use futures_util::stream::BoxStream;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tonic::{Response, Status, Streaming};

pub struct Core {
    pub address: String,
    sessions: mpsc::Receiver<Session>,
}
#[derive(Clone)]
struct Service {
    sessions: mpsc::Sender<Session>,
    target: SessionAccepted,
    key: Arc<Mutex<Option<[u8; 32]>>>,
}
pub struct Session {
    sender: mpsc::Sender<CoreEnvelope>,
    incoming: mpsc::Receiver<AgentEnvelope>,
    target: SessionAccepted,
}
fn envelope(session: &str, command: &str, body: CoreBody) -> CoreEnvelope {
    CoreEnvelope {
        envelope_id: Uuid::now_v7().to_string(),
        session_id: session.into(),
        command_id: command.into(),
        body: Some(body),
    }
}

#[tonic::async_trait]
impl EdgeAgentService for Service {
    type ConnectStream = BoxStream<'static, Result<CoreEnvelope, Status>>;
    async fn connect(
        &self,
        request: Request<Streaming<AgentEnvelope>>,
    ) -> Result<Response<Self::ConnectStream>, Status> {
        let this = self.clone();
        let mut incoming = request.into_inner();
        Ok(Response::new(Box::pin(async_stream::try_stream! {
            match incoming.message().await?.unwrap().body.unwrap() {
                AgentBody::EnrollmentRequest(value)=>{*this.key.lock().unwrap()=Some(value.public_key.try_into().unwrap());}
                AgentBody::Hello(value)=>{
                    assert_eq!(value.agent_id,this.target.agent_id);
                    let nonce=vec![42;32]; let time=chrono::Utc::now().timestamp();
                    yield envelope("","",CoreBody::AuthChallenge(AuthChallenge{nonce:nonce.clone(),timestamp_unix_seconds:time}));
                    let Some(AgentBody::AuthChallengeResponse(value))=incoming.message().await?.unwrap().body else {panic!("challenge response required")};
                    VerifyingKey::from_bytes(&this.key.lock().unwrap().unwrap()).unwrap().verify_strict(&[&time.to_le_bytes()[..],&nonce].concat(),&Signature::from_slice(&value.signature).unwrap()).unwrap();
                }
                value=>panic!("unexpected opening {value:?}"),
            }
            let target=SessionAccepted{session_id:Uuid::now_v7().to_string(),..this.target.clone()};
            yield envelope(&target.session_id,"",CoreBody::SessionAccepted(target.clone()));
            let (sender,mut outgoing)=mpsc::channel(4);
            let (received,inbox)=mpsc::channel(4);
            this.sessions.send(Session{sender,incoming:inbox,target}).await.unwrap();
            loop {
                enum Event { Outgoing(Option<CoreEnvelope>), Incoming(Result<Option<AgentEnvelope>,Status>) }
                let event=tokio::select! {
                    value=outgoing.recv()=>Event::Outgoing(value),
                    value=incoming.message()=>Event::Incoming(value),
                };
                match event {
                    Event::Outgoing(Some(value))=>yield value,
                    Event::Outgoing(None)=>break,
                    Event::Incoming(value)=>match value? {Some(value)=>{if received.send(value).await.is_err(){break;}},None=>break},
                }
            }
        })))
    }
}
impl Core {
    pub async fn start(stop: CancellationToken) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let (sender, sessions) = mpsc::channel(4);
        let id = Uuid::now_v7().to_string();
        let service = Service {
            sessions: sender,
            key: Arc::new(Mutex::new(None)),
            target: SessionAccepted {
                platform_id: id.clone(),
                resource_id: id,
                agent_id: Uuid::now_v7().to_string(),
                ..Default::default()
            },
        };
        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(
                    EdgeAgentServiceServer::new(service)
                        .max_decoding_message_size(LIMIT)
                        .max_encoding_message_size(LIMIT),
                )
                .serve_with_incoming_shutdown(
                    futures_util::stream::unfold(listener, |listener| async {
                        Some((listener.accept().await.map(|v| v.0), listener))
                    }),
                    stop.cancelled_owned(),
                )
                .await
                .unwrap();
        });
        Self { address, sessions }
    }
    pub async fn next(&mut self) -> Session {
        tokio::time::timeout(Duration::from_secs(15), self.sessions.recv())
            .await
            .unwrap()
            .expect("Edge must enroll or reconnect")
    }
}
impl Session {
    async fn start(&self, kind: EdgeCommandKind, request: impl Message) -> String {
        let id = Uuid::now_v7().to_string();
        let command = EdgeCommand {
            command_id: id.clone(),
            platform_id: self.target.platform_id.clone(),
            resource_id: self.target.resource_id.clone(),
            kind: kind as i32,
            payload: request.encode_to_vec(),
            payload_schema_version: 1,
            timeout_ms: 20_000,
            ..Default::default()
        };
        self.sender
            .send(envelope(
                &self.target.session_id,
                &id,
                CoreBody::Command(command),
            ))
            .await
            .unwrap();
        id
    }
    async fn receive(&mut self, id: &str) -> AgentBody {
        loop {
            let message = self
                .incoming
                .recv()
                .await
                .expect("Edge stream ended prematurely");
            if matches!(message.body, Some(AgentBody::Heartbeat(_))) {
                continue;
            }
            assert_eq!(message.command_id, id);
            return message.body.unwrap();
        }
    }
    pub async fn cycle(&mut self, round: usize) {
        for tty in [false, true] {
            let id = self
                .start(EdgeCommandKind::ContainerLogsStream, log_request(tty))
                .await;
            let mut bytes = 0;
            while bytes < MIB + 64 {
                let AgentBody::CommandOutput(value) = self.receive(&id).await else {
                    panic!("expected log output")
                };
                bytes += ContainerLogResponse::decode(value.payload.as_slice())
                    .unwrap()
                    .log
                    .len();
            }
            assert_eq!(bytes, MIB + 64);
            self.sender
                .send(envelope(
                    &self.target.session_id,
                    &id,
                    CoreBody::CancelCommand(CancelCommand {
                        reason: "memory test".into(),
                    }),
                ))
                .await
                .unwrap();
            let AgentBody::CommandFailed(value) = self.receive(&id).await else {
                panic!("cancellation must terminate the command")
            };
            assert_eq!(value.code, "cancelled");
        }
        let id = self
            .start(
                EdgeCommandKind::ImagePullStream,
                PullImageRequest {
                    from_image: "fixture".into(),
                    ..Default::default()
                },
            )
            .await;
        let mut frames = 0;
        loop {
            match self.receive(&id).await {
                AgentBody::CommandOutput(value) => {
                    assert_eq!(
                        PullImageResponse::decode(value.payload.as_slice())
                            .unwrap()
                            .status
                            .unwrap()
                            .len(),
                        256 * 1024
                    );
                    frames += 1;
                }
                AgentBody::CommandCompleted(_) => break,
                value => panic!("unexpected pull response {value:?}"),
            }
        }
        assert_eq!(frames, 4);
        let id = self
            .start(EdgeCommandKind::ImageBuildStream, build_request(round))
            .await;
        let mut completed = false;
        loop {
            match self.receive(&id).await {
                AgentBody::CommandOutput(value) => {
                    let value = ImageBuildResponse::decode(value.payload.as_slice()).unwrap();
                    assert!(value.error_message.is_none(), "{value:?}");
                    completed |= value.status.as_deref() == Some("completed");
                }
                AgentBody::CommandCompleted(_) => break,
                value => panic!("unexpected build response {value:?}"),
            }
        }
        assert!(completed);
    }
    pub async fn disconnect(mut self) {
        // Disconnect while Docker is still following logs, so the reconnect
        // check also verifies cleanup of unfinished session-owned commands.
        let id = self
            .start(EdgeCommandKind::ContainerLogsStream, log_request(false))
            .await;
        assert!(matches!(
            self.receive(&id).await,
            AgentBody::CommandOutput(_)
        ));
        self.sender
            .send(envelope(
                &self.target.session_id,
                "",
                CoreBody::Disconnect(Disconnect {
                    reason: "memory test reconnect".into(),
                }),
            ))
            .await
            .unwrap();
    }
}

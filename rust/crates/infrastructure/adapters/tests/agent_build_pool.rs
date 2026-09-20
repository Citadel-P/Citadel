use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use citadel_adapters::agent::{AgentClient, AgentRequestSigner};
use citadel_contracts::citadel::images::v1::image_service_server::{
    ImageService, ImageServiceServer,
};
use citadel_contracts::citadel::images::v1::*;
use citadel_contracts::citadel::shared_models::v1::{ImageReply, ListImageResponse};
use ed25519_dalek::{Signature, SigningKey, Verifier};
use futures_util::Stream;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};

#[derive(Clone)]
struct BuildHost {
    version: &'static str,
    calls: Arc<AtomicUsize>,
}

type Output<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[tonic::async_trait]
impl ImageService for BuildHost {
    async fn check_build_host(
        &self,
        request: Request<()>,
    ) -> Result<Response<CheckBuildHostResponse>, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let bytes = |name: &'static str| {
            request
                .metadata()
                .get_bin(name)
                .unwrap()
                .to_bytes()
                .unwrap()
        };
        let mut signed = bytes("x-timestamp-bin").to_vec();
        signed.extend_from_slice(&bytes("x-nonce-bin"));
        signed.extend_from_slice(b"/citadel.images.v1.ImageService/CheckBuildHost");
        signed.extend_from_slice(&Sha256::digest([]));
        SigningKey::from_bytes(&[31; 32])
            .verifying_key()
            .verify(
                &signed,
                &Signature::from_slice(&bytes("x-signature-bin")).unwrap(),
            )
            .map_err(|_| Status::unauthenticated("wrong identity"))?;
        assert!(request.metadata().get("grpc-timeout").is_some());
        Ok(Response::new(CheckBuildHostResponse {
            available: true,
            docker_version: self.version.into(),
            api_version: "1.49".into(),
            operating_system: "linux".into(),
            architecture: "amd64".into(),
            build_kit_version: String::new(),
        }))
    }
    async fn get(&self, _: Request<GetImageRequest>) -> Result<Response<ImageReply>, Status> {
        Err(Status::unimplemented("not used"))
    }
    async fn list(
        &self,
        _: Request<ListImagesRequest>,
    ) -> Result<Response<ListImageResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }
    async fn delete(
        &self,
        _: Request<DeleteImageRequest>,
    ) -> Result<Response<DeleteImageResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }
    async fn inspect(
        &self,
        _: Request<InspectImageRequest>,
    ) -> Result<Response<InspectImageResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }
    async fn history(
        &self,
        _: Request<HistoryImageRequest>,
    ) -> Result<Response<HistoryImageResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }
    async fn get_exposed_ports(
        &self,
        _: Request<GetExposedPortsRequest>,
    ) -> Result<Response<GetExposedPortsResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }
    async fn distribution_inspect(
        &self,
        _: Request<DistributionInspectRequest>,
    ) -> Result<Response<DistributionInspectResponse>, Status> {
        Err(Status::unimplemented("not used"))
    }
    type PullStream = Output<PullImageResponse>;
    async fn pull(
        &self,
        request: Request<PullImageRequest>,
    ) -> Result<Response<Self::PullStream>, Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(request.metadata().get_bin("x-signature-bin").is_some());
        let input = request.into_inner();
        if input.from_image == "fail:latest" {
            return Err(Status::unavailable("ambiguous pull failure"));
        }
        assert_eq!(input.from_image, "nginx:latest");
        assert_eq!(input.auth.as_deref(), Some("encoded-auth"));
        Ok(Response::new(Box::pin(async_stream::stream! {
            yield Ok(PullImageResponse{status:Some("Pulling".into()),progress:Some(JsonProgressReply{current:Some(1),total:Some(2),..Default::default()}),..Default::default()});
            std::future::pending::<()>().await;
        })))
    }
    type BuildStream = Output<ImageBuildResponse>;
    async fn build(
        &self,
        _: Request<BuildImageRequest>,
    ) -> Result<Response<Self::BuildStream>, Status> {
        Err(Status::unimplemented("not used"))
    }
    type PushStream = Output<ImageBuildResponse>;
    async fn push(
        &self,
        _: Request<PushImageRequest>,
    ) -> Result<Response<Self::PushStream>, Status> {
        Err(Status::unimplemented("not used"))
    }
}

#[tokio::test]
async fn image_pull_stream_preserves_progress_and_cancels_without_retrying() {
    use citadel_platforms::image_pull::ImagePullPort;
    use futures_util::StreamExt;
    let (address, calls, stop, task) = host("28.0.0").await;
    let client = AgentClient::connect(
        &address,
        AgentRequestSigner::from_bytes(&[31; 32]),
        Duration::from_secs(1),
        true,
    )
    .await
    .unwrap();
    let cancel = CancellationToken::new();
    assert!(
        client
            .pull_image_stream("fail:latest", None, &cancel)
            .await
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let mut stream = client
        .pull_image_stream("nginx:latest", Some("encoded-auth"), &cancel)
        .await
        .unwrap();
    let item = stream.next().await.unwrap().unwrap();
    assert_eq!(item.status.as_deref(), Some("Pulling"));
    assert_eq!(item.progress.unwrap().total, Some(2));
    cancel.cancel();
    assert!(
        tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    stop.cancel();
    task.abort();
}

async fn host(
    version: &'static str,
) -> (
    String,
    Arc<AtomicUsize>,
    CancellationToken,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let incoming = futures_util::stream::unfold(listener, |listener| async {
        Some((listener.accept().await.map(|(stream, _)| stream), listener))
    });
    let calls = Arc::new(AtomicUsize::new(0));
    let service = BuildHost {
        version,
        calls: calls.clone(),
    };
    let cancellation = CancellationToken::new();
    let shutdown = cancellation.clone();
    let handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(ImageServiceServer::new(service))
            .serve_with_incoming_shutdown(Box::pin(incoming), shutdown.cancelled_owned())
            .await
            .unwrap();
    });
    (address, calls, cancellation, handle)
}

// BuildAgentPoolCommandTests self-managed checks, with real signed gRPC instead
// of mocking IImageConnector. A selected pool cannot check a different daemon.
#[tokio::test]
async fn signed_pool_check_uses_selected_endpoint_and_never_replays_authentication_failure() {
    let (first, first_calls, first_stop, first_task) = host("28.0.0").await;
    let (second, second_calls, second_stop, second_task) = host("29.0.0").await;
    let cancellation = CancellationToken::new();
    let client = AgentClient::connect(
        &first,
        AgentRequestSigner::from_bytes(&[31; 32]),
        Duration::from_secs(2),
        true,
    )
    .await
    .unwrap();
    assert_eq!(
        client
            .check_build_host(&cancellation)
            .await
            .unwrap()
            .docker_version,
        "28.0.0"
    );
    let selected = client.for_address(&second, &cancellation).await.unwrap();
    assert_eq!(
        selected
            .check_build_host(&cancellation)
            .await
            .unwrap()
            .docker_version,
        "29.0.0"
    );
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
    let unauthorized = AgentClient::connect(
        &second,
        AgentRequestSigner::from_bytes(&[32; 32]),
        Duration::from_secs(2),
        true,
    )
    .await
    .unwrap();
    assert_eq!(
        unauthorized
            .check_build_host(&cancellation)
            .await
            .unwrap_err()
            .kind,
        citadel_platforms::RuntimeErrorKind::Authentication
    );
    assert_eq!(second_calls.load(Ordering::SeqCst), 2);
    cancellation.cancel();
    assert!(client.for_address(&second, &cancellation).await.is_err());
    assert!(selected.check_build_host(&cancellation).await.is_err());
    assert_eq!(second_calls.load(Ordering::SeqCst), 2);
    first_stop.cancel();
    second_stop.cancel();
    first_task.await.unwrap();
    second_task.await.unwrap();
}

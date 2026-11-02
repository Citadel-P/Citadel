use super::*;
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, StatusCode, Uri},
    response::Response,
};
use citadel_execution::ProcessOutput;
use serde_json::{Value, json};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

const MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/release/fixtures/update-stable.json"
));

struct Runner {
    calls: AtomicUsize,
    response: Mutex<Option<Result<ProcessOutput, ProcessError>>>,
    home: Mutex<Option<std::path::PathBuf>>,
}

impl Runner {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            response: Mutex::new(Some(Ok(valid_signature()))),
            home: Mutex::new(None),
        }
    }
}

impl ProcessRunner for Runner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(request.program, "cosign");
            assert_eq!(request.arguments, ["verify", "--certificate-identity",
                "https://github.com/Citadel-P/Citadel/.github/workflows/ci.yml@refs/tags/v1.2.3",
                "--certificate-oidc-issuer", RELEASE_OIDC_ISSUER,
                "--check-claims=true", "--output=json", &format!("ghcr.io/citadel-p/citadel@sha256:{}", "b".repeat(64))]
                .map(std::ffi::OsString::from));
            assert_eq!(request.environment.len(), 1);
            assert_eq!(request.environment[0].0, "HOME");
            let home = std::path::PathBuf::from(&request.environment[0].1);
            assert!(home.is_dir());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    std::fs::metadata(&home).unwrap().permissions().mode() & 0o077,
                    0
                );
            }
            *self.home.lock().unwrap() = Some(home);
            assert!(request.stdin.is_none());
            assert!(request.output.is_none());
            assert_eq!(request.limits.timeout, Duration::from_secs(90));
            assert_eq!(request.limits.maximum_stdout_bytes, 256 * 1024);
            assert_eq!(request.limits.maximum_stderr_bytes, 16 * 1024);
            self.response.lock().unwrap().take().unwrap()
        })
    }
}

fn valid_signature() -> ProcessOutput {
    ProcessOutput {
        exit_code: Some(0),
        stdout: serde_json::to_vec(&json!([{"critical":{"image":{"docker-manifest-digest":format!("sha256:{}", "b".repeat(64))}}}])).unwrap(),
        stderr: vec![], stdout_truncated: false, stderr_truncated: false,
    }
}

struct Fixture {
    source: GithubReleaseArtifacts,
    runner: Arc<Runner>,
    server: Option<tokio::task::JoinHandle<()>>,
    requests: Arc<AtomicUsize>,
}
impl Fixture {
    async fn start(
        publication: Value,
        manifest: Vec<u8>,
        checksum: Option<String>,
        status: StatusCode,
    ) -> Self {
        let requests = Arc::new(AtomicUsize::new(0));
        let count = requests.clone();
        let checksum = checksum.unwrap_or_else(|| {
            let digest: String = Sha256::digest(&manifest)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            format!("{digest}  release-manifest.json\n")
        });
        let router = Router::new().fallback(move |uri: Uri, headers: HeaderMap| {
            let publication = publication.clone();
            let manifest = manifest.clone();
            let checksum = checksum.clone();
            count.fetch_add(1, Ordering::SeqCst);
            async move {
                assert!(!headers.contains_key("authorization"));
                assert!(!headers.contains_key("cookie"));
                assert_eq!(headers["user-agent"], "Citadel-Update-Check");
                assert!(uri.query().is_none());
                let body = match uri.path() {
                    "/releases/tags/v1.2.3" => serde_json::to_vec(&publication).unwrap(),
                    "/v1.2.3/release-manifest.json" => manifest,
                    "/v1.2.3/SHA256SUMS" => checksum.into_bytes(),
                    unexpected => panic!("unexpected request {unexpected}"),
                };
                Response::builder()
                    .status(status)
                    .body(Body::from(body))
                    .unwrap()
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let root = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let runner = Arc::new(Runner::new());
        let mut source = GithubReleaseArtifacts::new(runner.clone()).unwrap();
        source.api_root = root.clone();
        source.download_root = root;
        Self {
            source,
            runner,
            server: Some(server),
            requests,
        }
    }
    async fn normal() -> Self {
        Self::start(
            publication(),
            MANIFEST.as_bytes().to_vec(),
            None,
            StatusCode::OK,
        )
        .await
    }
    async fn stop(mut self) {
        let server = self.server.take().unwrap();
        server.abort();
        let _ = server.await;
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(server) = &self.server {
            server.abort();
        }
    }
}
fn publication() -> Value {
    json!({"tag_name":"v1.2.3", "draft":false, "prerelease":false,
        "assets":[{"name":"release-manifest.json", "browser_download_url":"https://attacker.invalid"}, {"name":"SHA256SUMS"}]})
}

#[tokio::test]
async fn completed_publication_requires_an_actual_digest_signature_and_private_tool_environment() {
    let fixture = Fixture::normal().await;
    let artifact = fixture
        .source
        .verify("1.2.3", &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(
        artifact.image,
        format!("ghcr.io/citadel-p/citadel@sha256:{}", "b".repeat(64))
    );
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 3);
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), 1);
    assert!(
        !fixture
            .runner
            .home
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .exists()
    );
    fixture.stop().await;
}

#[tokio::test]
async fn unpublished_mismatched_and_incomplete_releases_never_reach_the_verifier() {
    for (field, value) in [
        ("draft", json!(true)),
        ("prerelease", json!(true)),
        ("tag_name", json!("v1.2.4")),
        ("assets", json!([])),
        (
            "assets",
            json!([{"name":"SHA256SUMS"},{"name":"release-manifest.json"},{"name":"release-manifest.json"}]),
        ),
    ] {
        let mut document = publication();
        document[field] = value;
        let fixture =
            Fixture::start(document, MANIFEST.as_bytes().to_vec(), None, StatusCode::OK).await;
        assert!(
            fixture
                .source
                .verify("1.2.3", &CancellationToken::new())
                .await
                .is_err(),
            "{field}"
        );
        assert_eq!(fixture.requests.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), 0);
        fixture.stop().await;
    }
}

#[tokio::test]
async fn corrupt_oversized_failed_or_untrusted_metadata_is_rejected_before_signature_io() {
    let mut untrusted: Value = serde_json::from_str(MANIFEST).unwrap();
    untrusted["signatures"]["certificateIdentity"] = json!("https://attacker.invalid");
    for (manifest, checksum, status) in [
        (
            MANIFEST.as_bytes().to_vec(),
            Some("invalid".into()),
            StatusCode::OK,
        ),
        (b"not json".to_vec(), None, StatusCode::OK),
        (vec![b'x'; MAX_MANIFEST + 1], None, StatusCode::OK),
        (
            MANIFEST.as_bytes().to_vec(),
            None,
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
        (
            serde_json::to_vec(&untrusted).unwrap(),
            None,
            StatusCode::OK,
        ),
    ] {
        let fixture = Fixture::start(publication(), manifest, checksum, status).await;
        assert!(
            fixture
                .source
                .verify("1.2.3", &CancellationToken::new())
                .await
                .is_err()
        );
        assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), 0);
        fixture.stop().await;
    }
}

#[tokio::test]
async fn failed_empty_wrong_or_truncated_signature_results_never_become_verified_artifacts() {
    let mut cases = vec![];
    let mut failed = valid_signature();
    failed.exit_code = Some(1);
    failed.stderr = b"fixture private output".to_vec();
    cases.push(Ok(failed));
    let mut empty = valid_signature();
    empty.stdout = b"[]".to_vec();
    cases.push(Ok(empty));
    let mut wrong = valid_signature();
    wrong.stdout =
        br#"[{"critical":{"image":{"docker-manifest-digest":"sha256:wrong"}}}]"#.to_vec();
    cases.push(Ok(wrong));
    let mut truncated = valid_signature();
    truncated.stdout_truncated = true;
    cases.push(Ok(truncated));
    cases.push(Err(ProcessError::Timeout(Duration::from_secs(90))));
    cases.push(Err(ProcessError::Cancelled));
    for response in cases {
        let cancelled = matches!(response, Err(ProcessError::Cancelled));
        let fixture = Fixture::normal().await;
        *fixture.runner.response.lock().unwrap() = Some(response);
        let error = fixture
            .source
            .verify("1.2.3", &CancellationToken::new())
            .await
            .unwrap_err();
        assert_eq!(matches!(error, UpdateError::Cancelled), cancelled);
        assert!(!error.to_string().contains("fixture private output"));
        assert!(
            !fixture
                .runner
                .home
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .exists()
        );
        fixture.stop().await;
    }
}

#[tokio::test]
async fn invalid_versions_and_pre_cancelled_attempts_do_not_make_requests() {
    let fixture = Fixture::normal().await;
    for version in ["latest", "1.2.3/../../other", "v1.2.3"] {
        assert!(
            fixture
                .source
                .verify(version, &CancellationToken::new())
                .await
                .is_err()
        );
    }
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        fixture.source.verify("1.2.3", &token).await,
        Err(UpdateError::Cancelled)
    ));
    assert_eq!(fixture.requests.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), 0);
    fixture.stop().await;
}

#[test]
fn redirects_allow_only_official_https_asset_hosts() {
    for allowed in [
        "https://github.com/Citadel-P/Citadel",
        "https://release-assets.githubusercontent.com/path?signature=value",
    ] {
        assert!(permitted_redirect(&allowed.parse().unwrap()));
    }
    for rejected in [
        "http://github.com/",
        "https://github.com:444/",
        "https://github.com.attacker.invalid/",
        "https://attacker@github.com/",
        "https://127.0.0.1/",
        "file:///etc/passwd",
    ] {
        assert!(
            !permitted_redirect(&rejected.parse().unwrap()),
            "{rejected}"
        );
    }
}

#[tokio::test]
async fn cancellation_waits_for_verifier_cleanup_before_removing_its_private_home() {
    #[derive(Default)]
    struct CancellingRunner {
        started: tokio::sync::Notify,
        cleaning: tokio::sync::Notify,
        finish: tokio::sync::Notify,
        home: Mutex<Option<std::path::PathBuf>>,
    }
    impl ProcessRunner for CancellingRunner {
        fn run<'a>(
            &'a self,
            request: ProcessRequest,
            cancellation: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
            Box::pin(async move {
                *self.home.lock().unwrap() = Some(request.environment[0].1.clone().into());
                self.started.notify_one();
                cancellation.cancelled().await;
                self.cleaning.notify_one();
                self.finish.notified().await;
                Err(ProcessError::Cancelled)
            })
        }
    }
    let runner = Arc::new(CancellingRunner::default());
    let source = GithubReleaseArtifacts::new(runner.clone()).unwrap();
    let cancellation = CancellationToken::new();
    let image = format!("ghcr.io/citadel-p/citadel@sha256:{}", "b".repeat(64));
    let attempt = source.verify_image("1.2.3", &image, &cancellation);
    tokio::pin!(attempt);
    tokio::select! {
        _ = runner.started.notified() => {},
        _ = &mut attempt => panic!("verification should be pending"),
    }
    cancellation.cancel();
    tokio::select! {
        _ = runner.cleaning.notified() => {},
        _ = &mut attempt => panic!("verification must await cleanup"),
    }
    let home = runner.home.lock().unwrap().clone().unwrap();
    assert!(home.exists());
    runner.finish.notify_one();
    assert!(matches!(attempt.await, Err(UpdateError::Cancelled)));
    assert!(!home.exists());
}

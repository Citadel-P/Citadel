//! Bounded OCI metadata reads. No layers, redirects with credentials, or registry writes.
use base64::Engine;
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind,
    image_updates::{ImagePlatform, RegistryImageObservation, reference_host},
};
use futures_util::StreamExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

const ACCEPT: &str = "application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.v2+json";
const LIMIT: usize = 2 * 1024 * 1024;
fn failed() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Remote,
        "Registry metadata check failed. Verify Registry credentials and HTTPS connectivity.",
        false,
    )
}
fn response_error(status: reqwest::StatusCode) -> RuntimeCapabilityError {
    use RuntimeErrorKind::*;
    let (kind, message) = match status {
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => (
            Authentication,
            "The Registry denied access to this image/tag. It may be private or unpublished; verify the reference and Registry credentials.",
        ),
        reqwest::StatusCode::NOT_FOUND => (
            NotFound,
            "The Registry could not find this image/tag. Locally built images must be published before registry updates can be checked.",
        ),
        reqwest::StatusCode::TOO_MANY_REQUESTS => (
            ResourceExhausted,
            "The Registry is limiting requests. Try again later.",
        ),
        _ => return failed(),
    };
    RuntimeCapabilityError::new(kind, message, false)
}
fn unknown() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Unavailable,
        "The Registry did not supply an unambiguous manifest for this image's OS, architecture and variant.",
        false,
    )
}
fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_default()
}
fn valid_digest(digest: &str) -> bool {
    digest
        .strip_prefix("sha256:")
        .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
}
fn hash(bytes: &[u8]) -> String {
    format!(
        "sha256:{}",
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

pub async fn inspect(
    reference: &str,
    auth: Option<&str>,
    target: &ImagePlatform,
    cancel: &CancellationToken,
) -> Result<RegistryImageObservation, RuntimeCapabilityError> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| failed())?;
    let host = reference_host(reference);
    let endpoint = if host == "docker.io" {
        "registry-1.docker.io"
    } else {
        &host
    };
    let base = url::Url::parse(&format!("https://{endpoint}/")).map_err(|_| failed())?;
    inspect_with_client(client, base, reference, auth, target, cancel).await
}
async fn inspect_with_client(
    client: reqwest::Client,
    base: url::Url,
    reference: &str,
    auth: Option<&str>,
    target: &ImagePlatform,
    cancel: &CancellationToken,
) -> Result<RegistryImageObservation, RuntimeCapabilityError> {
    let operation = async {
        let host = reference_host(reference);
        let mut name = reference.to_owned();
        let first = reference.split('/').next().unwrap_or_default();
        if reference.contains('/')
            && (first.contains('.') || first.contains(':') || first == "localhost")
        {
            name = reference.split_once('/').unwrap().1.into();
        }
        let pinned = name.split_once('@');
        let (repository, tag) =
            pinned.unwrap_or_else(|| name.rsplit_once(':').unwrap_or((&name, "latest")));
        let valid_reference = if pinned.is_some() {
            valid_digest(tag)
        } else {
            !tag.is_empty()
                && tag
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        };
        if !valid_reference
            || repository.split('/').any(|s| {
                s.is_empty()
                    || s == "."
                    || s == ".."
                    || !s.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b)
                    })
            })
        {
            return Err(failed());
        }
        let repository = if host == "docker.io" && !repository.contains('/') {
            format!("library/{repository}")
        } else {
            repository.into()
        };
        let credentials = match auth {
            Some(value) => {
                let bytes = zeroize::Zeroizing::new(
                    base64::engine::general_purpose::STANDARD
                        .decode(value)
                        .map_err(|_| failed())?,
                );
                let cfg: Value = serde_json::from_slice(&bytes).map_err(|_| failed())?;
                if citadel_platforms::image_updates::registry_host(text(&cfg, "serveraddress"))
                    != host
                {
                    return Err(failed());
                }
                Some((
                    zeroize::Zeroizing::new(text(&cfg, "username").to_owned()),
                    zeroize::Zeroizing::new(text(&cfg, "password").to_owned()),
                ))
            }
            None => None,
        };
        let mut session = Session {
            client,
            base,
            repository,
            credentials,
            token: None,
        };
        let (top_digest, mut manifest) = session.document("manifests", tag).await?;
        if pinned.is_some() && top_digest != tag {
            return Err(failed());
        }
        let mut manifest_digest = top_digest.clone();
        if let Some(manifests) = manifest.get("manifests").and_then(Value::as_array) {
            if manifests.len() > 256 {
                return Err(unknown());
            }
            let matches: Vec<_> = manifests
                .iter()
                .filter(|m| {
                    let p = &m["platform"];
                    text(p, "os") == target.os
                        && text(p, "architecture") == target.architecture
                        && text(p, "variant") == target.variant
                })
                .collect();
            if matches.len() != 1 {
                return Err(unknown());
            }
            let digest = text(matches[0], "digest");
            if !valid_digest(digest) {
                return Err(unknown());
            }
            let (observed, selected) = session.document("manifests", digest).await?;
            if observed != digest {
                return Err(failed());
            }
            manifest = selected;
            manifest_digest = observed;
        } else {
            // A standalone manifest has no platform descriptor: verify its config.
            let digest = text(&manifest["config"], "digest");
            if !valid_digest(digest) {
                return Err(unknown());
            }
            let (observed, config) = session.document("blobs", digest).await?;
            if observed != digest {
                return Err(failed());
            }
            if text(&config, "os") != target.os
                || text(&config, "architecture") != target.architecture
                || text(&config, "variant") != target.variant
            {
                return Err(unknown());
            }
        }
        if manifest["schemaVersion"].as_u64() != Some(2)
            || !matches!(
                text(&manifest, "mediaType"),
                "application/vnd.oci.image.manifest.v1+json"
                    | "application/vnd.docker.distribution.manifest.v2+json"
            )
        {
            return Err(unknown());
        }
        let config = &manifest["config"];
        if !matches!(
            text(config, "mediaType"),
            "application/vnd.oci.image.config.v1+json"
                | "application/vnd.docker.container.image.v1+json"
        ) || !valid_digest(text(config, "digest"))
        {
            return Err(unknown());
        }
        Ok(RegistryImageObservation {
            config_digest: text(config, "digest").into(),
            manifest_digest,
            index_digest: top_digest,
            observed_at: chrono::Utc::now(),
        })
    };
    tokio::select! { biased; ()=cancel.cancelled()=>Err(RuntimeCapabilityError::new(RuntimeErrorKind::Cancelled,"Image check cancelled.",false)), result=tokio::time::timeout(std::time::Duration::from_secs(35),operation)=>result.map_err(|_|failed())? }
}
struct Session {
    client: reqwest::Client,
    base: url::Url,
    repository: String,
    credentials: Option<(zeroize::Zeroizing<String>, zeroize::Zeroizing<String>)>,
    token: Option<zeroize::Zeroizing<String>>,
}
impl Session {
    async fn document(
        &mut self,
        kind: &str,
        name: &str,
    ) -> Result<(String, Value), RuntimeCapabilityError> {
        let url = self
            .base
            .join(&format!("v2/{}/{kind}/{name}", self.repository))
            .map_err(|_| failed())?;
        let mut response = self
            .request(url.clone())
            .send()
            .await
            .map_err(|_| failed())?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            let challenge = response
                .headers()
                .get(reqwest::header::WWW_AUTHENTICATE)
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| response_error(response.status()))?;
            let fields = challenge
                .strip_prefix("Bearer ")
                .or_else(|| challenge.strip_prefix("bearer "))
                .ok_or_else(|| response_error(response.status()))?;
            let fields: std::collections::BTreeMap<_, _> = fields
                .split(',')
                .filter_map(|s| s.trim().split_once('='))
                .map(|(k, v)| (k, v.trim_matches('"')))
                .collect();
            let realm =
                url::Url::parse(fields.get("realm").ok_or_else(failed)?).map_err(|_| failed())?;
            let same_origin = realm.origin() == self.base.origin();
            let docker_auth = self.base.host_str() == Some("registry-1.docker.io")
                && realm.host_str() == Some("auth.docker.io")
                && realm.port_or_known_default() == Some(443);
            if realm.scheme() != "https"
                || !realm.username().is_empty()
                || realm.password().is_some()
                || !(same_origin || docker_auth)
            {
                return Err(failed());
            }
            let mut request = self.client.get(realm).query(&[
                (
                    "service",
                    fields.get("service").copied().unwrap_or_default(),
                ),
                ("scope", &format!("repository:{}:pull", self.repository)),
            ]);
            if let Some((user, password)) = &self.credentials {
                request = request.basic_auth(user.as_str(), Some(password.as_str()));
            }
            let bytes = body(request.send().await.map_err(|_| failed())?).await?;
            let token: Value = serde_json::from_slice(&bytes).map_err(|_| failed())?;
            let token = token
                .get("token")
                .or_else(|| token.get("access_token"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty() && s.len() < 65536)
                .ok_or_else(failed)?;
            self.token = Some(zeroize::Zeroizing::new(token.to_owned()));
            response = self.request(url).send().await.map_err(|_| failed())?;
        }
        let bytes = body(response).await?;
        Ok((
            hash(&bytes),
            serde_json::from_slice(&bytes).map_err(|_| failed())?,
        ))
    }
    fn request(&self, url: url::Url) -> reqwest::RequestBuilder {
        let request = self.client.get(url).header(reqwest::header::ACCEPT, ACCEPT);
        if let Some(token) = &self.token {
            request.bearer_auth(token.as_str())
        } else if let Some((user, password)) = &self.credentials {
            request.basic_auth(user.as_str(), Some(password.as_str()))
        } else {
            request
        }
    }
}
async fn body(response: reqwest::Response) -> Result<Vec<u8>, RuntimeCapabilityError> {
    if !response.status().is_success() {
        // Classify the status without exposing registry response bodies or auth headers.
        return Err(response_error(response.status()));
    }
    if response.content_length().is_some_and(|n| n > LIMIT as u64) {
        return Err(failed());
    }
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| failed())?;
        if body.len() + chunk.len() > LIMIT {
            return Err(failed());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        extract::State,
        http::{Request, Response},
    };
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex},
    };
    struct Fixture {
        base: url::Url,
        paths: Arc<Mutex<Vec<String>>>,
        task: tokio::task::JoinHandle<()>,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.task.abort();
        }
    }
    async fn fixture(single: bool, duplicate: bool, arm_digest: &str) -> (Fixture, String) {
        let config=serde_json::json!({"architecture":"amd64","os":"linux","rootfs":{"type":"layers","diff_ids":[]}}).to_string();
        let config_digest = hash(config.as_bytes());
        let manifest=serde_json::json!({"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{"mediaType":"application/vnd.oci.image.config.v1+json","digest":config_digest},"layers":[]}).to_string();
        let digest = hash(manifest.as_bytes());
        let amd =
            serde_json::json!({"digest":digest,"platform":{"os":"linux","architecture":"amd64"}});
        let mut entries = vec![
            amd.clone(),
            serde_json::json!({"digest":arm_digest,"platform":{"os":"linux","architecture":"arm64","variant":"v8"}}),
        ];
        if duplicate {
            entries.push(amd);
        }
        let index=serde_json::json!({"schemaVersion":2,"mediaType":"application/vnd.oci.image.index.v1+json","manifests":entries}).to_string();
        let index_digest = hash(index.as_bytes());
        let documents = BTreeMap::from([
            (
                format!("/v2/library/nginx/manifests/{index_digest}"),
                index.clone(),
            ),
            (
                "/v2/library/nginx/manifests/ReleaseA".to_owned(),
                if single { manifest.clone() } else { index },
            ),
            (format!("/v2/library/nginx/manifests/{digest}"), manifest),
            (format!("/v2/library/nginx/blobs/{config_digest}"), config),
        ]);
        let paths = Arc::new(Mutex::new(Vec::new()));
        let state = (Arc::new(documents), paths.clone());
        type FixtureState = (Arc<BTreeMap<String, String>>, Arc<Mutex<Vec<String>>>);
        async fn handle(
            State((documents, paths)): State<FixtureState>,
            request: Request<Body>,
        ) -> Response<Body> {
            let path = request.uri().path().to_owned();
            paths.lock().unwrap().push(path.clone());
            assert_eq!(request.method(), "GET");
            match documents.get(&path) {
                Some(body) => Response::new(Body::from(body.clone())),
                None => Response::builder().status(404).body(Body::empty()).unwrap(),
            }
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = url::Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        let app = Router::new().fallback(handle).with_state(state);
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (Fixture { base, paths, task }, config_digest)
    }
    fn target() -> ImagePlatform {
        ImagePlatform {
            os: "linux".into(),
            architecture: "amd64".into(),
            variant: String::new(),
        }
    }
    async fn read(f: &Fixture) -> Result<RegistryImageObservation, RuntimeCapabilityError> {
        inspect_with_client(
            reqwest::Client::new(),
            f.base.clone(),
            "nginx:ReleaseA",
            None,
            &target(),
            &CancellationToken::new(),
        )
        .await
    }
    #[tokio::test]
    async fn metadata_comparison_selects_host_config_not_changing_index_or_other_architecture() {
        let (a, config) = fixture(false, false, &format!("sha256:{}", "a".repeat(64))).await;
        let (b, _) = fixture(false, false, &format!("sha256:{}", "b".repeat(64))).await;
        let a_result = read(&a).await.unwrap();
        let b_result = read(&b).await.unwrap();
        assert_ne!(a_result.index_digest, b_result.index_digest);
        assert_eq!(a_result.config_digest, config);
        assert_eq!(a_result.config_digest, b_result.config_digest);
        assert_eq!(
            a.paths.lock().unwrap().len(),
            2,
            "only index and selected manifest are fetched"
        );
        assert!(
            a.paths
                .lock()
                .unwrap()
                .iter()
                .all(|p| p.contains("/manifests/"))
        );
    }
    #[tokio::test]
    async fn single_manifest_verifies_config_and_ambiguous_platforms_remain_unknown() {
        let (single, config) = fixture(true, false, "").await;
        assert_eq!(read(&single).await.unwrap().config_digest, config);
        assert_eq!(single.paths.lock().unwrap().len(), 2);
        let (ambiguous, _) = fixture(false, true, "").await;
        assert_eq!(
            read(&ambiguous).await.unwrap_err().kind,
            RuntimeErrorKind::Unavailable
        );
        let mut arm = target();
        arm.architecture = "arm64".into();
        assert_eq!(
            inspect_with_client(
                reqwest::Client::new(),
                ambiguous.base.clone(),
                "nginx:ReleaseA",
                None,
                &arm,
                &CancellationToken::new()
            )
            .await
            .unwrap_err()
            .kind,
            RuntimeErrorKind::Unavailable
        );
    }
    #[tokio::test]
    async fn immutable_local_content_resolves_config_and_rejects_digest_substitution() {
        let (fixture, config) = fixture(false, false, &format!("sha256:{}", "a".repeat(64))).await;
        let current = read(&fixture).await.unwrap();
        for digest in [&current.index_digest, &current.manifest_digest] {
            let result = inspect_with_client(
                reqwest::Client::new(),
                fixture.base.clone(),
                &format!("nginx@{digest}"),
                None,
                &target(),
                &CancellationToken::new(),
            )
            .await
            .unwrap();
            assert_eq!(result.index_digest, *digest);
            assert_eq!(result.config_digest, config);
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = url::Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        let app = Router::new().fallback(|| async { "{}" });
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let substitute = Fixture {
            base,
            paths: Default::default(),
            task,
        };
        assert!(
            inspect_with_client(
                reqwest::Client::new(),
                substitute.base.clone(),
                &format!("nginx@{}", current.index_digest),
                None,
                &target(),
                &CancellationToken::new(),
            )
            .await
            .is_err()
        );
        assert!(
            fixture
                .paths
                .lock()
                .unwrap()
                .iter()
                .all(|p| p.contains("/manifests/") || p.ends_with(&config))
        );
    }

    #[tokio::test]
    async fn credentials_for_another_origin_are_rejected_without_network_access() {
        let (f, _) = fixture(false, false, "").await;
        let auth = base64::engine::general_purpose::STANDARD.encode(
            br#"{"serveraddress":"private.example","username":"user","password":"secret"}"#,
        );
        assert!(
            inspect_with_client(
                reqwest::Client::new(),
                f.base.clone(),
                "nginx:ReleaseA",
                Some(&auth),
                &target(),
                &CancellationToken::new()
            )
            .await
            .is_err()
        );
        assert!(f.paths.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unpublished_image_errors_preserve_categories_without_exposing_registry_bodies() {
        use RuntimeErrorKind::*;
        async fn rejected(State(status): State<u16>, request: Request<Body>) -> Response<Body> {
            assert_eq!(
                request.uri().path(),
                "/v2/library/citadel-agent/manifests/local"
            );
            Response::builder()
                .status(status)
                .header("www-authenticate", "Basic realm=private-registry")
                .body(Body::from("private-response-with-secret"))
                .unwrap()
        }
        for (status, expected) in [
            (401, Authentication),
            (403, Authentication),
            (404, NotFound),
            (429, ResourceExhausted),
            (500, Remote),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base =
                url::Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
            let app = Router::new().fallback(rejected).with_state(status);
            let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            let fixture = Fixture {
                base,
                paths: Arc::new(Mutex::new(Vec::new())),
                task,
            };
            let error = inspect_with_client(
                reqwest::Client::new(),
                fixture.base.clone(),
                "citadel-agent:local",
                None,
                &target(),
                &CancellationToken::new(),
            )
            .await
            .unwrap_err();
            assert_eq!(error.kind, expected, "HTTP {status}");
            assert!(!error.message.contains("secret"));
            assert!(!error.message.contains("private-registry"));
            // This is also the path for a second 401 after the Bearer-token exchange.
            let response = reqwest::get(
                fixture
                    .base
                    .join("v2/library/citadel-agent/manifests/local")
                    .unwrap(),
            )
            .await
            .unwrap();
            assert_eq!(body(response).await.unwrap_err().kind, expected);
        }
    }
}

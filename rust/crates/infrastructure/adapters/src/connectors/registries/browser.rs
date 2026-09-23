use citadel_registries::RegistryError;
use citadel_registries::registry_images::*;
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone)]
pub struct RegistryBrowser {
    client: reqwest::Client,
    docker_hub: String,
    github: String,
}
impl RegistryBrowser {
    pub fn new() -> Result<Self, RegistryError> {
        Self::with_endpoints("https://hub.docker.com", "https://api.github.com")
    }
    /// Infrastructure injection for external-service fixtures. End-user input never sets these origins.
    pub fn with_endpoints(docker_hub: &str, github: &str) -> Result<Self, RegistryError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("Citadel")
            .build()
            .map_err(|_| failure())?;
        Ok(Self {
            client,
            docker_hub: docker_hub.trim_end_matches('/').into(),
            github: github.trim_end_matches('/').into(),
        })
    }
    pub async fn browse(
        &self,
        configuration: &Value,
        kind: RegistryBrowseKind,
        name: Option<&str>,
    ) -> Result<Value, RegistryError> {
        let cfg_type = field(configuration, &["$type"]).unwrap_or_default();
        match (cfg_type, kind) {
            (
                "DockerHub",
                RegistryBrowseKind::Repositories
                | RegistryBrowseKind::DockerHubRepositories
                | RegistryBrowseKind::DockerHubTags,
            ) => self.docker(configuration, kind, name).await,
            ("GitHub", RegistryBrowseKind::Repositories | RegistryBrowseKind::GithubVersions) => {
                self.github(configuration, kind, name).await
            }
            _ => Err(RegistryError::NotFound),
        }
    }
    async fn docker(
        &self,
        cfg: &Value,
        kind: RegistryBrowseKind,
        name: Option<&str>,
    ) -> Result<Value, RegistryError> {
        let username = field(cfg, &["UserName", "userName", "username"]).unwrap_or_default();
        let token = self.docker_token(cfg).await?;
        let namespace = urlencoding::encode(username);
        let path = if kind == RegistryBrowseKind::DockerHubTags {
            format!(
                "/v2/namespaces/{namespace}/repositories/{}/tags?page=1&page_size=100",
                urlencoding::encode(name.unwrap_or_default())
            )
        } else {
            format!("/v2/repositories/{namespace}?page=1&page_size=100")
        };
        let response = body(
            self.client
                .get(format!("{}{path}", self.docker_hub))
                .bearer_auth(token)
                .send()
                .await
                .map_err(|_| failure())?,
        )
        .await?;
        let values = response
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(failure)?;
        if values.len() > 100 {
            return Err(failure());
        }
        let mut result = Vec::with_capacity(values.len());
        for value in values {
            if kind == RegistryBrowseKind::DockerHubTags {
                result.push(tag_view(value));
            } else {
                let repo: DockerHubRepositoryInfo =
                    serde_json::from_value(value.clone()).map_err(|_| failure())?;
                let mut repo = serde_json::to_value(repo).map_err(|_| failure())?;
                if kind == RegistryBrowseKind::Repositories {
                    let o = repo.as_object_mut().expect("repository object");
                    o.insert("$type".into(), "DockerHub".into());
                    o.remove("isTrusted");
                    o.remove("isAutomated");
                }
                result.push(repo);
            }
        }
        Ok(Value::Array(result))
    }
    async fn docker_token(&self, cfg: &Value) -> Result<String, RegistryError> {
        let username = field(cfg, &["UserName", "userName", "username"]).unwrap_or_default();
        let secret = field(cfg, &["PAT", "pat"]).unwrap_or_default();
        let response = self
            .client
            .post(format!("{}/v2/auth/token", self.docker_hub))
            .json(&json!({"identifier": username, "secret": secret}))
            .send()
            .await
            .map_err(|_| failure())?;
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(RegistryError::Validation(
                "Docker Hub rejected these credentials. Check your username and personal access token.".into(),
            ));
        }
        let payload = body(response).await?;
        payload
            .get("access_token")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .map(str::to_owned)
            .ok_or_else(failure)
    }
    async fn github(
        &self,
        cfg: &Value,
        kind: RegistryBrowseKind,
        name: Option<&str>,
    ) -> Result<Value, RegistryError> {
        let namespace = field(cfg, &["NameSpace", "nameSpace"]).unwrap_or_default();
        let token = field(cfg, &["PAT", "pat"]).unwrap_or_default();
        let path = if kind == RegistryBrowseKind::GithubVersions {
            format!(
                "/user/packages/container/{}/versions?page=1&per_page=100",
                urlencoding::encode(name.unwrap_or_default())
            )
        } else {
            format!(
                "/users/{}/packages?package_type=container&page=1&per_page=100",
                urlencoding::encode(namespace)
            )
        };
        let response = body(
            self.client
                .get(format!("{}{path}", self.github))
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|_| failure())?,
        )
        .await?;
        let values = response.as_array().ok_or_else(failure)?;
        if values.len() > 100 {
            return Err(failure());
        }
        let mut result = Vec::with_capacity(values.len());
        for value in values {
            if kind == RegistryBrowseKind::GithubVersions {
                let version: GithubPackageVersion =
                    serde_json::from_value(value.clone()).map_err(|_| failure())?;
                result.push(serde_json::to_value(version).map_err(|_| failure())?);
            } else {
                let id = value
                    .get("id")
                    .and_then(Value::as_i64)
                    .ok_or_else(failure)?;
                result.push(json!({"$type":"GitHub","id":id.to_string(),"name":value["name"],"createdAt":value["created_at"],"updatedAt":value["updated_at"],"url":value["url"],"htmlUrl":value["html_url"]}));
            }
        }
        Ok(Value::Array(result))
    }
}

impl citadel_registries::RegistryConnectionChecker for RegistryBrowser {
    fn check<'a>(
        &'a self,
        configuration: &'a Value,
    ) -> futures_util::future::BoxFuture<'a, Result<(), RegistryError>> {
        Box::pin(async move {
            let github_auth_enabled = configuration
                .as_object()
                .and_then(|fields| {
                    fields.iter().find_map(|(key, value)| {
                        key.eq_ignore_ascii_case("ghcrAuthEnabled").then_some(value)
                    })
                })
                .and_then(Value::as_bool)
                .unwrap_or(false);
            match citadel_registries::registry_type(configuration)?.as_str() {
                "DockerHub" => self.docker_token(configuration).await.map(|_| ()),
                "GitHub" if github_auth_enabled => {
                    self.github(configuration, RegistryBrowseKind::Repositories, None)
                        .await.map(|_| ()).map_err(|_| RegistryError::Validation(
                            "GitHub registry connection failed. Check connectivity, namespace, and personal access token permissions (read:packages).".into(),
                        ))
                }
                _ => Ok(()),
            }
        })
    }
}

fn field<'a>(value: &'a Value, names: &[&str]) -> Option<&'a str> {
    value.as_object()?.iter().find_map(|(key, value)| {
        names
            .iter()
            .any(|name| key.eq_ignore_ascii_case(name))
            .then(|| value.as_str())
            .flatten()
    })
}
fn failure() -> RegistryError {
    RegistryError::Validation(
        "Registry request failed. Check registry connectivity and credentials.".into(),
    )
}
async fn body(response: reqwest::Response) -> Result<Value, RegistryError> {
    if !response.status().is_success() {
        return Err(failure());
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| failure())?;
        if bytes.len() + chunk.len() > 4 * 1024 * 1024 {
            return Err(failure());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| failure())
}
fn tag_view(value: &Value) -> Value {
    let status = |v: &Value| {
        if v.as_str() == Some("active") {
            "Active"
        } else {
            "Inactive"
        }
    };
    let image = value.get("images").and_then(Value::as_array).and_then(|v|v.first()).map(|i|json!({"architecture":i["architecture"],"digest":i["digest"],"os":i["os"],"size":i["size"],"status":status(&i["status"]),"lastPulled":i["last_pulled"]}));
    json!({"id":value["id"],"name":value["name"],"image":image,"lastUpdated":value["last_updated"],"fullSize":value["full_size"],"status":status(&value["status"]),"lastPulled":value["tag_last_pulled"]})
}

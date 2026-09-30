use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryBrowseKind {
    Repositories,
    DockerHubRepositories,
    DockerHubTags,
    GithubVersions,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct DockerHubRepositoryInfo {
    pub name: Option<String>,
    pub namespace: Option<String>,
    pub last_updated: Option<String>,
    #[serde(default)]
    pub is_private: bool,
    #[serde(default)]
    pub is_trusted: bool,
    #[serde(default)]
    pub is_automated: bool,
    #[serde(default)]
    pub pull_count: i64,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct GithubPackageVersion {
    pub id: i64,
    pub name: String,
    pub url: Option<String>,
    pub html_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub package_html_url: Option<String>,
    pub metadata: Option<GithubPackageMetadata>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct GithubPackageMetadata {
    pub container: Option<GithubContainerMetadata>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct GithubContainerMetadata {
    pub tags: Option<Vec<String>>,
}

pub fn validate_browse_name(value: &str, registry: bool) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > if registry { 128 } else { 255 }
        || value.chars().any(char::is_control)
        || value == "."
        || value == ".."
    {
        return Err("A valid Registry and repository/package name are required.");
    }
    Ok(())
}

/// Repository summaries returned by the registry browser, never credentials.
#[derive(Debug, Serialize)]
#[serde(tag = "$type")]
pub enum ExternalRepository {
    #[serde(rename_all = "camelCase")]
    DockerHub {
        name: Option<String>,
        namespace: Option<String>,
        last_updated: Option<String>,
        is_private: bool,
        pull_count: i64,
    },
    #[serde(rename_all = "camelCase")]
    GitHub {
        id: String,
        name: Option<String>,
        created_at: Option<String>,
        updated_at: Option<String>,
        url: Option<String>,
        html_url: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum RegistryTagStatus {
    Active,
    Inactive,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerHubTag {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub image: Option<DockerHubTagImage>,
    pub last_updated: Option<String>,
    pub full_size: Option<i64>,
    pub status: RegistryTagStatus,
    pub last_pulled: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerHubTagImage {
    pub architecture: Option<String>,
    pub digest: Option<String>,
    pub os: Option<String>,
    pub size: Option<i64>,
    pub status: RegistryTagStatus,
    pub last_pulled: Option<String>,
}

//! Server-owned OpenAPI descriptions of registries values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

schema_model! {
    citadel_registries::registry_images::DockerHubRepositoryInfo =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
    #[schema(rename_all = "camelCase")]
    #[schema(as = DockerHubRepositoryInfo)]
    pub struct DockerHubRepositoryInfoSchema {
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
}

schema_model! {
    citadel_registries::registry_images::GithubPackageVersion =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
    #[schema(rename_all = "camelCase")]
    #[schema(as = GithubPackageVersion)]
    pub struct GithubPackageVersionSchema {
        pub id: i64,
        pub name: String,
        pub url: Option<String>,
        pub html_url: Option<String>,
        pub created_at: Option<String>,
        pub updated_at: Option<String>,
        pub package_html_url: Option<String>,
        #[schema(value_type = Option < crate::api::resources::schema_models::registries::GithubPackageMetadataSchema >)]
        pub metadata: Option<citadel_registries::registry_images::GithubPackageMetadata>,
    }
}

schema_model! {
    citadel_registries::registry_images::GithubPackageMetadata =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = GithubPackageMetadata)]
    pub struct GithubPackageMetadataSchema {
        #[schema(value_type = Option < crate::api::resources::schema_models::registries::GithubContainerMetadataSchema >)]
        pub container: Option<citadel_registries::registry_images::GithubContainerMetadata>,
    }
}

schema_model! {
    citadel_registries::registry_images::GithubContainerMetadata =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = GithubContainerMetadata)]
    pub struct GithubContainerMetadataSchema {
        pub tags: Option<Vec<String>>,
    }
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[doc = " Repository summaries returned by the registry browser, never credentials."]
#[serde(tag = "$type")]
#[schema(as = ExternalRepository)]
pub enum ExternalRepositorySchema {
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

impl From<citadel_registries::registry_images::ExternalRepository> for ExternalRepositorySchema {
    fn from(value: citadel_registries::registry_images::ExternalRepository) -> Self {
        match value {
            citadel_registries::registry_images::ExternalRepository::DockerHub {
                name,
                namespace,
                last_updated,
                is_private,
                pull_count,
            } => Self::DockerHub {
                name,
                namespace,
                last_updated,
                is_private,
                pull_count,
            },
            citadel_registries::registry_images::ExternalRepository::GitHub {
                id,
                name,
                created_at,
                updated_at,
                url,
                html_url,
            } => Self::GitHub {
                id,
                name,
                created_at,
                updated_at,
                url,
                html_url,
            },
        }
    }
}

enum_schema!(
    RegistryTagStatusSchema,
    "RegistryTagStatus",
    citadel_registries::registry_images::RegistryTagStatus,
    [Active, Inactive]
);

schema_model! {
    citadel_registries::registry_images::DockerHubTag =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = DockerHubTag)]
    pub struct DockerHubTagSchema {
        pub id: Option<i64>,
        pub name: Option<String>,
        #[schema(value_type = Option < crate::api::resources::schema_models::registries::DockerHubTagImageSchema >)]
        pub image: Option<citadel_registries::registry_images::DockerHubTagImage>,
        pub last_updated: Option<String>,
        pub full_size: Option<i64>,
        #[schema(value_type = crate::api::resources::schema_models::registries::RegistryTagStatusSchema)]
        pub status: citadel_registries::registry_images::RegistryTagStatus,
        pub last_pulled: Option<String>,
    }
}

schema_model! {
    citadel_registries::registry_images::DockerHubTagImage =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = DockerHubTagImage)]
    pub struct DockerHubTagImageSchema {
        pub architecture: Option<String>,
        pub digest: Option<String>,
        pub os: Option<String>,
        pub size: Option<i64>,
        #[schema(value_type = crate::api::resources::schema_models::registries::RegistryTagStatusSchema)]
        pub status: citadel_registries::registry_images::RegistryTagStatus,
        pub last_pulled: Option<String>,
    }
}

enum_schema!(
    RegistryStatusSchema,
    "RegistryStatus",
    citadel_registries::RegistryStatus,
    [Active, Disabled, Deprecated]
);

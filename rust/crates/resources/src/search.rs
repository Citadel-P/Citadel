use crate::ResourceMetadataError;
use citadel_domain::{ActorId, ResourceType};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const SEARCH_TYPES: &[(ResourceType, &str, usize)] = &[
    (ResourceType::Platform, "Platform", 0),
    (ResourceType::Stack, "Stack", 1),
    (ResourceType::Deployment, "Deployment", 2),
    (ResourceType::GitRepository, "GitRepository", 3),
    (ResourceType::Registry, "Registry", 4),
    (ResourceType::AutomationAction, "AutomationAction", 5),
    (ResourceType::BackupPolicy, "BackupPolicy", 6),
    (ResourceType::BackupRepository, "BackupRepository", 6),
    (ResourceType::Build, "Build", 7),
    (ResourceType::BuildAgentPool, "BuildAgentPool", 7),
    (ResourceType::SwarmService, "SwarmService", 8),
];
const CATEGORIES: [&str; 9] = [
    "Platforms",
    "Stacks",
    "Deployments",
    "Repositories",
    "Registries",
    "Automations",
    "Backups",
    "Builds",
    "SwarmServices",
];

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSearchQuery {
    pub q: Option<String>,
    pub types: Option<String>,
    pub limit_per_type: Option<i64>,
}

pub struct ValidatedSearch {
    pub query: String,
    pub resource_types: Vec<ResourceType>,
    pub limit_per_type: i64,
}

impl GlobalSearchQuery {
    pub fn validate(self) -> Result<ValidatedSearch, ResourceMetadataError> {
        let query = self.q.unwrap_or_default().trim().to_owned();
        if !(2..=100).contains(&query.chars().count()) {
            return Err(ResourceMetadataError::Validation(
                "q must be between 2 and 100 characters.".into(),
            ));
        }
        let limit_per_type = self.limit_per_type.unwrap_or(5);
        if !(1..=10).contains(&limit_per_type) {
            return Err(ResourceMetadataError::Validation(
                "limitPerType must be between 1 and 10.".into(),
            ));
        }
        let mut resource_types = Vec::new();
        if let Some(types) = self.types.filter(|value| !value.trim().is_empty()) {
            for part in types
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
            {
                let kind = SEARCH_TYPES
                    .iter()
                    .find(|(_, name, _)| name.eq_ignore_ascii_case(part))
                    .map(|(kind, _, _)| *kind)
                    .ok_or_else(|| {
                        ResourceMetadataError::Validation(format!(
                            "Search resource type '{part}' is not supported."
                        ))
                    })?;
                if !resource_types.contains(&kind) {
                    resource_types.push(kind);
                }
            }
            if resource_types.is_empty() {
                return Err(ResourceMetadataError::Validation(
                    "types must contain at least one supported resource type.".into(),
                ));
            }
        } else {
            resource_types.extend(SEARCH_TYPES.iter().map(|(kind, _, _)| *kind));
        }
        Ok(ValidatedSearch {
            query,
            resource_types,
            limit_per_type,
        })
    }
}

pub struct GlobalSearchMatch {
    pub id: Uuid,
    pub resource_type: ResourceType,
    pub name: String,
    pub secondary_text: Option<String>,
    pub status: Option<String>,
    pub parent: Option<GlobalSearchParent>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSearchParent {
    pub id: Uuid,
    pub resource_type: ResourceType,
    pub name: String,
}
#[derive(Serialize)]
pub struct GlobalSearchStatus {
    pub label: String,
    pub tone: &'static str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSearchItem {
    pub id: Uuid,
    pub resource_type: ResourceType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<GlobalSearchStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<GlobalSearchParent>,
}
#[derive(Serialize)]
pub struct GlobalSearchGroup {
    pub category: &'static str,
    pub items: Vec<GlobalSearchItem>,
}
#[derive(Serialize)]
pub struct GlobalSearchResponse {
    pub query: String,
    pub groups: Vec<GlobalSearchGroup>,
}

impl GlobalSearchResponse {
    pub fn from_matches(
        query: String,
        matches: Vec<GlobalSearchMatch>,
    ) -> Result<Self, ResourceMetadataError> {
        let mut groups: [Vec<GlobalSearchItem>; 9] = std::array::from_fn(|_| Vec::new());
        for item in matches {
            let index = SEARCH_TYPES
                .iter()
                .find(|(kind, _, _)| *kind == item.resource_type)
                .map(|(_, _, index)| *index)
                .ok_or_else(|| {
                    ResourceMetadataError::Storage("Unsupported search resource type.".into())
                })?;
            let status = item
                .status
                .filter(|status| !status.trim().is_empty())
                .map(|status| {
                    let tone = match status.as_str() {
                        "Online" | "Healthy" | "Active" | "Ready" | "Succeeded" | "Enabled" => {
                            "Positive"
                        }
                        "Offline" | "Failed" | "Invalid" | "TimedOut" | "Interrupted" => "Negative",
                        "Degraded"
                        | "Deprecated"
                        | "Pending"
                        | "Queued"
                        | "Paused"
                        | "Applying"
                        | "SucceededWithWarnings" => "Warning",
                        "Created" | "Uninitialized" | "Preparing" | "Processing"
                        | "Provisioning" | "Running" | "ApplyingRetention" => "Info",
                        _ => "Neutral",
                    };
                    let label = match status.as_str() {
                        "NotTested" => "Not tested",
                        "TimedOut" => "Timed out",
                        "SucceededWithWarnings" => "Succeeded with warnings",
                        "ApplyingRetention" => "Applying retention",
                        _ => status.as_str(),
                    }
                    .to_owned();
                    GlobalSearchStatus { label, tone }
                });
            groups[index].push(GlobalSearchItem {
                id: item.id,
                resource_type: item.resource_type,
                name: item.name,
                secondary_text: item.secondary_text,
                status,
                parent: item.parent,
            });
        }
        Ok(Self {
            query,
            groups: groups
                .into_iter()
                .zip(CATEGORIES)
                .filter(|(items, _)| !items.is_empty())
                .map(|(items, category)| GlobalSearchGroup { category, items })
                .collect(),
        })
    }
}

pub trait GlobalSearchStore: Send + Sync {
    fn search<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        query: &'a ValidatedSearch,
    ) -> BoxFuture<'a, Result<Vec<GlobalSearchMatch>, ResourceMetadataError>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_bounds_and_normalizes_case_insensitive_unique_types() {
        let query = GlobalSearchQuery {
            q: Some(" redis ".into()),
            types: Some("deployment, Deployment,swarmservice".into()),
            limit_per_type: None,
        }
        .validate()
        .unwrap();
        assert_eq!(query.query, "redis");
        assert_eq!(
            query.resource_types,
            [ResourceType::Deployment, ResourceType::SwarmService]
        );
        for input in [
            GlobalSearchQuery::default(),
            GlobalSearchQuery {
                q: Some("a".into()),
                ..Default::default()
            },
            GlobalSearchQuery {
                q: Some("ab".into()),
                types: Some(",".into()),
                limit_per_type: None,
            },
            GlobalSearchQuery {
                q: Some("ab".into()),
                types: None,
                limit_per_type: Some(11),
            },
        ] {
            assert!(input.validate().is_err());
        }
    }
}

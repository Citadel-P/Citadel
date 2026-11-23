use super::Group;
use crate::realtime::topic::Topic;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CollectionJoin {
    subscription_id: Uuid,
    group: String,
    #[serde(default)]
    tags: Vec<String>,
    platform_id: Option<Uuid>,
    include_disabled: Option<bool>,
}

impl Group {
    /// These catalog readers apply the actor's grants to individual rows in SQL.
    pub(super) fn uses_row_authorization(&self) -> bool {
        matches!(
            self.topic(),
            Topic::Platforms
                | Topic::Deployments
                | Topic::Stacks
                | Topic::ManagedSwarmServices
                | Topic::Registries
                | Topic::GitRepositories
                | Topic::Scripts
                | Topic::BackupPolicies
                | Topic::BuildProjects
                | Topic::BuildAgentPools
        )
    }

    pub(crate) fn from_argument(argument: &serde_json::Value) -> Option<Self> {
        if let Some(name) = argument.as_str() {
            return Self::parse(name);
        }
        let request: CollectionJoin = serde_json::from_value(argument.clone()).ok()?;
        let mut group = Self::parse(&request.group)?;
        if !group.uses_row_authorization() && !matches!(group.topic(), Topic::SwarmServices(_)) {
            return None;
        }
        if request.platform_id.is_some()
            && !matches!(
                group.topic(),
                Topic::Deployments | Topic::Stacks | Topic::ManagedSwarmServices
            )
        {
            return None;
        }
        if request.include_disabled.is_some() && !matches!(group.topic(), Topic::Registries) {
            return None;
        }
        group.include_disabled = request.include_disabled.unwrap_or(false);
        if request.subscription_id.is_nil() {
            return None;
        }
        group.tags = citadel_tags::tag_filter_names(request.tags).ok()?;
        group.platform_filter = request.platform_id;
        group.name = request.subscription_id.to_string();
        group.subscription_id = Some(request.subscription_id);
        Some(group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_typed_join_carries_only_constraints_supported_by_that_reader() {
        let id = Uuid::now_v7();
        let platform = Uuid::now_v7();
        let mut request = json!({
            "subscriptionId": id, "group": "managed-swarm-services",
            "tags": ["Production", " A:+,%_ ", "Production"], "platformId": platform,
        });
        let group = Group::from_argument(&request).unwrap();
        assert_eq!(group.tags, ["A:+,%_", "Production"]);
        assert_eq!(group.platform_filter, Some(platform));
        assert_eq!(group.subscription_id, Some(id));
        request["includeDisabled"] = json!(true);
        assert!(Group::from_argument(&request).is_none());
        request["group"] = json!("registries");
        assert!(Group::from_argument(&request).is_none());
        request.as_object_mut().unwrap().remove("platformId");
        assert!(Group::from_argument(&request).unwrap().include_disabled);
        request["tags"] = json!([" "]);
        assert!(Group::from_argument(&request).is_none());
        request["tags"] = json!([]);
        request["group"] = json!("instance-jobs");
        assert!(Group::from_argument(&request).is_none());
    }
}

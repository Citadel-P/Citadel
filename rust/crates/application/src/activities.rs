use std::sync::Arc;

use chrono::{DateTime, Utc};
use citadel_domain::{ActivityEventType, ActivityResourceType, ActivityStatus, ActorType};
use citadel_identity::{ActorPrincipal, IdentityError};
use futures_util::future::BoxFuture;
use uuid::Uuid;

pub const DEFAULT_ACTIVITY_PAGE_SIZE: i32 = 50;
pub const MAXIMUM_ACTIVITY_PAGE_SIZE: i32 = 500;

pub trait WebhookActivitySink: Send + Sync {
    fn record_webhook(
        &self,
        resource_type: ActivityResourceType,
        id: Uuid,
        details: citadel_domain::WebhookActivityDetails,
    ) -> BoxFuture<'_, Result<(), IdentityError>>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ActivityFilter {
    pub resource_id: Option<Uuid>,
    pub resource_type: Option<ActivityResourceType>,
    pub event_type: Option<ActivityEventType>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

impl ActivityFilter {
    pub fn validated(self) -> Result<ValidatedActivityFilter, IdentityError> {
        let page = self.page.unwrap_or(1);
        let page_size = self.page_size.unwrap_or(DEFAULT_ACTIVITY_PAGE_SIZE);
        if page <= 0 {
            return Err(IdentityError::Validation(
                "Page must be greater than zero.".to_owned(),
            ));
        }
        if !(1..=MAXIMUM_ACTIVITY_PAGE_SIZE).contains(&page_size) {
            return Err(IdentityError::Validation(format!(
                "PageSize must be between 1 and {MAXIMUM_ACTIVITY_PAGE_SIZE}."
            )));
        }
        Ok(ValidatedActivityFilter {
            resource_id: self.resource_id,
            resource_type: self.resource_type,
            event_type: self.event_type,
            page,
            page_size,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedActivityFilter {
    pub resource_id: Option<Uuid>,
    pub resource_type: Option<ActivityResourceType>,
    pub event_type: Option<ActivityEventType>,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone)]
pub struct ActivityRecord {
    pub id: Uuid,
    pub platform_id: Option<Uuid>,
    pub resource_id: Option<Uuid>,
    pub platform_name: String,
    pub resource_name: String,
    pub platform_status: String,
    pub resource_type: ActivityResourceType,
    pub event_type: ActivityEventType,
    pub status: ActivityStatus,
    pub created_at: DateTime<Utc>,
    pub info_json: String,
    pub actor_id: Uuid,
    pub actor_name: String,
    pub actor_type: ActorType,
}

#[derive(Debug, Clone)]
pub struct PagedActivityRecords {
    pub items: Vec<ActivityRecord>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
}

pub trait ActivityQueryStore: Send + Sync {
    fn get_authorized<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Option<ActivityRecord>, IdentityError>>;

    fn list_authorized<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        filter: ValidatedActivityFilter,
    ) -> BoxFuture<'a, Result<PagedActivityRecords, IdentityError>>;
}

pub struct ActivityService {
    store: Arc<dyn ActivityQueryStore>,
}

impl ActivityService {
    #[must_use]
    pub fn new(store: Arc<dyn ActivityQueryStore>) -> Self {
        Self { store }
    }

    pub async fn get(
        &self,
        principal: &ActorPrincipal,
        id: Uuid,
    ) -> Result<ActivityRecord, IdentityError> {
        self.store
            .get_authorized(principal, id)
            .await?
            .ok_or(IdentityError::NotFound)
    }

    pub async fn list(
        &self,
        principal: &ActorPrincipal,
        filter: ActivityFilter,
    ) -> Result<PagedActivityRecords, IdentityError> {
        self.store
            .list_authorized(principal, filter.validated()?)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_paging_is_bounded() {
        assert!(
            ActivityFilter {
                page_size: Some(MAXIMUM_ACTIVITY_PAGE_SIZE),
                ..ActivityFilter::default()
            }
            .validated()
            .is_ok()
        );
        assert!(
            ActivityFilter {
                page_size: Some(MAXIMUM_ACTIVITY_PAGE_SIZE + 1),
                ..ActivityFilter::default()
            }
            .validated()
            .is_err()
        );
    }
}

/// Convert persisted .NET activity property names to the public HTTP/realtime contract.
pub fn public_activity_info(value: serde_json::Value) -> serde_json::Value {
    use serde_json::{Map, Value};
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    let key = camel_case_key(key);
                    // These are user-defined dictionary keys, not DTO property names.
                    let value = if matches!(
                        key.as_str(),
                        "labels" | "environmentVariables" | "buildArguments" | "buildArgs"
                    ) && value.is_object()
                    {
                        value
                    } else {
                        public_activity_info(value)
                    };
                    (key, value)
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(public_activity_info).collect()),
        value => value,
    }
}

/// Resource details embed the same activity info as the activity endpoints.
pub fn public_latest_activity(
    mut activity: Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    if let Some(info) = activity.as_mut().and_then(|value| value.get_mut("info")) {
        *info = public_activity_info(info.take());
    }
    activity
}

fn camel_case_key(mut key: String) -> String {
    if key.starts_with('$') {
        return key;
    }
    let Some(first) = key.get_mut(0..1) else {
        return key;
    };
    first.make_ascii_lowercase();
    key
}

#[cfg(test)]
mod public_activity_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resource_errors_use_the_same_contract_as_activity_http_and_realtime() {
        for event in [
            "DeploymentApplied",
            "StackApplied",
            "StackRollback",
            "GitRepoCloned",
            "GitRepoPulled",
        ] {
            for status in ["Failure", "Warning"] {
                let stored = json!({"status":status,"info":{"$type":event,"Result":{"Message":"Docker or Git operation failed","ResourceBindings":[{"Name":"Port","State":"Missing"}]}}});
                let public = public_latest_activity(Some(stored)).unwrap();
                assert_eq!(
                    public["info"]["result"]["message"],
                    "Docker or Git operation failed"
                );
                assert_eq!(
                    public["info"]["result"]["resourceBindings"][0]["state"],
                    "Missing"
                );
                assert_eq!(public["info"]["$type"], event);
                assert_eq!(public["status"], status);
                assert_eq!(public_latest_activity(Some(public.clone())), Some(public));
            }
        }
        assert_eq!(public_latest_activity(None), None);
    }

    #[test]
    fn degradation_build_errors_and_user_dictionary_keys_survive_mapping() {
        for event in [
            "DeploymentDegraded",
            "StackDegraded",
            "StackDriftDetected",
            "BuildRunFailed",
            "AutomationRunFailed",
        ] {
            let public = public_activity_info(
                json!({"$type":event,"Reason":"Container exited","ErrorMessage":"Process failed","Spec":{"Labels":{"Owner":"OPS"},"EnvironmentVariables":{"PATH":"/bin"}}}),
            );
            assert_eq!(public["reason"], "Container exited");
            assert_eq!(public["errorMessage"], "Process failed");
            assert_eq!(public["spec"]["labels"]["Owner"], "OPS");
            assert_eq!(public["spec"]["environmentVariables"]["PATH"], "/bin");
        }
    }
}

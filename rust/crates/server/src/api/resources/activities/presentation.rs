/// Convert persisted activity property names to the public HTTP/realtime contract.
pub fn public_activity_info(value: serde_json::Value) -> serde_json::Value {
    use citadel_primitives::json_keys::{PropertyCase, map_property_keys};
    map_property_keys(
        value,
        PropertyCase::Camel,
        &[
            "labels",
            "environmentVariables",
            "buildArguments",
            "buildArgs",
        ],
    )
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

pub(super) fn camel_case_key(key: String) -> String {
    citadel_primitives::json_keys::PropertyCase::Camel.key(key)
}

#[cfg(test)]
mod public_activity_tests {
    use crate::api::resources::activities::presentation::*;
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

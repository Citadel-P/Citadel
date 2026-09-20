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

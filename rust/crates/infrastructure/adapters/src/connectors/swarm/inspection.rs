//! Docker inspection is mapped once for Local, Agent-equivalent inspection and adoption.
use crate::connectors::docker::projection::SwarmService;
use crate::connectors::docker::projection::SwarmTask;
use citadel_contracts::citadel::swarm::v1::*;
use citadel_platforms::RuntimeCapabilityError;
use serde_json::Value;

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_owned()
}
fn items(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}
fn strings(value: &Value) -> Vec<String> {
    items(value)
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
fn number(value: &Value) -> Option<i32> {
    value.as_i64().and_then(|n| i32::try_from(n).ok())
}
fn timestamp(value: &str) -> Option<prost_types::Timestamp> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|v| prost_types::Timestamp {
            seconds: v.timestamp(),
            nanos: v.timestamp_subsec_nanos() as i32,
        })
}
fn enum_name(value: &Value, default: &str) -> String {
    match value.as_str().unwrap_or(default) {
        "global" => "Global",
        "replicated" => "Replicated",
        "volume" => "Volume",
        "bind" => "Bind",
        "tmpfs" => "Tmpfs",
        "ingress" => "Ingress",
        "host" => "Host",
        "none" => "None",
        "on-failure" => "OnFailure",
        "any" => "Any",
        "stop-first" => "StopFirst",
        "start-first" => "StartFirst",
        "pause" => "Pause",
        "continue" => "Continue",
        "rollback" => "Rollback",
        other => other,
    }
    .to_owned()
}

pub(crate) fn runtime_hash(spec: &Value) -> String {
    crate::connectors::swarm::runtime_hash::hash(spec)
}

pub(crate) fn inspect_service_message(
    native: SwarmService,
    tasks: &[SwarmTask],
) -> Result<SwarmServiceMessage, RuntimeCapabilityError> {
    let spec = &native.spec;
    let task = &spec["TaskTemplate"];
    let container = &task["ContainerSpec"];
    let mode = if spec["Mode"].get("Global").is_some() {
        "Global"
    } else {
        "Replicated"
    };
    let networks = task
        .get("Networks")
        .or_else(|| spec.get("Networks"))
        .unwrap_or(&Value::Null);
    let network_ids: Vec<_> = items(networks).map(|n| text(n, "Target")).collect();
    let ports: Vec<_> = items(&spec["EndpointSpec"]["Ports"])
        .map(|p| SwarmPortSpecMessage {
            target_port: number(&p["TargetPort"]).unwrap_or_default(),
            published_port: number(&p["PublishedPort"]),
            protocol: p["Protocol"].as_str().unwrap_or("tcp").into(),
            publish_mode: enum_name(&p["PublishMode"], "Ingress"),
        })
        .collect();
    let resources = &task["Resources"];
    let health = &container["Healthcheck"];
    let restart = &task["RestartPolicy"];
    let update = &spec["UpdateConfig"];
    let force_update = task["ForceUpdate"].as_i64().unwrap_or_default();
    let definition = container
        .is_object()
        .then(|| SwarmServiceMutationSpecMessage {
            image: text(container, "Image"),
            scheduling_mode: mode.into(),
            replicas: if mode == "Global" {
                None
            } else {
                number(&spec["Mode"]["Replicated"]["Replicas"]).or(Some(1))
            },
            command: strings(&container["Command"]),
            arguments: strings(&container["Args"]),
            environment: strings(&container["Env"]),
            user: text(container, "User"),
            working_directory: text(container, "Dir"),
            health_check: health.is_object().then(|| SwarmHealthCheckSpecMessage {
                test: strings(&health["Test"]),
                interval_nanoseconds: health["Interval"].as_i64(),
                timeout_nanoseconds: health["Timeout"].as_i64(),
                retries: number(&health["Retries"]),
                start_period_nanoseconds: health["StartPeriod"].as_i64(),
            }),
            stop_grace_period_nanoseconds: container["StopGracePeriod"].as_i64(),
            ports: ports.clone(),
            network_ids: network_ids.clone(),
            mounts: items(&container["Mounts"])
                .map(|m| SwarmMountSpecMessage {
                    kind: enum_name(&m["Type"], "Volume"),
                    source: text(m, "Source"),
                    target: text(m, "Target"),
                    read_only: m["ReadOnly"].as_bool().unwrap_or(false),
                })
                .collect(),
            secrets: items(&container["Secrets"])
                .map(|s| SwarmSecretReferenceSpecMessage {
                    id: text(s, "SecretID"),
                    name: text(s, "SecretName"),
                    target_name: s["File"]["Name"]
                        .as_str()
                        .unwrap_or_else(|| s["SecretName"].as_str().unwrap_or_default())
                        .into(),
                })
                .collect(),
            configs: items(&container["Configs"])
                .map(|s| SwarmConfigReferenceSpecMessage {
                    id: text(s, "ConfigID"),
                    name: text(s, "ConfigName"),
                    target_name: s["File"]["Name"]
                        .as_str()
                        .unwrap_or_else(|| s["ConfigName"].as_str().unwrap_or_default())
                        .into(),
                })
                .collect(),
            resources: resources.is_object().then(|| SwarmResourceSpecMessage {
                limit_nano_cpus: resources["Limits"]["NanoCPUs"].as_i64(),
                limit_memory_bytes: resources["Limits"]["MemoryBytes"].as_i64(),
                reservation_nano_cpus: resources["Reservations"]["NanoCPUs"].as_i64(),
                reservation_memory_bytes: resources["Reservations"]["MemoryBytes"].as_i64(),
            }),
            placement_constraints: strings(&task["Placement"]["Constraints"]),
            restart_policy: restart.is_object().then(|| SwarmRestartPolicySpecMessage {
                condition: enum_name(&restart["Condition"], "Any"),
                delay_nanoseconds: restart["Delay"].as_i64(),
                maximum_attempts: number(&restart["MaxAttempts"]),
                window_nanoseconds: restart["Window"].as_i64(),
            }),
            update_policy: update.is_object().then(|| SwarmUpdatePolicySpecMessage {
                parallelism: number(&update["Parallelism"]).unwrap_or(1),
                delay_nanoseconds: update["Delay"].as_i64(),
                order: enum_name(&update["Order"], "StopFirst"),
                failure_action: enum_name(&update["FailureAction"], "Pause"),
            }),
            force_update: i32::try_from(force_update).unwrap_or(i32::MAX),
        });
    let mut warnings = Vec::new();
    if !spec["RollbackConfig"].is_null() {
        warnings.push("Custom rollback settings are not represented by managed Services.".into());
    }
    if spec["EndpointSpec"]["Mode"]
        .as_str()
        .is_some_and(|m| m != "vip")
    {
        warnings
            .push("DNS round-robin endpoint mode is not represented by managed Services.".into());
    }
    if !task["LogDriver"].is_null() {
        warnings.push("Custom log driver settings are not represented by managed Services.".into());
    }
    let current: Vec<_> = tasks
        .iter()
        .filter(|t| t.service_id == native.id && t.desired_state.eq_ignore_ascii_case("running"))
        .collect();
    let running = current
        .iter()
        .filter(|t| {
            t.status["State"]
                .as_str()
                .is_some_and(|s| s.eq_ignore_ascii_case("running"))
        })
        .count();
    let desired = if mode == "Global" {
        current.len().min(i32::MAX as usize) as i32
    } else {
        number(&spec["Mode"]["Replicated"]["Replicas"]).unwrap_or(1)
    };
    Ok(SwarmServiceMessage {
        id: native.id,
        version_index: native.version.index,
        name: text(spec, "Name"),
        mode: mode.into(),
        image: text(container, "Image"),
        running_task_count: running.min(i32::MAX as usize) as i32,
        desired_task_count: desired,
        update_state: native.update_status["State"]
            .as_str()
            .unwrap_or("None")
            .into(),
        update_message: text(&native.update_status, "Message"),
        ports: ports
            .iter()
            .map(|p| {
                format!(
                    "{}:{}/{}",
                    p.published_port.unwrap_or_default(),
                    p.target_port,
                    p.protocol
                )
            })
            .collect(),
        network_ids,
        secret_ids: items(&container["Secrets"])
            .map(|s| text(s, "SecretID"))
            .collect(),
        config_ids: items(&container["Configs"])
            .map(|s| text(s, "ConfigID"))
            .collect(),
        labels: spec["Labels"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.into())))
            .collect(),
        created_at: timestamp(&native.created_at),
        updated_at: timestamp(&native.updated_at),
        runtime_hash: runtime_hash(spec),
        force_update,
        definition,
        adoption_warnings: warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn inspection_preserves_definition_and_counts_only_current_tasks() {
        let service: SwarmService = serde_json::from_value(json!({
            "ID":"service","Version":{"Index":12},"Spec":{"Name":"redis","Mode":{"Replicated":{"Replicas":1}},
            "TaskTemplate":{"ContainerSpec":{"Image":"redis:latest","Env":["TOKEN=secret"],
            "Healthcheck":{"Test":["CMD","redis-cli","ping"],"Interval":1000},
            "Secrets":[{"SecretID":"secret","SecretName":"password","File":{"Name":"password.txt"}}],
            "Configs":[{"ConfigID":"config","ConfigName":"settings","File":{"Name":"config.yml"}}]},
            "Resources":{"Limits":{"MemoryBytes":268435456}}},"EndpointSpec":{"Ports":[{"TargetPort":6379,"PublishedPort":6379,"PublishMode":"ingress","Protocol":"tcp"}]}}
        })).unwrap();
        let tasks = vec![
            serde_json::from_value(json!({"ID":"current","ServiceID":"service","DesiredState":"running","Status":{"State":"running"}})).unwrap(),
            serde_json::from_value(json!({"ID":"historical","ServiceID":"service","DesiredState":"shutdown","Status":{"State":"running"}})).unwrap(),
            serde_json::from_value(json!({"ID":"other","ServiceID":"another","DesiredState":"running","Status":{"State":"running"}})).unwrap(),
        ];
        let hash = runtime_hash(&service.spec);
        let result = inspect_service_message(service, &tasks).unwrap();
        assert_eq!(result.running_task_count, 1);
        assert_eq!(result.desired_task_count, 1);
        assert_eq!(result.runtime_hash, hash);
        let spec = result.definition.unwrap();
        assert_eq!(spec.environment, ["TOKEN=secret"]);
        assert_eq!(spec.health_check.unwrap().interval_nanoseconds, Some(1000));
        assert_eq!(spec.resources.unwrap().limit_memory_bytes, Some(268435456));
        assert_eq!(spec.secrets[0].target_name, "password.txt");
        assert_eq!(spec.configs[0].target_name, "config.yml");
        assert_eq!(spec.ports[0].publish_mode, "Ingress");
    }
    #[test]
    fn unsupported_runtime_settings_are_reported_in_adoption_review() {
        let service = serde_json::from_value(json!({"Spec":{"RollbackConfig":{},"EndpointSpec":{"Mode":"dnsrr"},"TaskTemplate":{"ContainerSpec":{"Image":"nginx"},"LogDriver":{"Name":"syslog"}}}})).unwrap();
        let result = inspect_service_message(service, &[]).unwrap();
        assert_eq!(result.adoption_warnings.len(), 3);
    }
}

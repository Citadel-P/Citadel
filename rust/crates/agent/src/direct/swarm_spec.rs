use citadel_contracts::citadel::swarm::v1::{
    SwarmServiceMutationSpecMessage, SystemSwarmServiceSpecMessage,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use tonic::Status;
pub(super) fn managed(
    s: SwarmServiceMutationSpecMessage,
    name: String,
    labels: HashMap<String, String>,
    current: Option<Value>,
) -> Result<Value, Status> {
    if s.image.trim().is_empty() || name.trim().is_empty() {
        return Err(Status::invalid_argument(
            "Image and service name are required",
        ));
    }
    let mode = match s.scheduling_mode.to_ascii_lowercase().as_str() {
        "global" => json!({"Global":{}}),
        "replicated" => {
            if s.replicas.is_some_and(|n| n < 0) {
                return Err(Status::invalid_argument("Replicas cannot be negative"));
            }
            json!({"Replicated":{"Replicas":s.replicas.unwrap_or(1)}})
        }
        _ => return Err(Status::invalid_argument("Unsupported scheduling mode")),
    };
    if s.force_update < 0 {
        return Err(Status::invalid_argument("Force update cannot be negative"));
    }
    for p in &s.ports {
        if !(1..=65535).contains(&p.target_port)
            || p.published_port.is_some_and(|v| !(0..=65535).contains(&v))
            || !matches!(
                p.protocol.to_ascii_lowercase().as_str(),
                "tcp" | "udp" | "sctp"
            )
            || !matches!(
                p.publish_mode.to_ascii_lowercase().as_str(),
                "host" | "ingress"
            )
        {
            return Err(Status::invalid_argument("Invalid service port"));
        }
    }
    for m in &s.mounts {
        if !matches!(
            m.kind.to_ascii_lowercase().as_str(),
            "bind" | "volume" | "tmpfs"
        ) || m.target.is_empty()
        {
            return Err(Status::invalid_argument("Invalid service mount"));
        }
    }
    let mut result = current.unwrap_or_else(|| json!({}));
    let mut task = result["TaskTemplate"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let mut container = task
        .get("ContainerSpec")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let desired = json!({"Image":s.image,"Command":s.command,"Args":s.arguments,"Env":s.environment,"User":s.user,"Dir":s.working_directory,"StopGracePeriod":s.stop_grace_period_nanoseconds,
        "Healthcheck":s.health_check.map(|v|json!({"Test":v.test,"Interval":v.interval_nanoseconds,"Timeout":v.timeout_nanoseconds,"Retries":v.retries,"StartPeriod":v.start_period_nanoseconds})),
        "Mounts":s.mounts.into_iter().map(|v|json!({"Type":v.kind.to_ascii_lowercase(),"Source":v.source,"Target":v.target,"ReadOnly":v.read_only})).collect::<Vec<_>>(),
        "Secrets":s.secrets.into_iter().map(|v|json!({"SecretID":v.id,"SecretName":v.name,"File":{"Name":v.target_name,"UID":"0","GID":"0","Mode":292}})).collect::<Vec<_>>(),
        "Configs":s.configs.into_iter().map(|v|json!({"ConfigID":v.id,"ConfigName":v.name,"File":{"Name":v.target_name,"UID":"0","GID":"0","Mode":292}})).collect::<Vec<_>>()});
    container.extend(desired.as_object().expect("object").clone());
    task.insert("ContainerSpec".into(), Value::Object(container));
    task.insert("ForceUpdate".into(), json!(s.force_update));
    task.insert(
        "Networks".into(),
        json!(
            s.network_ids
                .into_iter()
                .map(|v| json!({"Target":v}))
                .collect::<Vec<_>>()
        ),
    );
    task.insert("Resources".into(),json!(s.resources.map(|v|json!({"Limits":{"NanoCPUs":v.limit_nano_cpus,"MemoryBytes":v.limit_memory_bytes},"Reservations":{"NanoCPUs":v.reservation_nano_cpus,"MemoryBytes":v.reservation_memory_bytes}}))));
    let mut placement = task
        .get("Placement")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    placement.insert("Constraints".into(), json!(s.placement_constraints));
    task.insert("Placement".into(), Value::Object(placement));
    task.insert("RestartPolicy".into(),json!(s.restart_policy.map(|v|json!({"Condition":if v.condition.eq_ignore_ascii_case("OnFailure"){"on-failure".into()}else{v.condition.to_ascii_lowercase()},"Delay":v.delay_nanoseconds,"MaxAttempts":v.maximum_attempts,"Window":v.window_nanoseconds}))));
    result["Name"] = json!(name);
    result["Labels"] = json!(labels);
    result["TaskTemplate"] = Value::Object(task);
    result["Mode"] = mode;
    result["UpdateConfig"]=json!(s.update_policy.map(|v|json!({"Parallelism":v.parallelism,"Delay":v.delay_nanoseconds,"Order":if v.order.eq_ignore_ascii_case("StartFirst"){"start-first"}else{"stop-first"},"FailureAction":v.failure_action.to_ascii_lowercase()})));
    result["EndpointSpec"] = json!({"Mode":"vip","Ports":s.ports.into_iter().map(|v|json!({"TargetPort":v.target_port,"PublishedPort":v.published_port,"Protocol":v.protocol.to_ascii_lowercase(),"PublishMode":v.publish_mode.to_ascii_lowercase()})).collect::<Vec<_>>()});
    Ok(result)
}
pub(super) fn system(
    s: SystemSwarmServiceSpecMessage,
    name: String,
    labels: HashMap<String, String>,
    container_labels: HashMap<String, String>,
) -> Result<Value, Status> {
    if name.trim().is_empty()
        || s.image.trim().is_empty()
        || s.manager_node_id.is_empty()
        || s.state_volume_name.is_empty()
        || s.bootstrap_secret_id.is_empty()
    {
        return Err(Status::invalid_argument(
            "Incomplete system service specification",
        ));
    }
    if s.limit_memory_bytes <= 0
        || s.limit_nano_cpus <= 0
        || s.pids_limit <= 0
        || s.stop_grace_period_nanoseconds < 0
    {
        return Err(Status::invalid_argument("Invalid system service limits"));
    }
    let configs = if s.ca_config_id.is_empty() {
        vec![]
    } else {
        vec![
            json!({"ConfigID":s.ca_config_id,"ConfigName":s.ca_config_name,"File":{"Name":"citadel-core-ca.crt","UID":"0","GID":"0","Mode":292}}),
        ]
    };
    Ok(
        json!({"Name":name,"Labels":labels,"Mode":{"Global":{}},"EndpointSpec":{"Mode":"vip","Ports":[]},"Networks":[],
        "TaskTemplate":{"ContainerSpec":{"Image":s.image,"Env":s.environment,"Labels":container_labels,"User":"0","ReadOnly":true,"Init":true,"Privileges":{"NoNewPrivileges":true},"CapabilityDrop":["ALL"],"StopGracePeriod":s.stop_grace_period_nanoseconds,
            "Mounts":[{"Type":"bind","Source":"/var/run/docker.sock","Target":"/var/run/docker.sock","ReadOnly":false},{"Type":"volume","Source":s.state_volume_name,"Target":"/app/data","ReadOnly":false},{"Type":"tmpfs","Target":"/tmp","ReadOnly":false,"TmpfsOptions":{"SizeBytes":67108864,"Mode":448}}],
            "Secrets":[{"SecretID":s.bootstrap_secret_id,"SecretName":s.bootstrap_secret_name,"File":{"Name":"citadel-edge-bootstrap","UID":"0","GID":"0","Mode":256}}],"Configs":configs,
            "Healthcheck":{"Test":["CMD-SHELL","wget -q -O - http://127.0.0.1:9000/health >/dev/null || exit 1"],"Interval":30_000_000_000i64,"Timeout":5_000_000_000i64,"Retries":3,"StartPeriod":10_000_000_000i64}},
            "Resources":{"Limits":{"NanoCPUs":s.limit_nano_cpus,"MemoryBytes":s.limit_memory_bytes,"Pids":s.pids_limit}},"Placement":{"Constraints":["node.platform.os == linux",format!("node.id != {}",s.manager_node_id)],"Platforms":s.supported_architectures.into_iter().map(|a|json!({"OS":"linux","Architecture":a})).collect::<Vec<_>>()},"RestartPolicy":{"Condition":"any","Delay":5_000_000_000i64},"LogDriver":{"Name":"json-file","Options":{"max-size":"10m","max-file":"3"}}},
        "UpdateConfig":{"Parallelism":1,"Delay":0,"FailureAction":"rollback","Monitor":30_000_000_000i64,"MaxFailureRatio":0,"Order":"stop-first"},"RollbackConfig":{"Parallelism":1,"Delay":0,"FailureAction":"pause","Monitor":30_000_000_000i64,"MaxFailureRatio":0,"Order":"stop-first"}}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn update_preserves_unmanaged_fields_and_explicit_zero() {
        let v=managed(SwarmServiceMutationSpecMessage{image:"redis".into(),scheduling_mode:"Replicated".into(),replicas:Some(0),stop_grace_period_nanoseconds:Some(0),..Default::default()},"cache".into(),HashMap::new(),Some(json!({"TaskTemplate":{"ContainerSpec":{"Init":true,"Labels":{"keep":"yes"}},"LogDriver":{"Name":"json-file"}},"RollbackConfig":{"Parallelism":2}}))).unwrap();
        assert_eq!(v["TaskTemplate"]["ContainerSpec"]["Init"], true);
        assert_eq!(v["TaskTemplate"]["ContainerSpec"]["Labels"]["keep"], "yes");
        assert_eq!(v["Mode"]["Replicated"]["Replicas"], 0);
        assert_eq!(v["TaskTemplate"]["ContainerSpec"]["StopGracePeriod"], 0);
        assert_eq!(v["RollbackConfig"]["Parallelism"], 2);
    }
}

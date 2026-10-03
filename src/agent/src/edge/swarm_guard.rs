//! Inspected state checks for operations that cannot be authorized from the request alone.
use super::{dispatch::decode, swarm_policy as policy};
use crate::config::SwarmIdentity;
use citadel_adapters::connectors::docker::DockerClient;
use citadel_contracts::citadel::{
    containers::v1::{CreateContainerRequest, ExecBinaryRequest},
    edge::v1::EdgeCommandKind,
    shared_models::v1::Mount,
    volumes::v1::{CreateVolumeRequest, RemoveVolumeRequest},
};
use prost::Message;
use serde_json::Value;
use tonic::Status;
use uuid::Uuid;

fn denied() -> Status {
    Status::permission_denied(
        "Operation is not permitted for this Swarm-node helper or restore volume",
    )
}
pub(super) async fn authorize(
    docker: &DockerClient,
    node: &SwarmIdentity,
    kind: EdgeCommandKind,
    payload: &mut Vec<u8>,
) -> Result<(), Status> {
    let platform = Uuid::parse_str(&node.platform_id).map_err(|_| denied())?;
    match kind {
        EdgeCommandKind::ContainerCreate => {
            let request: CreateContainerRequest = decode(payload)?;
            if !policy::volume_helper(&request, platform)
                && !policy::backup_helper(&request, platform)
            {
                return Err(denied());
            }
        }
        EdgeCommandKind::ContainerExecBinary => {
            let mut request: ExecBinaryRequest = decode(payload)?;
            let volume = policy::volume_exec(&request);
            let backup = policy::backup_exec(&request);
            if !volume && !backup {
                return Err(denied());
            }
            let document = docker
                .inspect_container_document(&request.container_id)
                .await
                .map_err(|_| denied())?;
            let inspected = helper_configuration(&document)?;
            if !(volume && policy::volume_helper(&inspected, platform)
                || backup && policy::backup_helper(&inspected, platform))
            {
                return Err(denied());
            }
            // Execute by immutable ID; a concurrent rename/replacement must not redirect exec.
            request.container_id = document["Id"]
                .as_str()
                .filter(|v| !v.is_empty())
                .ok_or_else(denied)?
                .into();
            *payload = request.encode_to_vec();
        }
        EdgeCommandKind::VolumeCreate => {
            if !policy::restore_create(&decode::<CreateVolumeRequest>(payload)?, platform) {
                return Err(denied());
            }
        }
        EdgeCommandKind::VolumeDelete => {
            let request: RemoveVolumeRequest = decode(payload)?;
            if request.force || request.names.len() != 1 || request.names[0].trim().is_empty() {
                return Err(denied());
            }
            let volume = docker
                .inspect_volume(&request.names[0])
                .await
                .map_err(|_| denied())?;
            if volume.driver != "local"
                || !volume.options.is_empty()
                || !policy::restore_owner(&volume.labels, platform)
            {
                return Err(denied());
            }
            // Inspect alone may omit UsageData. Include stopped containers in the reference check.
            let filter = serde_json::json!({"volume":[request.names[0]]}).to_string();
            let containers = docker
                .list_container_models(Some(true), None, None, Some(&filter))
                .await
                .map_err(|_| denied())?;
            if !containers.is_empty()
                || volume
                    .usage_data
                    .as_ref()
                    .and_then(|v| v["RefCount"].as_i64())
                    .is_some_and(|v| v > 0)
            {
                return Err(denied());
            }
        }
        _ => {}
    }
    Ok(())
}
fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect()
}
fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}
fn helper_configuration(v: &Value) -> Result<CreateContainerRequest, Status> {
    let c = &v["Config"];
    let h = &v["HostConfig"];
    if !c.is_object()
        || !h.is_object()
        || h["Privileged"].as_bool() != Some(false)
        || ["PidMode", "IpcMode"].iter().any(|key| {
            h[key]
                .as_str()
                .is_some_and(|v| v == "host" || v.starts_with("container:"))
        })
        || ["Devices", "DeviceRequests", "VolumesFrom"]
            .iter()
            .any(|key| h[key].as_array().is_some_and(|v| !v.is_empty()))
    {
        return Err(denied());
    }
    let labels = serde_json::from_value(c["Labels"].clone()).map_err(|_| denied())?;
    let mounts = h["Mounts"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|m| {
            if !m["BindOptions"].is_null() || !m["VolumeOptions"].is_null() {
                return Err(denied());
            }
            Ok(Mount {
                r#type: text(&m["Type"]),
                source: text(&m["Source"]),
                target: text(&m["Target"]),
                read_only: m["ReadOnly"].as_bool(),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, Status>>()?;
    // Docker defaults may add "default" networking for a helper without an explicit mode.
    let network = text(&h["NetworkMode"]).filter(|v| v != "default");
    let restart = match h["RestartPolicy"]["Name"].as_str() {
        None | Some("" | "no") => 0,
        _ => return Err(denied()),
    };
    if h["PortBindings"].as_object().is_some_and(|v| !v.is_empty()) {
        return Err(denied());
    }
    Ok(CreateContainerRequest {
        image_id: text(&v["Image"]).ok_or_else(denied)?,
        name: v["Name"]
            .as_str()
            .ok_or_else(denied)?
            .trim_start_matches('/')
            .into(),
        user: text(&c["User"]),
        working_dir: text(&c["WorkingDir"]),
        labels,
        mounts,
        entry_point: strings(&c["Entrypoint"]),
        command: strings(&c["Cmd"]),
        privileged: h["Privileged"].as_bool(),
        readonly_rootfs: h["ReadonlyRootfs"].as_bool(),
        auto_remove: h["AutoRemove"].as_bool(),
        memory_limit: h["Memory"].as_i64(),
        memory_swap: h["MemorySwap"].as_i64(),
        pids_limit: h["PidsLimit"].as_i64(),
        network_mode: network,
        restart_policy: restart,
        volumes: strings(&h["Binds"]),
        cap_add: strings(&h["CapAdd"]),
        cap_drop: strings(&h["CapDrop"]),
        security_opt: strings(&h["SecurityOpt"]),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inspected_labels_alone_never_authorize_a_privileged_or_reconfigured_helper() {
        let platform = Uuid::now_v7();
        let r = policy::volume_request(platform);
        let good = serde_json::json!({"Id":"immutable-id","Image":"fixture:latest","Name":r.name,
            "Config":{"Labels":r.labels,"User":"0","Entrypoint":r.entry_point,"Cmd":r.command},
            "HostConfig":{"Privileged":false,"ReadonlyRootfs":true,"NetworkMode":"none","CapAdd":r.cap_add,"CapDrop":r.cap_drop,"SecurityOpt":r.security_opt,
                "Mounts":[{"Type":"volume","Source":"fixture","Target":"/data","ReadOnly":true}]}});
        assert!(policy::volume_helper(
            &helper_configuration(&good).unwrap(),
            platform
        ));
        for (pointer, value) in [
            ("/HostConfig/Privileged", serde_json::json!(true)),
            ("/Config/Entrypoint", serde_json::json!(["sh"])),
            ("/HostConfig/Mounts/0/Type", serde_json::json!("bind")),
            ("/HostConfig/Mounts/0/ReadOnly", serde_json::json!(false)),
            (
                "/Config/Labels/com.citadel.platform-id",
                serde_json::json!(Uuid::now_v7().to_string()),
            ),
        ] {
            let mut bad = good.clone();
            *bad.pointer_mut(pointer).unwrap() = value;
            assert!(
                !helper_configuration(&bad)
                    .is_ok_and(|request| policy::volume_helper(&request, platform))
            );
        }
        for (key, value) in [
            ("PidMode", serde_json::json!("host")),
            ("Binds", serde_json::json!(["/:/host"])),
            ("Devices", serde_json::json!([{"PathOnHost":"/dev/sda"}])),
        ] {
            let mut bad = good.clone();
            bad["HostConfig"][key] = value;
            assert!(
                !helper_configuration(&bad)
                    .is_ok_and(|request| policy::volume_helper(&request, platform))
            );
        }
    }
}

//! Wire create options mapped to Docker, preserving explicit optional values.
use citadel_contracts::citadel::{
    containers::v1::CreateContainerRequest,
    shared_models::v1::{EndpointSettings, Mount},
};
use serde_json::{Value, json};
use tonic::Status;

pub(super) fn create(r: &CreateContainerRequest) -> Result<Value, Status> {
    let mut bindings = serde_json::Map::new();
    for port in &r.ports {
        let (host, container) = port
            .split_once('-')
            .or_else(|| port.split_once(':'))
            .ok_or_else(|| {
                Status::invalid_argument("Ports must specify host and container ports")
            })?;
        let host = host
            .parse::<u16>()
            .ok()
            .filter(|p| *p > 0)
            .ok_or_else(|| Status::invalid_argument("Invalid host port"))?;
        let (port, protocol) = container.split_once('/').unwrap_or((container, "tcp"));
        let port = port
            .parse::<u16>()
            .ok()
            .filter(|p| *p > 0)
            .ok_or_else(|| Status::invalid_argument("Invalid container port"))?;
        if !matches!(protocol, "tcp" | "udp" | "sctp") {
            return Err(Status::invalid_argument("Invalid port protocol"));
        }
        let values = bindings
            .entry(format!("{port}/{protocol}"))
            .or_insert_with(|| json!([]));
        values
            .as_array_mut()
            .expect("bindings array")
            .push(json!({"HostPort":host.to_string()}));
    }
    let restart = match r.restart_policy {
        0 => "",
        1 => "no",
        2 => "always",
        3 => "on-failure",
        4 => "unless-stopped",
        _ => return Err(Status::invalid_argument("Unknown restart policy")),
    };
    Ok(json!({
        "Image":r.image_id,"WorkingDir":r.working_dir.as_ref().filter(|v| !v.is_empty()),"User":r.user.as_ref().filter(|v| !v.is_empty()),
        "Labels":r.labels,"Env":r.env_vars,"Entrypoint":r.entry_point,"Cmd":r.command,
        "HostConfig":{
            "PortBindings":bindings,"Binds":r.volumes,"Mounts":r.mounts.iter().map(mount).collect::<Vec<_>>(),
            "Memory":r.memory_limit,"MemorySwap":r.memory_swap,"PidsLimit":r.pids_limit,
            "MemoryReservation":r.memory_reservation.filter(|v| *v!=0),"CpuPeriod":100000,"CpuQuota":r.cpu_quota.filter(|v| *v!=0),
            "AutoRemove":r.auto_remove.unwrap_or(false),"Privileged":r.privileged,"ReadonlyRootfs":r.readonly_rootfs,
            "RestartPolicy":{"Name":restart},"CapAdd":r.cap_add,"CapDrop":r.cap_drop,"SecurityOpt":r.security_opt,"NetworkMode":r.network_mode,
        },
        "NetworkingConfig":{"EndpointsConfig":r.networks.iter().map(|(key,v)| (key.clone(),endpoint(v))).collect::<serde_json::Map<_,_>>()}
    }))
}
fn endpoint(v: &EndpointSettings) -> Value {
    json!({"IPAMConfig":v.ipam_config.as_ref().map(|v| json!({"IPv4Address":v.ipv4_address,"IPv6Address":v.ipv6_address,"LinkLocalIPs":v.link_local_i_ps})),
        "Links":v.links,"MacAddress":v.mac_address,"Aliases":v.aliases,"NetworkID":v.network_id,"EndpointID":v.endpoint_id,
        "Gateway":v.gateway,"IPAddress":v.ip_address,"IPPrefixLen":v.ip_prefix_len,"IPv6Gateway":v.ipv6_gateway,
        "GlobalIPv6Address":v.global_i_pv6_address,"GlobalIPv6PrefixLen":v.global_i_pv6_prefix_len,"DriverOpts":v.driver_opts,"DNSNames":v.dns_names})
}
fn mount(v: &Mount) -> Value {
    json!({"Type":v.r#type,"Source":v.source,"Target":v.target,"ReadOnly":v.read_only,"Consistency":v.consistency,
        "BindOptions":v.bind_options.as_ref().map(|v|json!({"Propagation":v.propagation,"NonRecursive":v.non_recursive,"CreateMountpoint":v.create_mountpoint,"ReadOnlyNonRecursive":v.read_only_non_recursive,"ReadOnlyForceRecursive":v.read_only_force_recursive})),
        "VolumeOptions":v.volume_options.as_ref().map(|v|json!({"NoCopy":v.no_copy,"Labels":v.labels,"Subpath":v.subpath,
            "DriverConfig":v.driver_config.as_ref().map(|v|json!({"Name":v.name,"Options":v.options}))}))})
}

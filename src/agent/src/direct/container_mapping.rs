//! Docker inspection documents mapped explicitly to the public wire contract.
use super::strings;
use citadel_contracts::citadel::shared_models::v1::*;
use serde_json::Value;

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    value
        .as_object()
        .and_then(|v| {
            v.iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, v)| v)
        })
        .unwrap_or(&Value::Null)
}

pub(super) fn state(value: &str) -> i32 {
    match value.to_ascii_lowercase().as_str() {
        "created" => 1,
        "running" => 2,
        "paused" => 3,
        "restarting" => 4,
        "exited" => 5,
        "removing" => 6,
        "dead" => 7,
        _ => 0,
    }
}

pub(super) fn inspect_container_response(v: &Value) -> InspectContainerResponse {
    InspectContainerResponse {
        id: field(v, "id").as_str().unwrap_or_default().to_owned(),
        created: field(v, "created").as_str().unwrap_or_default().to_owned(),
        path: field(v, "path").as_str().map(str::to_owned),
        args: field(v, "args")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        state: (!field(v, "state").is_null()).then(|| container_state(field(v, "state"))),
        image: field(v, "image").as_str().map(str::to_owned),
        resolv_conf_path: field(v, "resolvConfPath").as_str().map(str::to_owned),
        hostname_path: field(v, "hostnamePath").as_str().map(str::to_owned),
        hosts_path: field(v, "hostsPath").as_str().map(str::to_owned),
        log_path: field(v, "logPath").as_str().map(str::to_owned),
        name: field(v, "name").as_str().map(str::to_owned),
        restart_count: field(v, "restartCount")
            .as_i64()
            .and_then(|v| v.try_into().ok()),
        driver: field(v, "driver").as_str().map(str::to_owned),
        platform: field(v, "platform").as_str().map(str::to_owned),
        mount_label: field(v, "mountLabel").as_str().map(str::to_owned),
        process_label: field(v, "processLabel").as_str().map(str::to_owned),
        app_armor_profile: field(v, "appArmorProfile").as_str().map(str::to_owned),
        exec_i_ds: field(v, "execIDs")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        host_config: (!field(v, "hostConfig").is_null())
            .then(|| host_config(field(v, "hostConfig"))),
        graph_driver: (!field(v, "graphDriver").is_null())
            .then(|| graph_driver_data(field(v, "graphDriver"))),
        size_rw: field(v, "sizeRw").as_i64(),
        size_root_fs: field(v, "sizeRootFs").as_i64(),
        mounts: field(v, "mounts")
            .as_array()
            .into_iter()
            .flatten()
            .map(mount_point)
            .collect(),
        config: (!field(v, "config").is_null()).then(|| container_config(field(v, "config"))),
        network_settings: (!field(v, "networkSettings").is_null())
            .then(|| network_settings(field(v, "networkSettings"))),
    }
}

pub(super) fn host_port_binding_list(v: &Value) -> HostPortBindingList {
    HostPortBindingList {
        host_port_binding: v
            .as_array()
            .into_iter()
            .flatten()
            .map(host_port_binding)
            .collect(),
    }
}

pub(super) fn host_port_binding(v: &Value) -> HostPortBinding {
    HostPortBinding {
        host_ip: field(v, "hostIP").as_str().map(str::to_owned),
        host_port: field(v, "hostPort").as_str().map(str::to_owned),
    }
}

pub(super) fn network_settings(v: &Value) -> NetworkSettings {
    NetworkSettings {
        bridge: field(v, "bridge").as_str().unwrap_or_default().to_owned(),
        sandbox_id: field(v, "sandboxID").as_str().map(str::to_owned),
        hairpin_mode: field(v, "hairpinMode").as_bool(),
        link_local_i_pv6_address: field(v, "linkLocalIPv6Address")
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        link_local_i_pv6_prefix_len: field(v, "linkLocalIPv6PrefixLen").as_i64(),
        ports: field(v, "ports")
            .as_object()
            .into_iter()
            .flatten()
            .map(|(key, value)| (key.clone(), host_port_binding_list(value)))
            .collect(),
        sandbox_key: field(v, "sandboxKey").as_str().map(str::to_owned),
        secondary_ip_addresses: field(v, "SecondaryIPAddresses")
            .as_array()
            .into_iter()
            .flatten()
            .map(address)
            .collect(),
        secondary_i_pv6_addresses: field(v, "SecondaryIPv6Addresses")
            .as_array()
            .into_iter()
            .flatten()
            .map(address)
            .collect(),
        endpoint_id: field(v, "endpointID").as_str().map(str::to_owned),
        gateway: field(v, "gateway").as_str().map(str::to_owned),
        global_i_pv6_address: field(v, "globalIPv6Address").as_str().map(str::to_owned),
        global_i_pv6_prefix_len: field(v, "globalIPv6PrefixLen").as_i64(),
        ip_address: field(v, "ipAddress").as_str().map(str::to_owned),
        ip_prefix_len: field(v, "ipPrefixLen").as_i64(),
        ipv6_gateway: field(v, "ipv6Gateway").as_str().map(str::to_owned),
        mac_address: field(v, "macAddress").as_str().map(str::to_owned),
        networks: field(v, "Networks")
            .as_object()
            .into_iter()
            .flatten()
            .map(|(key, value)| MapFieldNetwork {
                key: key.clone(),
                value: Some(endpoint_settings(value)),
            })
            .collect(),
    }
}

pub(super) fn endpoint_settings(v: &Value) -> EndpointSettings {
    EndpointSettings {
        ipam_config: (!field(v, "IPAMConfig").is_null())
            .then(|| endpoint_ipam_config(field(v, "IPAMConfig"))),
        links: field(v, "links")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        mac_address: field(v, "macAddress").as_str().map(str::to_owned),
        aliases: field(v, "aliases")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        network_id: field(v, "networkID").as_str().map(str::to_owned),
        endpoint_id: field(v, "endpointID").as_str().map(str::to_owned),
        gateway: field(v, "gateway").as_str().map(str::to_owned),
        ip_address: field(v, "ipAddress").as_str().map(str::to_owned),
        ip_prefix_len: field(v, "ipPrefixLen")
            .as_i64()
            .and_then(|v| v.try_into().ok()),
        ipv6_gateway: field(v, "ipv6Gateway").as_str().map(str::to_owned),
        global_i_pv6_address: field(v, "globalIPv6Address").as_str().map(str::to_owned),
        global_i_pv6_prefix_len: field(v, "globalIPv6PrefixLen").as_i64(),
        driver_opts: strings(field(v, "driverOpts")),
        dns_names: field(v, "DNSNames")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
    }
}

pub(super) fn endpoint_ipam_config(v: &Value) -> EndpointIpamConfig {
    EndpointIpamConfig {
        ipv4_address: field(v, "ipv4Address").as_str().map(str::to_owned),
        ipv6_address: field(v, "ipv6Address").as_str().map(str::to_owned),
        link_local_i_ps: field(v, "linkLocalIPs")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
    }
}

pub(super) fn address(v: &Value) -> Address {
    Address {
        addr: field(v, "addr").as_str().map(str::to_owned),
        prefix_len: field(v, "prefixLen").as_i64(),
    }
}

pub(super) fn container_config(v: &Value) -> ContainerConfig {
    ContainerConfig {
        hostname: field(v, "hostname").as_str().unwrap_or_default().to_owned(),
        domainname: field(v, "domainname").as_str().map(str::to_owned),
        user: field(v, "user").as_str().map(str::to_owned),
        attach_stdin: field(v, "attachStdin").as_bool(),
        attach_stdout: field(v, "attachStdout").as_bool(),
        attach_stderr: field(v, "attachStderr").as_bool(),
        exposed_ports: field(v, "exposedPorts")
            .as_object()
            .into_iter()
            .flat_map(|v| v.keys().cloned())
            .collect(),
        tty: field(v, "tty").as_bool(),
        open_stdin: field(v, "openStdin").as_bool(),
        stdin_once: field(v, "stdinOnce").as_bool(),
        env: field(v, "env")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        cmd: field(v, "cmd")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        image: field(v, "image").as_str().map(str::to_owned),
        volumes: field(v, "volumes")
            .as_object()
            .into_iter()
            .flat_map(|v| v.keys().cloned())
            .collect(),
        working_dir: field(v, "workingDir")
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        entrypoint: field(v, "entrypoint")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        network_disabled: field(v, "networkDisabled").as_bool(),
        mac_address: field(v, "macAddress").as_str().map(str::to_owned),
        on_build: field(v, "onBuild")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        labels: strings(field(v, "labels")),
        stop_signal: field(v, "stopSignal").as_str().map(str::to_owned),
        stop_timeout: field(v, "stopTimeout")
            .as_i64()
            .and_then(|v| v.try_into().ok()),
    }
}

pub(super) fn mount_point(v: &Value) -> MountPoint {
    MountPoint {
        r#type: field(v, "type").as_str().map(str::to_owned),
        name: field(v, "name").as_str().map(str::to_owned),
        source: field(v, "source").as_str().map(str::to_owned),
        destination: field(v, "destination").as_str().map(str::to_owned),
        driver: field(v, "driver").as_str().map(str::to_owned),
        mode: field(v, "mode").as_str().map(str::to_owned),
        r_w: field(v, "rW").as_bool(),
        propagation: field(v, "propagation").as_str().map(str::to_owned),
    }
}

pub(super) fn graph_driver_data(v: &Value) -> GraphDriverData {
    GraphDriverData {
        name: field(v, "name").as_str().map(str::to_owned),
        data: strings(field(v, "data")),
    }
}

pub(super) fn host_config(v: &Value) -> HostConfig {
    HostConfig {
        binds: field(v, "binds")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        container_id_file: field(v, "containerIDFile").as_str().map(str::to_owned),
        log_config: (!field(v, "logConfig").is_null()).then(|| log_config(field(v, "logConfig"))),
        network_mode: field(v, "networkMode").as_str().map(str::to_owned),
        port_bindings: field(v, "portBindings")
            .as_object()
            .into_iter()
            .flatten()
            .map(|(key, value)| (key.clone(), host_port_binding_list(value)))
            .collect(),
        restart_policy: (!field(v, "restartPolicy").is_null())
            .then(|| restart_policy(field(v, "restartPolicy"))),
        auto_remove: field(v, "autoRemove").as_bool(),
        volume_driver: field(v, "volumeDriver").as_str().map(str::to_owned),
        volumes_from: field(v, "volumesFrom")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        mounts: field(v, "mounts")
            .as_array()
            .into_iter()
            .flatten()
            .map(mount)
            .collect(),
        console_size: field(v, "consoleSize")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_i64)
            .filter_map(|v| v.try_into().ok())
            .collect(),
        annotations: strings(field(v, "annotations")),
        cap_add: field(v, "capAdd")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        cap_drop: field(v, "capDrop")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        cgroupns_mode: field(v, "cgroupnsMode").as_str().map(str::to_owned),
        dns: field(v, "dns")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        dns_options: field(v, "dnsOptions")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        dns_search: field(v, "dnsSearch")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        extra_hosts: field(v, "extraHosts")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        group_add: field(v, "groupAdd")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        ipc_mode: field(v, "ipcMode").as_str().map(str::to_owned),
        cgroup: field(v, "cgroup").as_str().map(str::to_owned),
        links: field(v, "links")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        oom_score_adj: field(v, "oomScoreAdj")
            .as_i64()
            .and_then(|v| v.try_into().ok()),
        pid_mode: field(v, "pidMode").as_str().map(str::to_owned),
        privileged: field(v, "privileged").as_bool(),
        publish_all_ports: field(v, "publishAllPorts").as_bool(),
        readonly_rootfs: field(v, "readonlyRootfs").as_bool(),
        security_opt: field(v, "securityOpt")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        storage_opt: strings(field(v, "storageOpt")),
        tmpfs: strings(field(v, "tmpfs")),
        u_ts_mode: field(v, "uTSMode").as_str().map(str::to_owned),
        userns_mode: field(v, "usernsMode").as_str().map(str::to_owned),
        shm_size: field(v, "shmSize").as_i64().unwrap_or_default(),
        sysctls: strings(field(v, "sysctls")),
        runtime: field(v, "runtime").as_str().map(str::to_owned),
        isolation: field(v, "isolation").as_str().map(str::to_owned),
        masked_paths: field(v, "maskedPaths")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        readonly_paths: field(v, "readonlyPaths")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        memory_swap: field(v, "memorySwap").as_i64(),
        memory_swappiness: field(v, "memorySwappiness").as_i64(),
        nano_cpus: field(v, "nanoCpus").as_i64(),
        pids_limit: field(v, "pidsLimit").as_i64(),
        memory: field(v, "memory").as_i64(),
        memory_reservation: field(v, "memoryReservation").as_i64(),
        io_maximum_bandwidth: field(v, "IOMaximumBandwidth").as_i64(),
        cpu_period: field(v, "cpuPeriod").as_i64(),
        cpu_percent: field(v, "cpuPercent").as_i64(),
        cpu_count: field(v, "CpuCount").as_i64(),
        ulimits: field(v, "ulimits")
            .as_array()
            .into_iter()
            .flatten()
            .map(ulimits)
            .collect(),
        kernel_memory_tcp: field(v, "kernelMemoryTCP").as_i64(),
    }
}

pub(super) fn ulimits(v: &Value) -> Ulimits {
    Ulimits {
        name: field(v, "name").as_str().map(str::to_owned),
        soft: field(v, "soft").as_i64(),
        hard: field(v, "hard").as_i64(),
    }
}

pub(super) fn mount(v: &Value) -> Mount {
    Mount {
        target: field(v, "target").as_str().map(str::to_owned),
        source: field(v, "source").as_str().map(str::to_owned),
        r#type: field(v, "type").as_str().map(str::to_owned),
        read_only: field(v, "readOnly").as_bool(),
        consistency: field(v, "consistency").as_str().map(str::to_owned),
        bind_options: (!field(v, "bindOptions").is_null())
            .then(|| bind_options(field(v, "bindOptions"))),
        volume_options: (!field(v, "volumeOptions").is_null())
            .then(|| volume_options(field(v, "volumeOptions"))),
    }
}

pub(super) fn volume_options(v: &Value) -> VolumeOptions {
    VolumeOptions {
        no_copy: field(v, "noCopy").as_bool(),
        labels: strings(field(v, "labels")),
        driver_config: (!field(v, "driverConfig").is_null())
            .then(|| driver_config(field(v, "driverConfig"))),
        subpath: field(v, "subpath").as_str().map(str::to_owned),
    }
}

pub(super) fn driver_config(v: &Value) -> DriverConfig {
    DriverConfig {
        name: field(v, "name").as_str().map(str::to_owned),
        options: strings(field(v, "options")),
    }
}

pub(super) fn bind_options(v: &Value) -> BindOptions {
    BindOptions {
        propagation: field(v, "propagation").as_str().map(str::to_owned),
        non_recursive: field(v, "nonRecursive").as_bool(),
        create_mountpoint: field(v, "createMountpoint").as_bool(),
        read_only_non_recursive: field(v, "readOnlyNonRecursive").as_bool(),
        read_only_force_recursive: field(v, "readOnlyForceRecursive").as_bool(),
    }
}

pub(super) fn restart_policy(v: &Value) -> RestartPolicy {
    RestartPolicy {
        name: field(v, "Name").as_str().map(str::to_owned),
        maximum_retry_count: field(v, "maximumRetryCount")
            .as_i64()
            .and_then(|v| v.try_into().ok()),
    }
}

pub(super) fn container_state(v: &Value) -> ContainerState {
    ContainerState {
        status: state(field(v, "status").as_str().unwrap_or_default()),
        running: field(v, "running").as_bool(),
        paused: field(v, "paused").as_bool(),
        restarting: field(v, "restarting").as_bool(),
        oom_killed: field(v, "OOMKilled").as_bool(),
        dead: field(v, "dead").as_bool(),
        pid: field(v, "pid").as_i64().and_then(|v| v.try_into().ok()),
        exit_code: field(v, "exitCode")
            .as_i64()
            .and_then(|v| v.try_into().ok()),
        error: field(v, "error").as_str().map(str::to_owned),
        started_at: field(v, "startedAt").as_str().map(str::to_owned),
        finished_at: field(v, "finishedAt").as_str().map(str::to_owned),
        health: (!field(v, "health").is_null()).then(|| health(field(v, "health"))),
    }
}

pub(super) fn log_config(v: &Value) -> LogConfig {
    LogConfig {
        r#type: field(v, "type").as_str().map(str::to_owned),
        config: strings(field(v, "config")),
    }
}

pub(super) fn health(v: &Value) -> Health {
    Health {
        status: field(v, "status").as_str().map(str::to_owned),
        failing_streak: field(v, "failingStreak")
            .as_i64()
            .and_then(|v| v.try_into().ok()),
    }
}

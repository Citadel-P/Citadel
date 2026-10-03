//! Exhaustive protobuf dispatch over the same operations used by Direct RPCs.
use super::commands::Output;
use crate::direct::Runtime;
use citadel_contracts::citadel::{
    containers::v1::{ExecClientMessage, container_service_server::ContainerService},
    deployments::v1::deployment_service_server::DeploymentService,
    edge::v1::EdgeCommandKind,
    images::v1::image_service_server::ImageService,
    networks::v1::network_service_server::NetworkService,
    platforms::v1::platform_service_server::PlatformService,
    stacks::v1::stack_service_server::StackService,
    swarm::v1::swarm_service_server::SwarmService,
    volumes::v1::volume_service_server::VolumeService,
};
use futures_util::{StreamExt, stream::BoxStream};
use prost::Message;
use tonic::{Request, Status};

pub(super) fn decode<T: Message + Default>(payload: &[u8]) -> Result<T, Status> {
    T::decode(payload)
        .map_err(|_| Status::invalid_argument("Invalid Edge command protobuf payload"))
}

pub(super) async fn execute(
    runtime: Runtime,
    kind: EdgeCommandKind,
    payload: &[u8],
    input: BoxStream<'static, Result<ExecClientMessage, Status>>,
    output: Output,
) -> Result<(), Status> {
    macro_rules! unary {
        ($service:ident, $method:ident) => {{
            let response = $service::$method(&runtime, Request::new(decode(payload)?)).await?;
            output.message(response.into_inner()).await
        }};
    }
    macro_rules! stream {
        ($service:ident, $method:ident) => {{
            let response = $service::$method(&runtime, Request::new(decode(payload)?)).await?;
            let mut stream = response.into_inner();
            while let Some(message) = stream.next().await {
                output.message(message?).await?;
            }
            Ok(())
        }};
    }
    use EdgeCommandKind::*;
    // No wildcard: a new protocol enum variant must acquire an explicit implementation.
    match kind {
        Unspecified => Err(Status::invalid_argument("An Edge command kind is required")),
        PlatformCheckHealth => unary!(PlatformService, check_health),
        PlatformGetInfo => unary!(PlatformService, get_platform_info),
        PlatformStatsStream => stream!(PlatformService, stream_platform_stats),
        PlatformDaemonEventsStream => stream!(PlatformService, stream_daemon_event),
        PlatformPrune => unary!(PlatformService, prune),
        ContainerList => unary!(ContainerService, list),
        ContainerLogsStream => stream!(ContainerService, stream_container_logs),
        ContainerInspect => unary!(ContainerService, inspect),
        ContainerCreate => unary!(ContainerService, create),
        ContainerStart => unary!(ContainerService, start),
        ContainerStop => unary!(ContainerService, stop),
        ContainerPause => unary!(ContainerService, pause),
        ContainerUnpause => unary!(ContainerService, unpause),
        ContainerRestart => unary!(ContainerService, restart),
        ContainerDelete => unary!(ContainerService, delete),
        ContainerStatsStream => stream!(ContainerService, stream_container_stats),
        ContainersStatsStream => stream!(ContainerService, stream_containers_stats),
        ContainerExec => {
            let opening = decode(payload)?;
            let incoming = futures_util::stream::once(async move { Ok(opening) }).chain(input);
            let mut stream = runtime
                .exec_messages(Box::pin(incoming))
                .await?
                .into_inner();
            while let Some(message) = stream.next().await {
                output.message(message?).await?;
            }
            Ok(())
        }
        ContainerExecBinary => stream!(ContainerService, exec_binary),
        ImageGet => unary!(ImageService, get),
        ImageList => unary!(ImageService, list),
        ImageInspect => unary!(ImageService, inspect),
        ImageDelete => unary!(ImageService, delete),
        ImageHistory => unary!(ImageService, history),
        ImageExposedPorts => unary!(ImageService, get_exposed_ports),
        ImageDistributionInspect => unary!(ImageService, distribution_inspect),
        ImagePullStream => stream!(ImageService, pull),
        ImageBuildStream => stream!(ImageService, build),
        ImagePushStream => stream!(ImageService, push),
        ImageCheckBuildHost => unary!(ImageService, check_build_host),
        VolumeList => unary!(VolumeService, list),
        VolumeInspect => unary!(VolumeService, inspect),
        VolumeCreate => unary!(VolumeService, create),
        VolumeDelete => unary!(VolumeService, remove),
        NetworkList => unary!(NetworkService, list),
        NetworkInspect => unary!(NetworkService, inspect),
        NetworkCreate => unary!(NetworkService, create),
        NetworkDelete => unary!(NetworkService, delete),
        StackApplyStream => stream!(StackService, apply),
        DeploymentApply => unary!(DeploymentService, apply),
        SwarmNodeList => unary!(SwarmService, list_nodes),
        SwarmNodeInspect => unary!(SwarmService, inspect_node),
        SwarmNodeUpdate => unary!(SwarmService, update_node),
        SwarmServiceList => unary!(SwarmService, list_services),
        SwarmServiceInspect => unary!(SwarmService, inspect_service),
        SwarmServiceCreate => unary!(SwarmService, create_service),
        SwarmServiceUpdate => unary!(SwarmService, update_service),
        SwarmServiceDelete => unary!(SwarmService, delete_service),
        SwarmServiceRestart => unary!(SwarmService, restart_service),
        SwarmServiceLogs => unary!(SwarmService, get_service_logs),
        SwarmTaskList => unary!(SwarmService, list_tasks),
        SwarmTaskInspect => unary!(SwarmService, inspect_task),
        SwarmTaskLogs => unary!(SwarmService, get_task_logs),
        SwarmNetworkList => unary!(SwarmService, list_networks),
        SwarmNetworkInspect => unary!(SwarmService, inspect_network),
        SwarmSecretList => unary!(SwarmService, list_secrets),
        SwarmSecretInspect => unary!(SwarmService, inspect_secret),
        SwarmSecretCreate => unary!(SwarmService, create_secret),
        SwarmSecretUpdate => unary!(SwarmService, update_secret_labels),
        SwarmSecretDelete => unary!(SwarmService, delete_secret),
        SwarmConfigList => unary!(SwarmService, list_configs),
        SwarmConfigInspect => unary!(SwarmService, inspect_config),
        SwarmConfigData => unary!(SwarmService, get_config_data),
        SwarmConfigCreate => unary!(SwarmService, create_config),
        SwarmConfigUpdate => unary!(SwarmService, update_config_labels),
        SwarmConfigDelete => unary!(SwarmService, delete_config),
        SwarmSystemServiceCreate => unary!(SwarmService, create_system_service),
        SwarmSystemServiceUpdate => unary!(SwarmService, update_system_service),
    }
}

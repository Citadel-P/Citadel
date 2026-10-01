use super::runtime::Runtime;
use citadel_platforms::*;
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;
impl NetworkInventoryPort for Runtime<'_> {
    fn list_network_topology<'a>(
        &'a self,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeNetworkSummary>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => NetworkInventoryPort::list_network_topology(*r, cancel),
            Runtime::Agent(r) => NetworkInventoryPort::list_network_topology(r, cancel),
            Runtime::Edge(r) => NetworkInventoryPort::list_network_topology(r, cancel),
        }
    }

    fn list_networks<'a>(
        &'a self,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeNetworkSummary>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => NetworkInventoryPort::list_networks(*runtime, cancel),
            Runtime::Agent(runtime) => NetworkInventoryPort::list_networks(runtime, cancel),
            Runtime::Edge(runtime) => NetworkInventoryPort::list_networks(runtime, cancel),
        }
    }
}
impl VolumeInventoryPort for Runtime<'_> {
    fn list_volumes<'a>(
        &'a self,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeVolumeSummary>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => VolumeInventoryPort::list_volumes(*runtime, cancel),
            Runtime::Agent(runtime) => VolumeInventoryPort::list_volumes(runtime, cancel),
            Runtime::Edge(runtime) => VolumeInventoryPort::list_volumes(runtime, cancel),
        }
    }
}
impl SwarmTaskRuntimePort for Runtime<'_> {
    fn inspect_task<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeSwarmTask, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => SwarmTaskRuntimePort::inspect_task(*runtime, id, cancel),
            Runtime::Agent(runtime) => SwarmTaskRuntimePort::inspect_task(runtime, id, cancel),
            Runtime::Edge(runtime) => SwarmTaskRuntimePort::inspect_task(runtime, id, cancel),
        }
    }
}

impl NetworkObservationPort for Runtime<'_> {
    fn inspect_network<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeNetworkSummary, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => {
                NetworkObservationPort::inspect_network(*runtime, id, cancel)
            }
            Runtime::Agent(runtime) => NetworkObservationPort::inspect_network(runtime, id, cancel),
            Runtime::Edge(runtime) => NetworkObservationPort::inspect_network(runtime, id, cancel),
        }
    }
}

impl VolumeObservationPort for Runtime<'_> {
    fn inspect_volume<'a>(
        &'a self,
        name: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => {
                VolumeObservationPort::inspect_volume(*runtime, name, cancel)
            }
            Runtime::Agent(runtime) => VolumeObservationPort::inspect_volume(runtime, name, cancel),
            Runtime::Edge(runtime) => VolumeObservationPort::inspect_volume(runtime, name, cancel),
        }
    }
}

impl NetworkMutationPort for Runtime<'_> {
    fn create_network<'a>(
        &'a self,
        input: &'a CreateRuntimeNetwork,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<CreatedRuntimeNetwork, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => NetworkMutationPort::create_network(*runtime, input, cancel),
            Runtime::Agent(runtime) => NetworkMutationPort::create_network(runtime, input, cancel),
            Runtime::Edge(runtime) => NetworkMutationPort::create_network(runtime, input, cancel),
        }
    }

    fn delete_network<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => NetworkMutationPort::delete_network(*runtime, id, cancel),
            Runtime::Agent(runtime) => NetworkMutationPort::delete_network(runtime, id, cancel),
            Runtime::Edge(runtime) => NetworkMutationPort::delete_network(runtime, id, cancel),
        }
    }
}

impl VolumeMutationPort for Runtime<'_> {
    fn create_volume<'a>(
        &'a self,
        input: &'a CreateRuntimeVolume,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => VolumeMutationPort::create_volume(*runtime, input, cancel),
            Runtime::Agent(runtime) => VolumeMutationPort::create_volume(runtime, input, cancel),
            Runtime::Edge(runtime) => VolumeMutationPort::create_volume(runtime, input, cancel),
        }
    }

    fn delete_volume<'a>(
        &'a self,
        name: &'a str,
        force: bool,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => {
                VolumeMutationPort::delete_volume(*runtime, name, force, cancel)
            }
            Runtime::Agent(runtime) => {
                VolumeMutationPort::delete_volume(runtime, name, force, cancel)
            }
            Runtime::Edge(runtime) => {
                VolumeMutationPort::delete_volume(runtime, name, force, cancel)
            }
        }
    }
}

impl PlatformInfoPort for Runtime<'_> {
    fn get_info<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimePlatformInfo, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => PlatformInfoPort::get_info(*r, cancellation),
            Runtime::Agent(r) => PlatformInfoPort::get_info(r, cancellation),
            Runtime::Edge(r) => PlatformInfoPort::get_info(r, cancellation),
        }
    }
}

impl SwarmInventoryPort for Runtime<'_> {
    fn list_swarm_nodes<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmNode>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => SwarmInventoryPort::list_swarm_nodes(*r, cancellation),
            Runtime::Agent(r) => SwarmInventoryPort::list_swarm_nodes(r, cancellation),
            Runtime::Edge(r) => SwarmInventoryPort::list_swarm_nodes(r, cancellation),
        }
    }
    fn list_swarm_services<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmService>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => SwarmInventoryPort::list_swarm_services(*r, cancellation),
            Runtime::Agent(r) => SwarmInventoryPort::list_swarm_services(r, cancellation),
            Runtime::Edge(r) => SwarmInventoryPort::list_swarm_services(r, cancellation),
        }
    }
    fn list_swarm_tasks<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmTask>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => SwarmInventoryPort::list_swarm_tasks(*r, cancellation),
            Runtime::Agent(r) => SwarmInventoryPort::list_swarm_tasks(r, cancellation),
            Runtime::Edge(r) => SwarmInventoryPort::list_swarm_tasks(r, cancellation),
        }
    }
    fn list_swarm_configs<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmConfig>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => SwarmInventoryPort::list_swarm_configs(*r, cancellation),
            Runtime::Agent(r) => SwarmInventoryPort::list_swarm_configs(r, cancellation),
            Runtime::Edge(r) => SwarmInventoryPort::list_swarm_configs(r, cancellation),
        }
    }
    fn list_swarm_secrets<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmSecret>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(r) => SwarmInventoryPort::list_swarm_secrets(*r, cancellation),
            Runtime::Agent(r) => SwarmInventoryPort::list_swarm_secrets(r, cancellation),
            Runtime::Edge(r) => SwarmInventoryPort::list_swarm_secrets(r, cancellation),
        }
    }
}

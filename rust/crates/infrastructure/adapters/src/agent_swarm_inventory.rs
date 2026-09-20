use super::*;
use citadel_contracts::citadel::swarm::v1::*;

// Every write is sent exactly once: retrying an ambiguous create/restart could
// duplicate a resource or restart an already-restarted Service.
macro_rules! operation {
    ($method:ident, $rpc:ident, $wire:literal, $input:ty, $output:ty) => {
        pub(crate) async fn $method(&self, input: $input, cancel: &CancellationToken) -> Result<$output, RuntimeCapabilityError> {
            let request = self.signer.sign(input, concat!("/citadel.swarm.v1.SwarmService/", $wire), Some(self.operation_timeout))?;
            let mut client = self.swarm_client();
            tokio::select! {
                biased;
                () = cancel.cancelled() => Err(cancelled_error()),
                result = tokio::time::timeout(self.operation_timeout, client.$rpc(request)) => {
                    result.map_err(|_| timeout_error("operating on Swarm inventory"))?.map(|r| r.into_inner()).map_err(normalize_status)
                }
            }
        }
    };
}
impl AgentClient {
    operation!(
        swarm_inspect_node,
        inspect_node,
        "InspectNode",
        InspectSwarmNodeRequest,
        SwarmNodeMessage
    );
    operation!(
        swarm_update_node,
        update_node,
        "UpdateNode",
        UpdateSwarmNodeRequest,
        ()
    );
    operation!(
        swarm_restart_service,
        restart_service,
        "RestartService",
        RestartSwarmServiceRequest,
        ()
    );
    operation!(
        swarm_create_secret,
        create_secret,
        "CreateSecret",
        CreateSwarmSecretRequest,
        SwarmResourceCreateResponse
    );
    operation!(
        swarm_create_config,
        create_config,
        "CreateConfig",
        CreateSwarmConfigRequest,
        SwarmResourceCreateResponse
    );
    operation!(
        swarm_update_secret,
        update_secret_labels,
        "UpdateSecretLabels",
        UpdateSwarmResourceLabelsRequest,
        ()
    );
    operation!(
        swarm_update_config,
        update_config_labels,
        "UpdateConfigLabels",
        UpdateSwarmResourceLabelsRequest,
        ()
    );
    operation!(
        swarm_delete_secret,
        delete_secret,
        "DeleteSecret",
        DeleteSwarmSecretRequest,
        ()
    );
    operation!(
        swarm_delete_config,
        delete_config,
        "DeleteConfig",
        DeleteSwarmConfigRequest,
        ()
    );
    operation!(
        swarm_config_data,
        get_config_data,
        "GetConfigData",
        InspectSwarmConfigRequest,
        SwarmConfigDataResponse
    );
}

use super::*;
use citadel_contracts::citadel::swarm::v1::{
    DeleteSwarmConfigRequest, DeleteSwarmSecretRequest, InspectSwarmConfigRequest,
    InspectSwarmSecretRequest,
};
use citadel_contracts::citadel::{
    images::v1::{DistributionInspectRequest, DistributionInspectResponse},
    swarm::v1::*,
};
use citadel_platforms::node_agents::lifecycle::NodeAgentResource;
use uuid::Uuid;

macro_rules! setup_rpc {
    ($name:ident,$client:ident,$method:ident,$request:ty,$response:ty,$path:literal) => {
        impl AgentClient {
            pub(crate) async fn $name(&self,input:$request,cancel:&CancellationToken)->Result<$response,RuntimeCapabilityError> {
                if cancel.is_cancelled(){return Err(cancelled_error());}
                let mut client=self.$client();
                let request=self.signer.sign(input,$path,Some(self.operation_timeout))?;
                tokio::select! {()=cancel.cancelled()=>Err(cancelled_error()),result=tokio::time::timeout(self.operation_timeout,client.$method(request))=>result.map_err(|_|timeout_error("managing node agents"))?.map(|r|r.into_inner()).map_err(normalize_status)}
            }
        }
    }
}
setup_rpc!(
    node_agent_distribution,
    image_client,
    distribution_inspect,
    DistributionInspectRequest,
    DistributionInspectResponse,
    "/citadel.images.v1.ImageService/DistributionInspect"
);
setup_rpc!(
    node_agent_secret,
    swarm_client,
    create_secret,
    CreateSwarmSecretRequest,
    SwarmResourceCreateResponse,
    "/citadel.swarm.v1.SwarmService/CreateSecret"
);
setup_rpc!(
    node_agent_config,
    swarm_client,
    create_config,
    CreateSwarmConfigRequest,
    SwarmResourceCreateResponse,
    "/citadel.swarm.v1.SwarmService/CreateConfig"
);
setup_rpc!(
    node_agent_create,
    swarm_client,
    create_system_service,
    CreateSystemSwarmServiceRequest,
    SwarmServiceMutationResponse,
    "/citadel.swarm.v1.SwarmService/CreateSystemService"
);
setup_rpc!(
    node_agent_update,
    swarm_client,
    update_system_service,
    UpdateSystemSwarmServiceRequest,
    SwarmServiceMutationResponse,
    "/citadel.swarm.v1.SwarmService/UpdateSystemService"
);

impl AgentClient {
    pub(crate) async fn inspect_node_agent_resource(
        &self,
        kind: NodeAgentResource,
        id: &str,
        cancellation: &CancellationToken,
    ) -> Result<std::collections::BTreeMap<String, String>, RuntimeCapabilityError> {
        let mut client = self.swarm_client();
        let request = async {
            let labels = match kind {
                NodeAgentResource::Secret => {
                    client
                        .inspect_secret(self.signer.sign(
                            InspectSwarmSecretRequest {
                                secret_id: id.into(),
                            },
                            "/citadel.swarm.v1.SwarmService/InspectSecret",
                            Some(self.operation_timeout),
                        )?)
                        .await
                        .map_err(normalize_status)?
                        .into_inner()
                        .labels
                }
                NodeAgentResource::Config => {
                    client
                        .inspect_config(self.signer.sign(
                            InspectSwarmConfigRequest {
                                config_id: id.into(),
                            },
                            "/citadel.swarm.v1.SwarmService/InspectConfig",
                            Some(self.operation_timeout),
                        )?)
                        .await
                        .map_err(normalize_status)?
                        .into_inner()
                        .labels
                }
                NodeAgentResource::Service => {
                    return self
                        .inspect_managed_swarm_service(id, cancellation)
                        .await
                        .map(|s| s.labels.into_iter().collect());
                }
            };
            Ok(labels.into_iter().collect())
        };
        tokio::select! { ()=cancellation.cancelled()=>Err(cancelled_error()),
        result=tokio::time::timeout(self.operation_timeout,request)=>result.map_err(|_|timeout_error("inspecting node-agent infrastructure"))? }
    }
    pub(crate) async fn delete_node_agent_resource(
        &self,
        kind: NodeAgentResource,
        id: &str,
        operation_id: Uuid,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let mut client = self.swarm_client();
        let request = async {
            match kind {
                NodeAgentResource::Secret => {
                    client
                        .delete_secret(self.signer.sign(
                            DeleteSwarmSecretRequest {
                                secret_id: id.into(),
                            },
                            "/citadel.swarm.v1.SwarmService/DeleteSecret",
                            Some(self.operation_timeout),
                        )?)
                        .await
                        .map_err(normalize_status)?;
                }
                NodeAgentResource::Config => {
                    client
                        .delete_config(self.signer.sign(
                            DeleteSwarmConfigRequest {
                                config_id: id.into(),
                            },
                            "/citadel.swarm.v1.SwarmService/DeleteConfig",
                            Some(self.operation_timeout),
                        )?)
                        .await
                        .map_err(normalize_status)?;
                }
                NodeAgentResource::Service => {
                    return self
                        .delete_managed_swarm_service(
                            DeleteManagedSwarmServiceRequest {
                                service_id: id.into(),
                                operation_id: operation_id.to_string(),
                            },
                            cancellation,
                        )
                        .await;
                }
            }
            Ok(())
        };
        tokio::select! { ()=cancellation.cancelled()=>Err(cancelled_error()), result=tokio::time::timeout(self.operation_timeout,request)=>result.map_err(|_|timeout_error("removing node-agent infrastructure"))? }
    }
}

//! Volume inventory requests and Docker-shaped cluster volume mappings.
use super::{
    AgentClient, INSPECT_VOLUME_METHOD, LIST_VOLUMES_METHOD, ListVolumesRequest,
    RuntimeCapabilityError, RuntimeVolumeSummary, map_volume, normalize_status,
};
use citadel_contracts::citadel::shared_models::v1::ClusterVolumeMessage;
use citadel_contracts::citadel::volumes::v1::{InspectVolumeRequest, ListVolumesResponse};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

pub(super) fn cluster(v: ClusterVolumeMessage) -> Value {
    json!({
        "ID": v.id, "CreatedAt": v.created_at, "UpdatedAt": v.updated_at,
        "Version": v.version.map(|v| json!({"Index": v.index})),
        "Spec": v.spec.map(|v| json!({
            "Group": v.group,
            "AccessMode": v.access_mode.map(|a| json!({
                "Scope": match a.scope { 1 => "multi", _ => "single" },
                "Sharing": match a.sharing { 1 => "readonly", 2 => "onewriter", 3 => "all", _ => "none" },
                "Availability": a.availability,
                "Secrets": a.secrets.into_iter().map(|s| json!({"Key": s.key, "Secret": s.secret})).collect::<Vec<_>>(),
                "CapacityRange": a.capacity_range.map(|c| json!({"RequiredBytes": c.required_bytes, "LimitBytes": c.limit_bytes})),
            })),
        })),
        "Info": v.info.map(|i| json!({
            "CapacityBytes": i.capacity_bytes, "VolumeContext": i.volume_context, "VolumeID": i.volume_id,
            "AccessibleTopology": i.accessible_topology.into_iter().map(|t| t.labels).collect::<Vec<_>>(),
        })),
        "PublishStatus": v.publish_status.into_iter().map(|p| json!({
            "NodeID": p.node_id, "State": p.state, "PublishContext": p.publish_context,
        })).collect::<Vec<_>>(),
    })
}

impl AgentClient {
    async fn inspect_volume_once(
        &self,
        name: &str,
        observation: citadel_platforms::VolumeObservation,
    ) -> Result<RuntimeVolumeSummary, RuntimeCapabilityError> {
        let request = self.signer.sign(
            InspectVolumeRequest {
                name: name.to_owned(),
                observation: super::volume_observation(observation),
            },
            INSPECT_VOLUME_METHOD,
            Some(self.operation_timeout),
        )?;
        self.volume_client()
            .inspect(request)
            .await
            .map(|value| map_volume(value.into_inner()))
            .map_err(normalize_status)
    }

    pub(crate) async fn named_volume_metadata(
        &self,
        name: &str,
        cancellation: &CancellationToken,
    ) -> Result<RuntimeVolumeSummary, RuntimeCapabilityError> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(super::cancelled_error()),
            result = self.inspect_volume_once(name, citadel_platforms::VolumeObservation::Metadata) => result,
        }
    }
    async fn volume_list(
        &self,
        names_only: bool,
        cancellation: &CancellationToken,
    ) -> Result<ListVolumesResponse, RuntimeCapabilityError> {
        self.retry_unary(cancellation, || async {
            let request = self.signer.sign(
                ListVolumesRequest {
                    names_only,
                    ..Default::default()
                },
                LIST_VOLUMES_METHOD,
                Some(self.operation_timeout),
            )?;
            self.volume_client()
                .list(request)
                .await
                .map(|v| v.into_inner())
                .map_err(normalize_status)
        })
        .await
    }
}
impl citadel_platforms::VolumeInventoryPort for AgentClient {
    fn list_volume_names<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>> {
        Box::pin(async move {
            Ok(self
                .volume_list(true, cancellation)
                .await?
                .volumes
                .into_iter()
                .map(|v| v.name)
                .collect())
        })
    }
    fn list_volumes<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeVolumeSummary>, RuntimeCapabilityError>> {
        Box::pin(async move {
            Ok(self
                .volume_list(false, cancellation)
                .await?
                .volumes
                .into_iter()
                .map(map_volume)
                .collect())
        })
    }
}

impl citadel_platforms::VolumeObservationPort for AgentClient {
    fn observe_volume<'a>(
        &'a self,
        name: &'a str,
        observation: citadel_platforms::VolumeObservation,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.retry_unary(cancellation, || self.inspect_volume_once(name, observation))
                .await
        })
    }
}

use crate::{
    CreatePlatformInput, PlatformConnectorType, PlatformRegistrationError, PlatformType,
    PlatformView, RuntimePlatformInfo,
};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePlatformInput {
    pub id: Uuid,
    pub name: String,
}

pub fn patch_input(
    current: &PlatformView,
    patch: &Value,
) -> Result<CreatePlatformInput, PlatformRegistrationError> {
    let patch = patch
        .as_object()
        .ok_or_else(|| invalid("The Platform patch must be an object."))?;
    let mut value = serde_json::json!({"name":current.name,"address":current.address,"description":current.description,"type":current.platform_type,"connectorType":current.connector_type,"pruneHistoricalSwarmTaskContainers":current.prune_historical_swarm_task_containers});
    let object = value.as_object_mut().expect("object");
    for field in [
        "name",
        "address",
        "description",
        "type",
        "connectorType",
        "pruneHistoricalSwarmTaskContainers",
    ] {
        if let Some(next) = patch.get(field) {
            object.insert(field.into(), next.clone());
        }
    }
    let input: CreatePlatformInput = serde_path_to_error::deserialize(value)
        .map_err(|error| invalid(&format!("Invalid Platform patch: {error}")))?;
    let input = crate::registration::validate(input)?;
    if input.platform_type.as_str() != current.platform_type {
        return Err(invalid(
            "Platform orchestration type cannot change after creation.",
        ));
    }
    if input.connector_type == PlatformConnectorType::Unknown {
        return Err(invalid("A supported connector is required."));
    }
    if (input.connector_type == PlatformConnectorType::EdgeAgent)
        != (current.connector_type == "EdgeAgent")
    {
        return Err(invalid(
            "Edge enrollment cannot be changed into a different connector type.",
        ));
    }
    if input.connector_type == PlatformConnectorType::EdgeAgent
        && input.address.as_deref() != Some(current.address.as_str())
    {
        return Err(invalid("An Edge enrollment address cannot be changed."));
    }
    Ok(input)
}

pub fn validate_target(
    current: &PlatformView,
    info: &RuntimePlatformInfo,
) -> Result<(), PlatformRegistrationError> {
    let kind = if current.platform_type == "DockerSwarm" {
        PlatformType::DockerSwarm
    } else {
        PlatformType::Docker
    };
    crate::registration::validate_platform_type(kind, info.swarm.as_ref())?;
    if info.daemon_id.trim().is_empty() {
        return Err(invalid("Docker Engine did not report a daemon ID."));
    }
    if kind == PlatformType::DockerSwarm
        && current.cluster_id.as_ref().is_some_and(|expected| {
            info.swarm.as_ref().and_then(|s| s.cluster_id.as_ref()) != Some(expected)
        })
    {
        return Err(invalid(
            "The new manager belongs to a different Swarm cluster.",
        ));
    }
    Ok(())
}

fn invalid(message: &str) -> PlatformRegistrationError {
    PlatformRegistrationError::Validation(message.into())
}

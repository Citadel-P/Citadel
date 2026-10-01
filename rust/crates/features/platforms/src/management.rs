use crate::CreatePlatformInput;
use crate::PlatformConnectorType;
use crate::PlatformDetails;
use crate::PlatformRegistrationError;
use crate::PlatformType;
use crate::RuntimePlatformInfo;
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
    current: &PlatformDetails,
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
    if input.platform_type != PlatformType::from(current.platform_type) {
        return Err(invalid(
            "Platform orchestration type cannot change after creation.",
        ));
    }
    if input.connector_type == PlatformConnectorType::Unknown {
        return Err(invalid("A supported connector is required."));
    }
    if (input.connector_type == PlatformConnectorType::EdgeAgent)
        != (current.connector_type == crate::ConnectorKind::EdgeAgent)
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
    current: &PlatformDetails,
    info: &RuntimePlatformInfo,
) -> Result<(), PlatformRegistrationError> {
    let kind = if current.platform_type == crate::PlatformKind::DockerSwarm {
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

pub trait PlatformManagementRepository: Send + Sync {
    fn update<'a>(
        &'a self,
        current: &'a PlatformDetails,
        input: &'a CreatePlatformInput,
        info: Option<&'a RuntimePlatformInfo>,
        actor: citadel_primitives::ActorId,
        rename: bool,
    ) -> futures_util::future::BoxFuture<'a, Result<(), PlatformRegistrationError>>;
}
pub trait PlatformManagementRuntime: Send + Sync {
    fn inspect_candidate<'a>(
        &'a self,
        id: Uuid,
        input: &'a CreatePlatformInput,
        cancel: &'a tokio_util::sync::CancellationToken,
    ) -> futures_util::future::BoxFuture<
        'a,
        Result<RuntimePlatformInfo, crate::RuntimeCapabilityError>,
    >;
}
pub struct PlatformManagementService {
    repository: std::sync::Arc<dyn PlatformManagementRepository>,
    runtime: std::sync::Arc<dyn PlatformManagementRuntime>,
}
impl PlatformManagementService {
    pub fn new(
        repository: std::sync::Arc<dyn PlatformManagementRepository>,
        runtime: std::sync::Arc<dyn PlatformManagementRuntime>,
    ) -> Self {
        Self {
            repository,
            runtime,
        }
    }
    pub async fn update(
        &self,
        current: &PlatformDetails,
        input: &CreatePlatformInput,
        actor: citadel_primitives::ActorId,
        rename: bool,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<(), PlatformRegistrationError> {
        let info = if rename {
            None
        } else {
            let info = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                self.runtime.inspect_candidate(current.id, input, cancel),
            )
            .await
            .map_err(|_| {
                PlatformRegistrationError::Runtime(crate::RuntimeCapabilityError::new(
                    crate::RuntimeErrorKind::Timeout,
                    "Platform validation timed out.",
                    false,
                ))
            })??;
            validate_target(current, &info)?;
            Some(info)
        };
        self.repository
            .update(current, input, info.as_ref(), actor, rename)
            .await
    }
}

pub trait AgentSigningPort: Send + Sync {
    fn public_key_base64(&self) -> String;
    fn rotate(
        &self,
    ) -> futures_util::future::BoxFuture<'_, Result<(), crate::RuntimeCapabilityError>>;
}

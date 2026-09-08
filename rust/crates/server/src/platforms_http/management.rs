use super::*;
use citadel_platforms::prune::{PlatformPrunePort, PrunePlatformInput};
use citadel_platforms::{
    PlatformRuntimePort,
    management::{RenamePlatformInput, patch_input, validate_target},
};

static PRUNE_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

pub(super) async fn prune(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PrunePlatformInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    if id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation(
                "A Platform ID is required.".into(),
            )),
            &headers,
        );
    }
    authorize_platform_level(&state, &principal, id, PermissionLevel::Execute, &headers).await?;
    let _permit = identity_result(
        PRUNE_SLOTS
            .try_acquire()
            .map_err(|_| IdentityError::Conflict("Prune capacity is busy. Retry later.".into())),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let operation = async {
        let runtime = runtime_for(&state, id).await?;
        match &runtime {
            RuntimeRef::Local(r) => r.prune(input.resource, &cancel).await,
            RuntimeRef::Agent(r) => r.prune(input.resource, &cancel).await,
            RuntimeRef::Edge(r) => r.prune(input.resource, &cancel).await,
        }
    };
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(300), operation).await;
    // Partial failures can still delete resources. Invalidate inventory on every outcome.
    for resource in ["volume", "network", "image", "platform"] {
        publish_runtime_change(&state, id, resource, "update", &id.to_string());
    }
    match outcome {
        Ok(Ok(value)) => Ok(no_store(Json(value).into_response())),
        Ok(Err(error)) => Ok(runtime_error_response(error, &headers)),
        Err(_) => Ok(runtime_error_response(
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Timeout,
                "Prune timed out; some deletions may have completed.",
                false,
            ),
            &headers,
        )),
    }
}

#[derive(Clone)]
pub struct AgentSetupContext {
    pub signer: citadel_adapters::agent::AgentRequestSigner,
    pub image: String,
    pub requires_tls: bool,
}
impl AgentSetupContext {
    pub fn view(&self) -> citadel_platforms::agent_setup::AgentSetupView {
        citadel_platforms::agent_setup::AgentSetupView::new(
            self.signer.public_key_base64(),
            self.image.clone(),
            self.requires_tls,
        )
    }
}

pub(super) async fn rotate_key(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    setup: Option<Extension<AgentSetupContext>>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    identity_result(
        state
            .identity
            .authorize(
                &principal,
                ResourceType::Platform,
                PermissionLevel::Write,
                None,
            )
            .await,
        &headers,
    )?;
    let Extension(setup) = identity_result(
        setup.ok_or_else(|| IdentityError::Storage("Agent key storage is not configured.".into())),
        &headers,
    )?;
    let signer = setup.signer.clone();
    identity_result(
        tokio::task::spawn_blocking(move || signer.rotate())
            .await
            .map_err(|_| IdentityError::Storage("Agent key rotation failed.".into()))
            .and_then(|v| {
                v.map_err(|_| {
                    IdentityError::Storage("Agent key rotation could not be persisted.".into())
                })
            }),
        &headers,
    )?;
    Ok(no_store(Json(setup.view()).into_response()))
}

pub(super) async fn rename(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenamePlatformInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    update(
        state,
        principal,
        input.id,
        serde_json::json!({"name":input.name}),
        true,
        headers,
    )
    .await
}

pub(super) async fn patch(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<serde_json::Value>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    update(state, principal, id, input, false, headers).await
}

async fn update(
    state: PlatformsHttpState,
    principal: ActorPrincipal,
    id: Uuid,
    input: serde_json::Value,
    rename: bool,
    headers: HeaderMap,
) -> IdentityHttpResult {
    if id.is_nil() {
        return identity_result(
            Err(IdentityError::Validation(
                "A Platform ID is required.".into(),
            )),
            &headers,
        );
    }
    let capabilities =
        authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let current = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    let input = identity_result(
        patch_input(&current, &input).map_err(platform_registration_error),
        &headers,
    )?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let info = if rename {
        None
    } else {
        let operation = async {
            match input.connector_type {
                citadel_platforms::PlatformConnectorType::Local => {
                    state.docker.get_info(&cancel).await
                }
                citadel_platforms::PlatformConnectorType::Agent => {
                    let agent = state.agent.as_ref().ok_or_else(|| {
                        RuntimeCapabilityError::new(
                            RuntimeErrorKind::Unavailable,
                            "Agent transport is not configured.",
                            false,
                        )
                    })?;
                    let agent = agent
                        .for_address(input.address.as_deref().unwrap_or_default(), &cancel)
                        .await?;
                    agent.get_info(&cancel).await
                }
                citadel_platforms::PlatformConnectorType::EdgeAgent => {
                    let session = state.edge.get(&EdgeTarget::platform(id)).map_err(|_| {
                        RuntimeCapabilityError::new(
                            RuntimeErrorKind::Unavailable,
                            "Edge Agent is disconnected.",
                            false,
                        )
                    })?;
                    EdgeRuntime { session }.get_info(&cancel).await
                }
                _ => unreachable!("validated connector"),
            }
        };
        let info = match tokio::time::timeout(std::time::Duration::from_secs(30), operation).await {
            Ok(Ok(info)) => info,
            Ok(Err(error)) => return Ok(runtime_error_response(error, &headers)),
            Err(_) => {
                return Ok(runtime_error_response(
                    RuntimeCapabilityError::new(
                        RuntimeErrorKind::Timeout,
                        "Platform validation timed out.",
                        false,
                    ),
                    &headers,
                ));
            }
        };
        identity_result(
            validate_target(&current, &info).map_err(platform_registration_error),
            &headers,
        )?;
        Some(info)
    };
    identity_result(
        citadel_adapters::platform_management::update(
            &state.pool,
            &current,
            &input,
            info.as_ref(),
            principal.actor_id,
            rename,
        )
        .await
        .map_err(platform_registration_error),
        &headers,
    )?;
    let mut updated = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    updated.capabilities = Some(capabilities);
    publish_runtime_change(&state, id, "platform", "update", &id.to_string());
    Ok(no_store(Json(updated).into_response()))
}

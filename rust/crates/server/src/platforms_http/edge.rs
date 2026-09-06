use super::*;
use citadel_adapters::edge::{EdgeRegistry, EdgeStoreError, EdgeTarget, PostgresEdgeStore};

#[derive(Clone)]
pub struct EdgeHttpContext {
    pub store: PostgresEdgeStore,
    pub registry: EdgeRegistry,
    pub core_url: String,
    pub agent_image: String,
    pub node_agent_ca_bundle: Option<Arc<[u8]>>,
}

use citadel_platforms::node_agents::setup::{NodeAgentSetupService, SetupKind, SetupOptions};
macro_rules! setup_handler {
    ($name:ident,$kind:ident) => {
        pub(super) async fn $name(
            State(state): State<PlatformsHttpState>,
            Extension(edge): Extension<EdgeHttpContext>,
            principal: Option<Extension<ActorPrincipal>>,
            path: Result<Path<Uuid>, PathRejection>,
            headers: HeaderMap,
        ) -> IdentityHttpResult {
            node_agent_operation(
                state,
                principal,
                path,
                headers,
                Some((
                    SetupKind::$kind,
                    SetupOptions {
                        core_url: edge.core_url,
                        image: edge.agent_image,
                        ca_bundle: edge.node_agent_ca_bundle,
                    },
                )),
            )
            .await
        }
    };
}
setup_handler!(install_node_agents, Install);
setup_handler!(repair_node_agents, Repair);
setup_handler!(upgrade_node_agents, Upgrade);

pub(super) async fn remove_node_agents(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    node_agent_operation(state, principal, path, headers, None).await
}
async fn node_agent_operation(
    state: PlatformsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    setup: Option<(SetupKind, SetupOptions)>,
) -> IdentityHttpResult {
    static OPERATIONS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let capabilities =
        authorize_platform_level(&state, &principal, id, PermissionLevel::Execute, &headers)
            .await?;
    if !capabilities.can_manage_node_agents {
        return identity_result(Err(IdentityError::Forbidden), &headers);
    }
    let platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if platform.platform_type != "DockerSwarm" {
        return identity_result(
            Err(IdentityError::Validation(
                "Node agents require a Docker Swarm platform.".into(),
            )),
            &headers,
        );
    }
    if let Some((_, options)) = &setup {
        identity_result(
            options
                .validate()
                .map_err(|e| IdentityError::Validation(e.message)),
            &headers,
        )?;
    }
    let permit = match OPERATIONS.try_acquire() {
        Ok(permit) => permit,
        Err(_) => {
            return Ok(runtime_error_response(
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::ResourceExhausted,
                    "Too many node-agent operations are running.",
                    true,
                ),
                &headers,
            ));
        }
    };
    let notify_state = state.clone();
    let store = Arc::new(
        citadel_adapters::node_agent_lifecycle_store::PostgresNodeAgentLifecycleStore(
            state.pool.clone(),
        ),
    );
    let runtime = Arc::new(
        citadel_adapters::node_agent_runtime::NodeAgentRuntimeRouter {
            pool: state.pool,
            docker: state.docker,
            agent: state.agent,
            edge: state.edge,
        },
    );
    let changed: Arc<dyn Fn(Uuid) + Send + Sync> = Arc::new(move |id| {
        publish_runtime_change(
            &notify_state,
            id,
            "nodeAgentCoverage",
            "update",
            &id.to_string(),
        );
    });
    let cancellation = CancellationToken::new();
    let guard = cancellation.clone().drop_guard();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(32);
    tokio::spawn(async move {
        let _permit = permit;
        if let Some((kind, options)) = setup {
            NodeAgentSetupService {
                store,
                runtime,
                changed,
            }
            .run(principal.actor_id, id, kind, options, sender, cancellation)
            .await;
        } else {
            citadel_platforms::node_agents::lifecycle::NodeAgentRemovalService {
                store,
                runtime,
                changed,
            }
            .remove(principal.actor_id, id, sender, cancellation)
            .await;
        }
    });
    let stream = async_stream::stream! {
        let _guard = guard;
        yield Ok::<_,std::convert::Infallible>(bytes::Bytes::from_static(b"["));
        let mut first=true;
        while let Some(item)=receiver.recv().await {
            if !first {yield Ok(bytes::Bytes::from_static(b","));} first=false;
            yield Ok(bytes::Bytes::from(serde_json::to_vec(&item).expect("Node-agent progress serializes")));
        }
        yield Ok(bytes::Bytes::from_static(b"]"));
    };
    let mut response = no_store(axum::body::Body::from_stream(stream).into_response());
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    Ok(response)
}

pub(super) async fn node_coverage(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let capabilities =
        authorize_platform_level(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let mut platform = required(
        state
            .platforms
            .get_platform(id)
            .await
            .map_err(platform_error),
        &headers,
    )?;
    if platform.platform_type != "DockerSwarm" {
        return identity_result(
            Err(IdentityError::Validation(
                "Node Agent coverage is available only for Docker Swarm platforms.".into(),
            )),
            &headers,
        );
    }
    let mut coverage = identity_result(
        citadel_adapters::node_agent_coverage::read(&state.pool, &state.edge, &platform)
            .await
            .map_err(|error| IdentityError::Storage(error.to_string())),
        &headers,
    )?;
    if coverage.total_nodes == 0 {
        let initialized = async {
            let runtime = runtime_for(&state, id).await?;
            let port: &dyn PlatformInventoryPort = match &runtime {
                RuntimeRef::Local(port) => *port,
                RuntimeRef::Agent(port) => *port,
                RuntimeRef::Edge(port) => port,
            };
            let cancellation = CancellationToken::new();
            let _cancel_on_drop = cancellation.clone().drop_guard();
            let target = citadel_platforms::jobs::InventoryCollectionTarget {
                platform_id: id,
                platform_type: platform.platform_type.clone(),
            };
            let snapshot = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                citadel_platforms::jobs::collect_inventory(port, &target, &cancellation),
            )
            .await
            .map_err(|_| {
                RuntimeCapabilityError::new(
                    RuntimeErrorKind::Timeout,
                    "Swarm inventory initialization timed out.",
                    true,
                )
            })??;
            citadel_adapters::inventory_projection_store::PostgresInventoryProjectionStore::new(
                state.pool.clone(),
            )
            .initialize_swarm(&snapshot)
            .await
        }
        .await;
        let changed = match initialized {
            Ok(changed) => changed,
            Err(error) => return Ok(runtime_error_response(error, &headers)),
        };
        if changed {
            publish_runtime_change(&state, id, "platform", "update", &id.to_string());
        }
        platform = required(
            state
                .platforms
                .get_platform(id)
                .await
                .map_err(platform_error),
            &headers,
        )?;
        coverage = identity_result(
            citadel_adapters::node_agent_coverage::read(&state.pool, &state.edge, &platform)
                .await
                .map_err(|error| IdentityError::Storage(error.to_string())),
            &headers,
        )?;
    }
    coverage.can_manage_node_agents = capabilities.can_manage_node_agents;
    Ok(no_store(Json(coverage).into_response()))
}

pub(super) async fn enroll(
    State(state): State<PlatformsHttpState>,
    Extension(edge): Extension<EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    edge.enrollment(
        EdgeTarget::platform(id),
        principal.actor_id.value(),
        &headers,
    )
    .await
}
pub(super) async fn status(
    State(state): State<PlatformsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_platform_level(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let status = identity_result(
        PostgresEdgeStore::new(state.pool)
            .status(&EdgeTarget::platform(id))
            .await
            .map_err(error),
        &headers,
    )?;
    Ok(no_store(Json(status).into_response()))
}
pub(super) async fn revoke(
    State(state): State<PlatformsHttpState>,
    Extension(edge): Extension<EdgeHttpContext>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_platform_level(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let target = EdgeTarget::platform(id);
    let store = PostgresEdgeStore::new(state.pool.clone());
    identity_result(store.status(&target).await.map_err(error), &headers)?;
    identity_result(store.revoke(&target).await.map_err(error), &headers)?;
    edge.registry.disconnect(&target);
    publish_runtime_change(&state, id, "platform", "update", &id.to_string());
    Ok(no_store(StatusCode::NO_CONTENT.into_response()))
}
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}
fn error(error: EdgeStoreError) -> IdentityError {
    EdgeHttpContext::error(error)
}

impl EdgeHttpContext {
    pub(crate) async fn enrollment(
        &self,
        target: EdgeTarget,
        actor_id: Uuid,
        headers: &HeaderMap,
    ) -> IdentityHttpResult {
        let (enrollment, token, expires) = identity_result(
            self.store
                .create_enrollment(&target, actor_id)
                .await
                .map_err(Self::error),
            headers,
        )?;
        let name = if target.resource_type == 1 {
            "edge-build-agent"
        } else {
            "edge-agent"
        };
        let host_mount = if target.resource_type == 0 {
            " -v /:/host:ro --label com.citadel.system=true --label com.citadel.system-role=edge-agent"
        } else {
            ""
        };
        let volume = name.replace('-', "_") + "_data";
        let environment = std::collections::BTreeMap::from([
            ("CITADEL_AGENT_MODE", "edge".to_owned()),
            ("CITADEL_CORE_URL", self.core_url.clone()),
            ("CITADEL_EDGE_ENROLLMENT_TOKEN", token.clone()),
            (
                "CITADEL_EDGE_AGENT_KEY_PATH",
                format!("/app/data/{name}.key"),
            ),
            (
                "CITADEL_EDGE_IDENTITY_PATH",
                format!("/app/data/{name}.identity.json"),
            ),
        ]);
        let env = environment
            .iter()
            .map(|(key, value)| format!(" -e {}", shell_quote(&format!("{key}={value}"))))
            .collect::<String>();
        let command = format!(
            "docker run -d --name {name} --restart unless-stopped -v /var/run/docker.sock:/var/run/docker.sock{host_mount} -v {volume}:/app/data{env} {}",
            shell_quote(&self.agent_image)
        );
        Ok(no_store(Json(serde_json::json!({"enrollmentId":enrollment,"platformId":target.platform_id,"token":token,"expiresAtUtc":expires,"instructions":{"coreUrl":self.core_url,"environment":environment,"agentImage":self.agent_image,"dockerRunCommand":command}})).into_response()))
    }
    pub(crate) fn error(error: EdgeStoreError) -> IdentityError {
        match error {
            EdgeStoreError::NotFound => IdentityError::NotFound,
            EdgeStoreError::Unauthorized => IdentityError::Conflict(error.to_string()),
            EdgeStoreError::Invalid(message) => IdentityError::Validation(message.into()),
            EdgeStoreError::Storage(source) => IdentityError::Storage(source.to_string()),
        }
    }
}

use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_contracts::citadel::images::v1::{
    DistributionInspectRequest, DistributionInspectResponse,
};
use citadel_platforms::node_agents::setup::*;
use serde_json::{Value, json};

impl NodeAgentRuntimeRouter {
    async fn checked_target(
        &self,
        claim: &NodeAgentRemovalClaim,
        cancel: &CancellationToken,
    ) -> Result<Target, RuntimeCapabilityError> {
        let target = self.target(claim.platform_id).await?;
        validate_identity(&target.inventory().get_info(cancel).await?, claim)?;
        Ok(target)
    }
}
fn validate_identity(
    info: &RuntimePlatformInfo,
    claim: &NodeAgentRemovalClaim,
) -> Result<(), RuntimeCapabilityError> {
    if info.daemon_id != claim.manager_daemon_id
        || !info.swarm.as_ref().is_some_and(|s| {
            s.control_available
                && s.local_node_state.eq_ignore_ascii_case("active")
                && s.node_id == claim.manager_node_id
                && s.cluster_id.as_ref() == Some(&claim.cluster_id)
        })
    {
        return Err(failure(
            "The pinned manager identity changed during node-agent setup.",
        ));
    }
    Ok(())
}
impl NodeAgentSetupRuntime for NodeAgentRuntimeRouter {
    fn snapshot<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeInventorySnapshot, RuntimeCapabilityError>> {
        Box::pin(async move {
            let target = self.target(claim.platform_id).await?;
            let snapshot = citadel_platforms::jobs::collect_inventory(
                target.inventory(),
                &citadel_platforms::jobs::InventoryCollectionTarget {
                    platform_id: claim.platform_id,
                    platform_type: "DockerSwarm".into(),
                },
                cancel,
            )
            .await?;
            validate_identity(&snapshot.info, claim)?;
            Ok(snapshot)
        })
    }
    fn distribution<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        image: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<AgentDistribution, RuntimeCapabilityError>> {
        Box::pin(async move {
            let target = self.checked_target(claim, cancel).await?;
            let request = DistributionInspectRequest {
                image_name: image.into(),
                auth: None,
            };
            let response = match target {
                Target::Local(d) => {
                    let value = d.distribution_inspect(image).await.map_err(docker_error)?;
                    return Ok(docker_distribution(&value));
                }
                Target::Agent(a) => a.node_agent_distribution(request, cancel).await?,
                Target::Edge(e) => {
                    unary::<_, DistributionInspectResponse>(
                        &e.session,
                        EdgeCommandKind::ImageDistributionInspect,
                        request,
                        cancel,
                    )
                    .await?
                }
            };
            let descriptor = response.descriptor.unwrap_or_default();
            let platforms = response.platforms.into_iter().chain(descriptor.platform);
            Ok(AgentDistribution {
                digest: descriptor.digest,
                linux_architectures: platforms
                    .filter(|p| p.os.eq_ignore_ascii_case("linux"))
                    .map(|p| architecture(&p.architecture).to_owned())
                    .collect(),
            })
        })
    }
    fn create_material<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        kind: NodeAgentResource,
        name: &'a str,
        data: &'a [u8],
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, RuntimeCapabilityError>> {
        Box::pin(async move {
            let target = self.checked_target(claim, cancel).await?;
            let labels = ownership(claim).into_iter().collect();
            let secret = matches!(kind, NodeAgentResource::Secret);
            if matches!(kind, NodeAgentResource::Service) {
                return Err(failure("Invalid node-agent material type."));
            }
            let response = match target {
                Target::Local(d) => {
                    let response=d.create_swarm_material(secret,&json!({"Name":name,"Data":STANDARD.encode(data),"Labels":ownership(claim)})).await.map_err(docker_error)?;
                    return response["ID"]
                        .as_str()
                        .filter(|id| !id.is_empty())
                        .map(str::to_owned)
                        .ok_or_else(|| failure("Docker returned no material ID."));
                }
                Target::Agent(a) => {
                    if secret {
                        a.node_agent_secret(
                            CreateSwarmSecretRequest {
                                name: name.into(),
                                data: data.into(),
                                labels,
                            },
                            cancel,
                        )
                        .await?
                    } else {
                        a.node_agent_config(
                            CreateSwarmConfigRequest {
                                name: name.into(),
                                data: data.into(),
                                labels,
                            },
                            cancel,
                        )
                        .await?
                    }
                }
                Target::Edge(e) => {
                    if secret {
                        unary::<_, SwarmResourceCreateResponse>(
                            &e.session,
                            EdgeCommandKind::SwarmSecretCreate,
                            CreateSwarmSecretRequest {
                                name: name.into(),
                                data: data.into(),
                                labels,
                            },
                            cancel,
                        )
                        .await?
                    } else {
                        unary(
                            &e.session,
                            EdgeCommandKind::SwarmConfigCreate,
                            CreateSwarmConfigRequest {
                                name: name.into(),
                                data: data.into(),
                                labels,
                            },
                            cancel,
                        )
                        .await?
                    }
                }
            };
            if response.resource_id.is_empty() {
                return Err(failure("Agent returned no material ID."));
            }
            Ok(response.resource_id)
        })
    }
    fn apply_system<'a>(
        &'a self,
        claim: &'a NodeAgentRemovalClaim,
        spec: &'a SystemAgentSpec,
        current: Option<&'a RuntimeSwarmService>,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, RuntimeCapabilityError>> {
        Box::pin(async move {
            let target = self.checked_target(claim, cancel).await?;
            // Never update a foreign system Service even if the caller supplied a stale projection.
            if let Some(service) = current {
                if service.version_index < 0 {
                    return Err(failure("Docker returned an invalid Service version."));
                }
                let live = target.inventory().list_swarm_services(cancel).await?;
                if !live.iter().any(|s| {
                    s.id == service.id
                        && s.version_index == service.version_index
                        && owned(&s.labels, claim.platform_id, &claim.cluster_id)
                }) {
                    return Err(failure(
                        "Node-agent Service ownership or version changed. Repair coverage to retry.",
                    ));
                }
            }
            let proto = SystemSwarmServiceSpecMessage {
                image: spec.image.clone(),
                environment: spec.environment.clone(),
                manager_node_id: spec.manager_node_id.clone(),
                state_volume_name: spec.volume_name.clone(),
                bootstrap_secret_id: spec.secret_id.clone(),
                bootstrap_secret_name: spec.secret_name.clone(),
                ca_config_id: spec.ca_config_id.clone().unwrap_or_default(),
                ca_config_name: spec.ca_config_name.clone().unwrap_or_default(),
                limit_nano_cpus: 500_000_000,
                limit_memory_bytes: 512 * 1024 * 1024,
                pids_limit: 256,
                stop_grace_period_nanoseconds: 30_000_000_000,
                supported_architectures: spec.architectures.clone(),
            };
            let create = CreateSystemSwarmServiceRequest {
                docker_name: spec.name.clone(),
                spec: Some(proto.clone()),
                labels: spec.labels.clone().into_iter().collect(),
                container_labels: spec.labels.clone().into_iter().collect(),
                operation_id: claim.operation_id.to_string(),
            };
            let update = current.map(|s| UpdateSystemSwarmServiceRequest {
                service_id: s.id.clone(),
                version_index: s.version_index as u64,
                spec: Some(proto),
                labels: create.labels.clone(),
                container_labels: create.container_labels.clone(),
                operation_id: create.operation_id.clone(),
            });
            let response = match target {
                Target::Local(d) => {
                    let body = system_docker_spec(spec);
                    return if let Some(current) = current {
                        d.update_swarm_service(&current.id, current.version_index, &body)
                            .await
                            .map_err(docker_error)?;
                        Ok(current.id.clone())
                    } else {
                        let created = d.create_swarm_service(&body).await.map_err(docker_error)?;
                        created["ID"]
                            .as_str()
                            .filter(|id| !id.is_empty())
                            .map(str::to_owned)
                            .ok_or_else(|| failure("Docker returned no Service ID."))
                    };
                }
                Target::Agent(a) => {
                    if let Some(update) = update {
                        a.node_agent_update(update, cancel).await?
                    } else {
                        a.node_agent_create(create, cancel).await?
                    }
                }
                Target::Edge(e) => {
                    if let Some(update) = update {
                        unary::<_, SwarmServiceMutationResponse>(
                            &e.session,
                            EdgeCommandKind::SwarmSystemServiceUpdate,
                            update,
                            cancel,
                        )
                        .await?
                    } else {
                        unary(
                            &e.session,
                            EdgeCommandKind::SwarmSystemServiceCreate,
                            create,
                            cancel,
                        )
                        .await?
                    }
                }
            };
            if response.service_id.is_empty() {
                return Err(failure("Agent returned no Service ID."));
            }
            Ok(response.service_id)
        })
    }
    fn connected(&self, id: Uuid, node: &str) -> bool {
        self.edge
            .get(&EdgeTarget::node(id, node.into()))
            .is_ok_and(|s| !s.is_closed())
    }
}

fn docker_distribution(value: &Value) -> AgentDistribution {
    let platforms = value["Platforms"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(value["Descriptor"].get("platform"));
    AgentDistribution {
        digest: value["Descriptor"]["digest"]
            .as_str()
            .unwrap_or_default()
            .into(),
        linux_architectures: platforms
            .filter(|p| {
                p["os"]
                    .as_str()
                    .is_some_and(|os| os.eq_ignore_ascii_case("linux"))
            })
            .filter_map(|p| p["architecture"].as_str())
            .map(architecture)
            .collect(),
    }
}

fn system_docker_spec(spec: &SystemAgentSpec) -> Value {
    let configs:Vec<_>=spec.ca_config_id.iter().map(|id|json!({"ConfigID":id,"ConfigName":spec.ca_config_name,"File":{"Name":"citadel-core-ca.crt","UID":"0","GID":"0","Mode":292}})).collect();
    json!({"Name":spec.name,"Labels":spec.labels,"Mode":{"Global":{}},"EndpointSpec":{"Mode":"vip","Ports":[]},"Networks":[],
        "TaskTemplate":{"ContainerSpec":{"Image":spec.image,"Env":spec.environment,"Labels":spec.labels,"User":"0","ReadOnly":true,"Init":true,"Privileges":{"NoNewPrivileges":true},"CapabilityDrop":["ALL"],"StopGracePeriod":30_000_000_000i64,
            "Mounts":[{"Type":"bind","Source":"/var/run/docker.sock","Target":"/var/run/docker.sock","ReadOnly":false},{"Type":"volume","Source":spec.volume_name,"Target":"/app/data"},{"Type":"tmpfs","Target":"/tmp","TmpfsOptions":{"SizeBytes":67108864,"Mode":448}}],
            "Secrets":[{"SecretID":spec.secret_id,"SecretName":spec.secret_name,"File":{"Name":"citadel-edge-bootstrap","UID":"0","GID":"0","Mode":256}}],"Configs":configs,
            "Healthcheck":{"Test":["CMD-SHELL","wget -q -O - http://127.0.0.1:9000/health >/dev/null || exit 1"],"Interval":30_000_000_000i64,"Timeout":5_000_000_000i64,"Retries":3,"StartPeriod":10_000_000_000i64}},
            "Resources":{"Limits":{"NanoCPUs":500_000_000,"MemoryBytes":536870912,"Pids":256}},"Placement":{"Constraints":["node.platform.os == linux",format!("node.id != {}",spec.manager_node_id)],"Platforms":spec.architectures.iter().map(|a|json!({"OS":"linux","Architecture":a})).collect::<Vec<_>>()},"RestartPolicy":{"Condition":"any","Delay":5_000_000_000i64},"LogDriver":{"Name":"json-file","Options":{"max-size":"10m","max-file":"3"}}},
        "UpdateConfig":{"Parallelism":1,"FailureAction":"rollback","Monitor":30_000_000_000i64,"MaxFailureRatio":0,"Order":"stop-first"},"RollbackConfig":{"Parallelism":1,"FailureAction":"pause","Monitor":30_000_000_000i64,"MaxFailureRatio":0,"Order":"stop-first"}})
}

#[cfg(test)]
mod tests {
    use super::*;
    // Ports both GetLinuxArchitectures .NET unit tests: single OCI manifest and multi-arch index.
    #[test]
    fn distribution_unions_linux_manifest_and_descriptor_platforms() {
        let descriptor = json!({"Descriptor":{"digest":"sha256:abc","platform":{"os":"linux","architecture":"x86_64"}}});
        assert_eq!(
            docker_distribution(&descriptor).linux_architectures,
            ["amd64".into()].into()
        );
        let mut index = descriptor;
        index["Platforms"] =
            json!([{"os":"linux","architecture":"aarch64"},{"os":"windows","architecture":"386"}]);
        assert_eq!(
            docker_distribution(&index).linux_architectures,
            ["amd64".into(), "arm64".into()].into()
        );
    }
    #[test]
    fn system_service_has_bounded_resources_and_valid_read_only_file_targets() {
        let spec = SystemAgentSpec {
            name: "agent".into(),
            image: "agent@sha256:pinned".into(),
            environment: vec![],
            manager_node_id: "manager".into(),
            volume_name: "durable-state".into(),
            secret_id: "secret".into(),
            secret_name: "bootstrap".into(),
            ca_config_id: Some("ca".into()),
            ca_config_name: Some("ca-name".into()),
            architectures: vec!["amd64".into()],
            labels: Default::default(),
        };
        let value = system_docker_spec(&spec);
        let task = &value["TaskTemplate"];
        let container = &task["ContainerSpec"];
        assert_eq!(container["ReadOnly"], true);
        assert_eq!(container["CapabilityDrop"], json!(["ALL"]));
        assert_eq!(
            container["Secrets"][0]["File"],
            json!({"Name":"citadel-edge-bootstrap","UID":"0","GID":"0","Mode":256})
        );
        assert_eq!(
            container["Configs"][0]["File"],
            json!({"Name":"citadel-core-ca.crt","UID":"0","GID":"0","Mode":292})
        );
        assert_eq!(container["Mounts"][1]["Source"], "durable-state");
        assert_eq!(task["Resources"]["Limits"]["MemoryBytes"], 536870912);
        assert_eq!(
            task["Placement"]["Constraints"],
            json!(["node.platform.os == linux", "node.id != manager"])
        );
        assert_eq!(value["Mode"], json!({"Global":{}}));
        assert_eq!(value["UpdateConfig"]["Order"], "stop-first");
        assert_eq!(value["EndpointSpec"]["Ports"], json!([]));
    }
}

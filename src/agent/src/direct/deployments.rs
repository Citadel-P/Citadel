use super::Runtime;
use citadel_adapters::connectors::docker::deployments::{parse_mount, parse_port};
use citadel_contracts::citadel::deployments::v1::{
    deployment_service_server::DeploymentService, *,
};
use citadel_deployments::{DeploymentError, RuntimeContainerState};
use serde_json::{Value, json};
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl DeploymentService for Runtime {
    async fn apply(
        &self,
        request: Request<ApplyDeploymentRequest>,
    ) -> Result<Response<ApplyDeploymentResponse>, Status> {
        let r = request.into_inner();
        let body = create_body(&r)?;
        let result = self
            .docker
            .apply_container_config(&r.name, &r.image_id, &body, &self.shutdown)
            .await
            .map_err(error)?;
        Ok(Response::new(ApplyDeploymentResponse {
            container_id: result.docker_container_id,
            deployed_container_state: match result.state {
                RuntimeContainerState::Running => DeployedContainerState::Running,
                RuntimeContainerState::Exited => DeployedContainerState::Exited,
                RuntimeContainerState::Timeout => DeployedContainerState::Timeout,
            } as i32,
        }))
    }
}

fn create_body(r: &ApplyDeploymentRequest) -> Result<Value, Status> {
    if r.image_id.trim().is_empty() {
        return Err(Status::invalid_argument("Image is required"));
    }
    let spec = r.spec.clone().unwrap_or_default();
    let mut bindings = serde_json::Map::new();
    for port in &spec.ports {
        let (key, host) = parse_port(port).map_err(error)?;
        if let Some(host) = host {
            bindings.insert(key, json!([{"HostPort":host.to_string()}]));
        }
    }
    let mounts = spec
        .volumes
        .iter()
        .map(|v| parse_mount(v).map_err(error))
        .collect::<Result<Vec<_>, _>>()?;
    let resource = spec.resource_spec.unwrap_or_default();
    let number = |v: Option<f32>| -> Result<Option<i64>, Status> {
        v.map(|v| {
            if v.is_finite() && v >= 0.0 && (v as f64) < i64::MAX as f64 {
                Ok(v as i64)
            } else {
                Err(Status::invalid_argument("Invalid resource limit"))
            }
        })
        .transpose()
    };
    let restart = spec
        .life_cycle_spec
        .as_ref()
        .map(|v| match v.restart_policy {
            0 => Ok("no"),
            1 => Ok("always"),
            2 => Ok("on-failure"),
            3 => Ok("unless-stopped"),
            _ => Err(Status::invalid_argument("Invalid restart policy")),
        })
        .transpose()?;
    let signal = spec
        .life_cycle_spec
        .as_ref()
        .and_then(|v| v.stop_signal)
        .map(|v| match v {
            0 => Ok("SIGTERM"),
            1 => Ok("SIGKILL"),
            2 => Ok("SIGINT"),
            3 => Ok("SIGQUIT"),
            _ => Err(Status::invalid_argument("Invalid stop signal")),
        })
        .transpose()?;
    Ok(
        json!({"Image":r.image_id,"Labels":spec.labels,"Cmd":(!spec.command.is_empty()).then_some(spec.command),
            "Env":(!spec.env_vars.is_empty()).then_some(spec.env_vars),"StopSignal":signal,
            "StopTimeout":spec.life_cycle_spec.and_then(|v|v.stop_timeout),
            "HostConfig":{"PortBindings":bindings,"Mounts":mounts,"Memory":number(resource.memory_limit)?,"NanoCpus":number(resource.nano_cpus)?,"RestartPolicy":restart.map(|v|json!({"Name":v}))},
            "NetworkingConfig":{"EndpointsConfig":spec.networks.iter().map(|name|(name.clone(),json!({}))).collect::<serde_json::Map<_,_>>()}
        }),
    )
}
fn error(e: DeploymentError) -> Status {
    match e {
        DeploymentError::Cancelled => Status::cancelled("Deployment cancelled"),
        DeploymentError::Validation(v) => Status::invalid_argument(v),
        e => Status::unavailable(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_units_mounts_and_presence_are_preserved() {
        let r = ApplyDeploymentRequest {
            image_id: "nginx".into(),
            name: "test".into(),
            spec: Some(DeploymentSpec {
                ports: vec!["8080:80".into(), "53/udp".into()],
                volumes: vec!["data:/data:ro".into()],
                networks: vec!["frontend".into()],
                resource_spec: Some(ResourceSpec {
                    nano_cpus: Some(1_000_000_000.0),
                    memory_limit: Some(0.0),
                }),
                life_cycle_spec: Some(LifeCycleSpec {
                    stop_timeout: Some(0),
                    stop_signal: Some(1),
                    restart_policy: 3,
                }),
                ..Default::default()
            }),
        };
        let v = create_body(&r).unwrap();
        assert_eq!(v["HostConfig"]["NanoCpus"], 1_000_000_000i64);
        assert_eq!(v["HostConfig"]["Memory"], 0);
        assert_eq!(v["StopTimeout"], 0);
        assert_eq!(v["StopSignal"], "SIGKILL");
        assert_eq!(
            v["HostConfig"]["Mounts"][0],
            json!({"Type":"volume","Source":"data","Target":"/data","ReadOnly":true})
        );
        assert_eq!(
            v["HostConfig"]["PortBindings"],
            json!({"80/tcp":[{"HostPort":"8080"}]})
        );
        assert_eq!(
            v["NetworkingConfig"]["EndpointsConfig"],
            json!({"frontend":{}})
        );
    }
}

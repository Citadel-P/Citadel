use super::Runtime;
use citadel_contracts::citadel::deployments::v1::{
    deployment_service_server::DeploymentService, *,
};
use citadel_deployments::{DeploymentError, RuntimeContainerState};
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
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
    let spec = r.spec.clone().unwrap_or_default();
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
    citadel_docker::client::deployments::CreateConfig {
        image: &r.image_id,
        labels: spec.labels.into_iter().collect(),
        command: (!spec.command.is_empty()).then_some(&spec.command),
        environment: (!spec.env_vars.is_empty()).then_some(&spec.env_vars),
        ports: &spec.ports,
        volumes: &spec.volumes,
        networks: &spec.networks,
        memory: number(resource.memory_limit)?,
        nano_cpus: number(resource.nano_cpus)?,
        restart,
        stop_signal: signal,
        stop_timeout: spec.life_cycle_spec.and_then(|v| v.stop_timeout),
    }
    .body()
    .map_err(error)
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
        assert_eq!(v["ExposedPorts"]["53/udp"], json!({}));
        assert_eq!(v["ExposedPorts"]["80/tcp"], json!({}));
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

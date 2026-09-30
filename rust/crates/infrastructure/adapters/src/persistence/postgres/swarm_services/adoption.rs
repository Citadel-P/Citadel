use super::*;
use crate::connectors::routing::swarm_services::SwarmServiceRuntimeRouter;
use citadel_contracts::citadel::swarm::v1::SwarmServiceMessage;
use citadel_primitives::AuthorizedResource;
use citadel_swarm_services::{adoption::*, *};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

pub struct PostgresSwarmServiceAdoption {
    pool: PgPool,
    runtime: SwarmServiceRuntimeRouter,
    key: Zeroizing<Vec<u8>>,
}
impl PostgresSwarmServiceAdoption {
    pub fn new(pool: PgPool, runtime: SwarmServiceRuntimeRouter, key: &[u8]) -> Self {
        Self {
            pool,
            runtime,
            key: Zeroizing::new(key.to_vec()),
        }
    }
    async fn load(
        &self,
        actor: ActorId,
        admin: bool,
        platform: Uuid,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<(String, SwarmServiceMessage), SwarmServiceError> {
        if platform.is_nil() || id.is_empty() || id.len() > 255 {
            return Err(validation("Invalid Platform or Docker Service id."));
        }
        let mut tx = self.pool.begin().await.map_err(storage)?;
        ensure_platform(&mut tx, actor, admin, platform, true).await?;
        if !admin
            && (!has_access(
                &mut tx,
                actor,
                Uuid::nil(),
                policy::CreateSwarmService::REQUIREMENT,
            )
            .await?
                || !has_access(
                    &mut tx,
                    actor,
                    platform,
                    PermissionRequirement {
                        resource_type: ResourceType::Platform,
                        level: PermissionLevel::Read,
                        specific: Some(citadel_primitives::SpecificPermission::Inspect),
                    },
                )
                .await?)
        {
            return Err(SwarmServiceError::Forbidden);
        }
        eligible(&mut tx, platform, id).await?;
        let name = sqlx::query_scalar("SELECT name FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        let service = self.runtime.inspect_service(platform, id, cancel).await?;
        if service.id != id || service.name.len() > 63 || service.definition.is_none() {
            return Err(conflict(
                "Docker did not return a Service that Citadel can adopt.",
            ));
        }
        let mut tx = self.pool.begin().await.map_err(storage)?;
        ownership(
            &mut tx,
            &serde_json::to_value(&service.labels).map_err(storage)?,
        )
        .await?;
        tx.commit().await.map_err(storage)?;
        Ok((name, service))
    }
    fn fingerprint(
        &self,
        platform: Uuid,
        service: &SwarmServiceMessage,
    ) -> Result<String, SwarmServiceError> {
        let labels: std::collections::BTreeMap<_, _> = service.labels.iter().collect();
        let value = serde_json::to_vec(&(
            platform,
            &service.id,
            service.version_index,
            &service.runtime_hash,
            &service.name,
            &service.image,
            labels,
        ))
        .map_err(storage)?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).map_err(storage)?;
        mac.update(b"citadel-swarm-service-adoption-v1\0");
        mac.update(&value);
        Ok(mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }
}
fn validation(message: &str) -> SwarmServiceError {
    SwarmServiceError::Validation(message.into())
}
fn conflict(message: &str) -> SwarmServiceError {
    SwarmServiceError::Conflict(message.into())
}

async fn eligible(
    tx: &mut Transaction<'_, Postgres>,
    platform: Uuid,
    id: &str,
) -> Result<(), SwarmServiceError> {
    let row = sqlx::query("SELECT isstale,ownership,labels FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid=$2 FOR UPDATE")
        .bind(platform).bind(id).fetch_optional(&mut **tx).await.map_err(storage)?.ok_or(SwarmServiceError::NotFound)?;
    if row.get::<bool, _>("isstale") {
        return Err(conflict(
            "Swarm Service inventory is stale. Refresh the platform and try again.",
        ));
    }
    if !matches!(
        row.get::<String, _>("ownership").as_str(),
        "Unmanaged" | "CitadelService"
    ) {
        return Err(conflict(
            "Service is managed or belongs to a Stack. Import a Stack together.",
        ));
    }
    ownership(tx, &row.get::<Value, _>("labels")).await?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM swarmservices WHERE platformid=$1 AND dockerserviceid=$2)",
    )
    .bind(platform)
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;
    if exists {
        return Err(conflict("Swarm Service is already managed by Citadel."));
    }
    Ok(())
}
async fn ownership(
    tx: &mut Transaction<'_, Postgres>,
    labels: &Value,
) -> Result<(), SwarmServiceError> {
    if labels["com.docker.stack.namespace"]
        .as_str()
        .is_some_and(|v| !v.trim().is_empty())
    {
        return Err(conflict(
            "Services owned by a Docker stack must be imported with their stack.",
        ));
    }
    if !labels.as_object().is_some_and(|l| {
        l.keys()
            .any(|k| k.to_ascii_lowercase().starts_with("com.citadel."))
    }) {
        return Ok(());
    }
    let owner = labels["com.citadel.service-id"]
        .as_str()
        .and_then(|v| Uuid::parse_str(v).ok());
    if owner.is_none()
        || !labels["com.citadel.managed"]
            .as_str()
            .is_some_and(|v| v.eq_ignore_ascii_case("true"))
        || labels.get("com.citadel.stack-id").is_some()
        || labels.get("com.citadel.deployment-id").is_some()
        || labels.get("com.citadel.system").is_some()
    {
        return Err(conflict(
            "Swarm Service has conflicting Citadel ownership labels.",
        ));
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmservices WHERE id=$1)")
        .bind(owner)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    if exists {
        return Err(conflict(
            "Swarm Service is owned by an existing Citadel Service.",
        ));
    }
    Ok(())
}

impl SwarmServiceAdoptionPort for PostgresSwarmServiceAdoption {
    fn draft<'a>(
        &'a self,
        actor: ActorId,
        admin: bool,
        platform: Uuid,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<SwarmServiceAdoptionDraft, SwarmServiceError>> {
        Box::pin(async move {
            let (platform_name, service) = self.load(actor, admin, platform, id, cancel).await?;
            let mut spec = definition(&service)?;
            let mut issues: Vec<_> = service
                .adoption_warnings
                .iter()
                .enumerate()
                .map(|(i, message)| SwarmServiceAdoptionIssue {
                    code: format!("unsupported-{}", i + 1),
                    message: message.clone(),
                })
                .collect();
            for entry in &mut spec.environment {
                if let Some((name, value)) = entry.split_once('=')
                    && citadel_primitives::is_sensitive_environment_name(name)
                    && !value.is_empty()
                {
                    issues.push(SwarmServiceAdoptionIssue { code: format!("sensitive-environment-{}",issues.len()+1),
                            message: format!("Enter a new value or Citadel binding for sensitive environment variable '{name}'.") });
                    *entry = format!("{name}=********");
                }
            }
            Ok(SwarmServiceAdoptionDraft {
                name: available_service_name(&self.pool, platform, &service.name, false).await?,
                description: Some(format!(
                    "Adopted from Docker Swarm Service {}.",
                    service.name
                )),
                preview_fingerprint: self.fingerprint(platform, &service)?,
                source: SwarmServiceAdoptionSource {
                    docker_service_id: service.id,
                    name: service.name,
                    platform_id: platform,
                    platform_name,
                },
                spec,
                issues,
            })
        })
    }
    fn adopt<'a>(
        &'a self,
        actor: ActorId,
        admin: bool,
        platform: Uuid,
        id: &'a str,
        input: &'a AdoptSwarmService,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        Box::pin(async move {
            let (_, service) = self.load(actor, admin, platform, id, cancel).await?;
            if !bool::from(
                self.fingerprint(platform, &service)?
                    .as_bytes()
                    .ct_eq(input.preview_fingerprint.as_bytes()),
            ) {
                return Err(conflict(
                    "Docker Service configuration changed. Reload the adoption draft and review it again.",
                ));
            }
            let SwarmServiceImageInfo::External { image_tag, .. } = &input.spec.image else {
                return Err(validation(
                    "Select an external image from the Docker Service's current image repository.",
                ));
            };
            if repository(&service.image).is_none()
                || repository(&service.image) != repository(image_tag)
            {
                return Err(validation(
                    "Select an external image from the Docker Service's current image repository.",
                ));
            }
            if input
                .spec
                .environment
                .iter()
                .filter_map(|v| v.split_once('='))
                .any(|(n, v)| {
                    citadel_primitives::is_sensitive_environment_name(n) && v == "********"
                })
            {
                return Err(validation(
                    "Enter new values or Citadel bindings for redacted sensitive environment variables.",
                ));
            }
            let create = CreateSwarmService {
                name: input.name.clone(),
                platform_id: platform,
                description: input.description.clone(),
                spec: input.spec.clone(),
                tag_ids: input.tag_ids.clone(),
                duplicate_source: None,
            };
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_platform(&mut tx, actor, admin, platform, true).await?;
            // Serialize adopters on the projected identity; no Docker mutation is performed.
            eligible(&mut tx, platform, id).await?;
            ownership(
                &mut tx,
                &serde_json::to_value(&service.labels).map_err(storage)?,
            )
            .await?;
            validate_references(&mut tx, actor, admin, &create).await?;
            validate_tags(&mut tx, &input.tag_ids).await?;
            let new_id = Uuid::now_v7();
            let health = if service.desired_task_count == 0 {
                citadel_swarm_services::SwarmServiceHealth::Stopped
            } else if service.update_state.contains("paused")
                || service.running_task_count < service.desired_task_count
            {
                citadel_swarm_services::SwarmServiceHealth::Degraded
            } else {
                citadel_swarm_services::SwarmServiceHealth::Healthy
            };
            let version = i64::try_from(service.version_index).map_err(storage)?;
            sqlx::query("INSERT INTO swarmservices(id,platformid,name,description,dockername,dockerserviceid,dockerversionindex,spec,desiredspechash,lastappliedruntimehash,health,synchronizationstate,controlstate,rowversion,createdbyactorid,createdat,updatedat,autoupdatestate_lastcheckedat,autoupdatestate_status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'DesiredChangesPending','Idle',0,$12,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,'-infinity','Unknown')")
                .bind(new_id).bind(platform).bind(&input.name).bind(&input.description).bind(&service.name).bind(id).bind(version)
                .bind(input.spec.to_storage_value()?).bind(input.spec.desired_hash()).bind(&service.runtime_hash).bind(health.as_str()).bind(actor.value())
                .execute(&mut *tx).await.map_err(database_error)?;
            replace_tags(&mut tx, new_id, actor, &input.tag_ids).await?;
            sqlx::query("UPDATE swarmserviceprojections SET swarmserviceid=$3,ownership='CitadelService',ownershipdiagnostic=NULL,runningtaskcount=$4,desiredtaskcount=$5,versionindex=$6,updatestate=$7,updatemessage=$8,liveruntimehash=$9 WHERE platformid=$1 AND dockerserviceid=$2")
                .bind(platform).bind(id).bind(new_id).bind(service.running_task_count).bind(service.desired_task_count)
                .bind(version).bind(&service.update_state).bind(&service.update_message).bind(&service.runtime_hash).execute(&mut *tx).await.map_err(storage)?;
            insert_swarm_activity(
                &mut tx,
                new_id,
                &input.name,
                platform,
                actor,
                ActivityEventInfo::SwarmServiceAdopted {
                    service: SwarmServiceActivitySnapshot {
                        id: new_id,
                        platform_id: platform,
                        name: input.name.clone(),
                        description: input.description.clone(),
                        docker_name: service.name,
                        docker_service_id: Some(id.into()),
                        spec: input.spec.to_storage_value()?,
                    },
                    docker_service_id: id.into(),
                },
                ActivityStatus::Information,
            )
            .await?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor, admin, new_id).await
        })
    }
}

fn repository(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches('/').to_ascii_lowercase();
    if value.is_empty() || value.starts_with("sha256:") {
        return None;
    }
    let value = value.split('@').next().unwrap_or_default();
    let value = match value.rfind(':') {
        Some(i) if value.rfind('/').is_none_or(|s| i > s) => &value[..i],
        _ => value,
    };
    let Some((host, _)) = value.split_once('/') else {
        return Some(format!("docker.io/library/{value}"));
    };
    Some(if host == "index.docker.io" || host == "docker.io" {
        let path = value.split_once('/').unwrap().1;
        if path.contains('/') {
            format!("docker.io/{path}")
        } else {
            format!("docker.io/library/{path}")
        }
    } else if host.contains('.') || host.contains(':') || host == "localhost" {
        value.into()
    } else {
        format!("docker.io/{value}")
    })
}

fn definition(service: &SwarmServiceMessage) -> Result<SwarmServiceSpec, SwarmServiceError> {
    let d = service
        .definition
        .as_ref()
        .ok_or_else(|| validation("Docker did not return a Service definition."))?;
    fn parse<T: serde::de::DeserializeOwned>(v: &str) -> Result<T, SwarmServiceError> {
        serde_json::from_value(Value::String(v.into()))
            .map_err(|_| validation("Unsupported Service setting in Docker inspection."))
    }
    Ok(SwarmServiceSpec {
        image: SwarmServiceImageInfo::External {
            registry_id: Uuid::nil(),
            image_tag: d.image.clone(),
            resolved_digest: None,
        },
        update_behavior: UpdateBehavior::Disabled,
        scheduling_mode: parse(&d.scheduling_mode)?,
        replicas: d.replicas,
        command: d.command.clone(),
        arguments: d.arguments.clone(),
        environment: d.environment.clone(),
        labels: service
            .labels
            .iter()
            .filter(|(k, _)| !k.to_ascii_lowercase().starts_with("com.citadel."))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        user: (!d.user.is_empty()).then(|| d.user.clone()),
        working_directory: (!d.working_directory.is_empty()).then(|| d.working_directory.clone()),
        health_check: d.health_check.as_ref().map(|h| SwarmServiceHealthCheck {
            test: h.test.clone(),
            interval_nanoseconds: h.interval_nanoseconds,
            timeout_nanoseconds: h.timeout_nanoseconds,
            retries: h.retries,
            start_period_nanoseconds: h.start_period_nanoseconds,
        }),
        stop_grace_period_nanoseconds: d.stop_grace_period_nanoseconds,
        ports: d
            .ports
            .iter()
            .map(|p| {
                Ok(SwarmServicePort {
                    target_port: p.target_port,
                    published_port: p.published_port,
                    protocol: p.protocol.clone(),
                    publish_mode: parse(&p.publish_mode)?,
                })
            })
            .collect::<Result<_, SwarmServiceError>>()?,
        network_ids: d.network_ids.clone(),
        mounts: d
            .mounts
            .iter()
            .map(|m| {
                Ok(SwarmServiceMount {
                    kind: parse(&m.kind)?,
                    source: m.source.clone(),
                    target: m.target.clone(),
                    read_only: m.read_only,
                })
            })
            .collect::<Result<_, SwarmServiceError>>()?,
        secrets: d
            .secrets
            .iter()
            .map(|s| SwarmServiceSecretReference {
                secret_id: s.id.clone(),
                secret_name: s.name.clone(),
                target_name: s.target_name.clone(),
            })
            .collect(),
        configs: d
            .configs
            .iter()
            .map(|s| SwarmServiceConfigReference {
                config_id: s.id.clone(),
                config_name: s.name.clone(),
                target_name: s.target_name.clone(),
            })
            .collect(),
        resources: d.resources.as_ref().map(|r| SwarmServiceResources {
            limit_nano_cpus: r.limit_nano_cpus,
            limit_memory_bytes: r.limit_memory_bytes,
            reservation_nano_cpus: r.reservation_nano_cpus,
            reservation_memory_bytes: r.reservation_memory_bytes,
        }),
        placement_constraints: d.placement_constraints.clone(),
        restart_policy: d
            .restart_policy
            .as_ref()
            .map(|r| {
                Ok(SwarmServiceRestartPolicy {
                    condition: parse(&r.condition)?,
                    delay_nanoseconds: r.delay_nanoseconds,
                    maximum_attempts: r.maximum_attempts,
                    window_nanoseconds: r.window_nanoseconds,
                })
            })
            .transpose()?,
        update_policy: d
            .update_policy
            .as_ref()
            .map(|u| {
                Ok(SwarmServiceUpdatePolicy {
                    parallelism: u.parallelism,
                    delay_nanoseconds: u.delay_nanoseconds,
                    order: parse(&u.order)?,
                    failure_action: parse(&u.failure_action)?,
                })
            })
            .transpose()?,
        webhook: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_repository_check_accepts_tags_and_digests_but_not_unrelated_images() {
        for image in [
            "redis",
            "redis:latest",
            "redis@sha256:abc",
            "docker.io/library/redis:7",
            "index.docker.io/library/redis",
            "docker.io/redis",
        ] {
            assert_eq!(
                repository(image),
                Some("docker.io/library/redis".into()),
                "{image}"
            );
        }
        assert_ne!(repository("redis"), repository("nginx"));
        assert_ne!(repository("example.org/redis"), repository("redis"));
        assert_eq!(repository("sha256:abc"), None);
        assert_eq!(repository(""), None);
        assert_eq!(
            repository("localhost:5000/team/redis:7"),
            Some("localhost:5000/team/redis".into())
        );
    }
}

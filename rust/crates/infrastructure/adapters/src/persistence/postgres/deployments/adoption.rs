use super::*;
use crate::connectors::containers::ownership::{Owner, owner};
use crate::connectors::routing::containers::ContainerRuntimeRouter;
use crate::connectors::routing::containers::Runtime;
use citadel_deployments::adoption::*;
use citadel_identity::SecretProtector;
use citadel_platforms::{PlatformReader, containers::ContainerTarget};
use citadel_primitives::AuthorizedResource;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

pub struct PostgresContainerAdoption {
    pool: PgPool,
    runtime: ContainerRuntimeRouter,
    protector: Arc<dyn SecretProtector>,
    key: Zeroizing<Vec<u8>>,
}
struct Context {
    source: AdoptionSource,
    target: ContainerTarget,
    image_id: Option<Uuid>,
    image_tags: Value,
    inspection: Value,
    defaults: Option<Value>,
}

impl PostgresContainerAdoption {
    pub fn new(
        pool: PgPool,
        runtime: ContainerRuntimeRouter,
        protector: Arc<dyn SecretProtector>,
        key: &[u8],
    ) -> Self {
        Self {
            pool,
            runtime,
            protector,
            key: Zeroizing::new(key.to_vec()),
        }
    }
    async fn load(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        cancel: &CancellationToken,
    ) -> Result<Context, DeploymentError> {
        let row=sqlx::query("SELECT c.*,p.name platformname,p.status platformstatus,p.platformdescriptor,i.id originalimageid,i.tags imagetags FROM containers c JOIN platforms p ON p.id=c.platformid LEFT JOIN images i ON i.platformid=c.platformid AND i.dockerimageid=c.dockerimageid WHERE c.id=$1")
            .bind(id).fetch_optional(&self.pool).await.map_err(storage)?.ok_or(DeploymentError::NotFound)?;
        let platform_id: Uuid = row.get("platformid");
        self.authorize(actor, administrator, platform_id).await?;
        eligible(&row)?;
        let target = ContainerTarget {
            id,
            platform_id,
            docker_id: row.get("dockercontainerid"),
            node_id: None,
        };
        let runtime = self
            .runtime
            .resolve(&target, cancel)
            .await
            .map_err(runtime_error)?;
        let inspection = match &runtime {
            Runtime::Local(docker) => docker
                .inspect_container_document(&target.docker_id)
                .await
                .map_err(|_| DeploymentError::Runtime("Container inspection failed.".into()))?,
            Runtime::Agent(agent) => serde_json::to_value(
                agent
                    .inspect_container(&target.docker_id, cancel)
                    .await
                    .map_err(runtime_error)?,
            )
            .map_err(|_| DeploymentError::Runtime("Invalid Container inspection.".into()))?,
            Runtime::Edge(edge) => {
                let response:citadel_contracts::citadel::shared_models::v1::InspectContainerResponse=crate::connectors::agent::execution::unary(&edge.session,citadel_contracts::citadel::edge::v1::EdgeCommandKind::ContainerInspect,citadel_contracts::citadel::containers::v1::InspectContainerRequest{container_id:target.docker_id.clone()},cancel).await.map_err(runtime_error)?;
                serde_json::to_value(response)
                    .map_err(|_| DeploymentError::Runtime("Invalid Container inspection.".into()))?
            }
        };
        if text(&inspection, "Id") != target.docker_id {
            return Err(DeploymentError::Conflict(
                "Container identity changed. Reload the adoption draft.".into(),
            ));
        }
        ensure_orphaned_owner(&self.pool, &inspection).await?;
        let image_id: Option<Uuid> = row.get("originalimageid");
        let defaults = if image_id.is_some() {
            self.image_defaults(&target, &row.get::<String, _>("dockerimageid"), cancel)
                .await
                .ok()
        } else {
            None
        };
        Ok(Context {
            source: AdoptionSource {
                id,
                docker_container_id: target.docker_id.clone(),
                name: row.get("name"),
                platform_id,
                platform_name: row.get("platformname"),
                state: row.get("state"),
            },
            target,
            image_id,
            image_tags: row
                .get::<Option<Value>, _>("imagetags")
                .unwrap_or(Value::Null),
            inspection,
            defaults,
        })
    }
    async fn authorize(
        &self,
        actor: ActorId,
        administrator: bool,
        platform_id: Uuid,
    ) -> Result<(), DeploymentError> {
        if administrator {
            return Ok(());
        }
        let permissions =
            crate::persistence::postgres::platforms::PostgresPlatformReader::new(self.pool.clone())
                .permissions_for_platforms(actor, &[platform_id])
                .await
                .map_err(|_| DeploymentError::Forbidden)?;
        if !permissions.get(&platform_id).is_some_and(|p| {
            p.level_mask >= PermissionLevel::Read as i32 && p.specific_mask & 2 != 0
        }) {
            return Err(DeploymentError::Forbidden);
        }
        let mut tx = self.pool.begin().await.map_err(storage)?;
        ensure_access(
            &mut tx,
            actor,
            administrator,
            Uuid::nil(),
            citadel_deployments::permissions::CreateDeployment::REQUIREMENT,
        )
        .await
    }
    async fn image_defaults(
        &self,
        target: &ContainerTarget,
        image: &str,
        cancel: &CancellationToken,
    ) -> Result<Value, DeploymentError> {
        let runtime = self
            .runtime
            .resolve(target, cancel)
            .await
            .map_err(runtime_error)?;
        let document = match runtime {
            Runtime::Local(docker) => docker
                .inspect_image_document(image)
                .await
                .map_err(|_| DeploymentError::Runtime("Image inspection failed.".into()))?,
            Runtime::Agent(agent) => agent
                .inspect_image_document(image, cancel)
                .await
                .map_err(runtime_error)?,
            Runtime::Edge(edge) => {
                let response: citadel_contracts::citadel::images::v1::InspectImageResponse =
                    crate::connectors::agent::execution::unary(
                        &edge.session,
                        citadel_contracts::citadel::edge::v1::EdgeCommandKind::ImageInspect,
                        citadel_contracts::citadel::images::v1::InspectImageRequest {
                            id: image.into(),
                        },
                        cancel,
                    )
                    .await
                    .map_err(runtime_error)?;
                serde_json::to_value(response)
                    .map_err(|_| DeploymentError::Runtime("Invalid Image inspection.".into()))?
            }
        };
        if text(&document, "Id") != image {
            return Err(DeploymentError::Conflict("Image identity changed.".into()));
        }
        Ok(if field(&document, "Config").is_null() {
            document
        } else {
            field(&document, "Config").clone()
        })
    }
    fn fingerprint(&self, context: &Context) -> Result<String, DeploymentError> {
        let value = serde_json::json!([
            context.source.id,
            context.source.platform_id,
            context.source.docker_container_id,
            context.source.state,
            context.image_id,
            context.image_tags,
            field(&context.inspection, "Config"),
            field(&context.inspection, "HostConfig"),
            field(&context.inspection, "Mounts"),
            field(&context.inspection, "NetworkSettings")
        ]);
        let bytes =
            Zeroizing::new(serde_json::to_vec(&value).map_err(|_| {
                DeploymentError::Runtime("Cannot fingerprint adoption draft.".into())
            })?);
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key)
            .map_err(|_| DeploymentError::Runtime("Adoption signing is unavailable.".into()))?;
        mac.update(b"citadel-container-adoption-v1\0");
        mac.update(&bytes);
        Ok(mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }
    async fn available_name(&self, name: &str, platform: Uuid) -> Result<String, DeploymentError> {
        let base = draft_name(name);
        for suffix in 1..=100 {
            let candidate = if suffix == 1 {
                base.clone()
            } else {
                format!("{}-{suffix}", &base[..base.len().min(59)])
            };
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM deployments WHERE platformid=$1 AND lower(name)=lower($2))").bind(platform).bind(&candidate).fetch_one(&self.pool).await.map_err(storage)?;
            if !exists {
                return Ok(candidate);
            }
        }
        Ok(format!(
            "{}-{}",
            &base[..base.len().min(27)],
            Uuid::now_v7().simple()
        ))
    }
}

impl ContainerAdoptionPort for PostgresContainerAdoption {
    fn draft<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ContainerAdoptionDraft, DeploymentError>> {
        Box::pin(async move {
            let context = self.load(actor, administrator, id, cancel).await?;
            let name = self
                .available_name(&context.source.name, context.source.platform_id)
                .await?;
            let fingerprint = self.fingerprint(&context)?;
            map_draft(
                context.source,
                &context.inspection,
                context.image_id,
                context.defaults.as_ref(),
                fingerprint,
                name,
            )
        })
    }
    fn adopt<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        mut input: AdoptContainer,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
        Box::pin(async move {
            let mut context = self.load(actor, administrator, id, cancel).await?;
            let fingerprint = self.fingerprint(&context)?;
            if !bool::from(
                fingerprint
                    .as_bytes()
                    .ct_eq(input.preview_fingerprint.as_bytes()),
            ) {
                return Err(DeploymentError::Conflict("Container configuration changed. Reload the adoption draft and review it again.".into()));
            }
            match &input.spec.image {
                DeploymentImageInfo::Local { image_id } => {
                    let image_id = Uuid::parse_str(image_id).map_err(|_| {
                        DeploymentError::Validation("Select a valid local image.".into())
                    })?;
                    let (docker_id, tags): (String, Value) = sqlx::query_as(
                        "SELECT dockerimageid,tags FROM images WHERE id=$1 AND platformid=$2",
                    )
                    .bind(image_id)
                    .bind(context.source.platform_id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(storage)?
                    .ok_or(DeploymentError::NotFound)?;
                    if Some(image_id) != context.image_id
                        && !same_repository(&context, &strings(&tags))
                    {
                        return Err(DeploymentError::Validation(
                            "Select a replacement from the container's original image repository."
                                .into(),
                        ));
                    }
                    context.defaults = Some(
                        self.image_defaults(&context.target, &docker_id, cancel)
                            .await?,
                    );
                    context.image_id = Some(image_id);
                }
                DeploymentImageInfo::External {
                    registry_id,
                    image_tag,
                    ..
                } => {
                    let host: String = sqlx::query_scalar(
                        "SELECT registryhost FROM registries WHERE id=$1 AND status != 'Disabled'",
                    )
                    .bind(registry_id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(storage)?
                    .ok_or(DeploymentError::NotFound)?;
                    let reference =
                        crate::connectors::routing::deployments::qualify_image_reference(
                            &host, image_tag,
                        )?;
                    if !same_repository(&context, &[reference]) {
                        return Err(DeploymentError::Validation("Select an external image from the container's original image repository.".into()));
                    }
                }
                DeploymentImageInfo::Build { .. } if context.image_id.is_none() => {
                    return Err(DeploymentError::Validation(
                        "Select a local replacement or an external image before adopting.".into(),
                    ));
                }
                _ => {}
            }
            let draft = map_draft(
                context.source.clone(),
                &context.inspection,
                context.image_id,
                context.defaults.as_ref(),
                fingerprint,
                input.name.clone(),
            )?;
            if let Some(blocker) = draft
                .issues
                .iter()
                .find(|issue| issue.severity == "Blocker")
            {
                return Err(DeploymentError::Validation(blocker.message.clone()));
            }
            let sensitive: Vec<_> = sensitive_values(&context.inspection)
                .into_iter()
                .map(|(name, value)| (name, Zeroizing::new(value)))
                .collect();
            let mut encrypted = Vec::new();
            if input.import_sensitive_environment_as_secrets {
                if !draft.can_import_sensitive_environment_values {
                    return Err(DeploymentError::Validation(
                        "Sensitive environment values cannot be imported.".into(),
                    ));
                }
                let env = input.spec.environment_variables.get_or_insert_default();
                for (name, value) in sensitive.iter() {
                    env.retain(|entry| {
                        entry
                            .split_once('=')
                            .map_or(entry.as_str(), |(name, _)| name)
                            != name
                    });
                    env.push(format!("{name}=${{{name}}}"));
                    encrypted.push((
                        name.clone(),
                        self.protector.protect(value.as_bytes()).map_err(|_| {
                            DeploymentError::Runtime("Could not encrypt imported secret.".into())
                        })?,
                    ));
                }
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            // Reconciliation locks Platform before child rows; retain that order.
            sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR SHARE")
                .bind(context.source.platform_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(DeploymentError::NotFound)?;
            ensure_platform(&mut tx, actor, administrator, context.source.platform_id).await?;
            let locked=sqlx::query("SELECT c.*,p.status platformstatus,p.platformdescriptor FROM containers c JOIN platforms p ON p.id=c.platformid WHERE c.id=$1 FOR UPDATE OF c")
                .bind(id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or(DeploymentError::NotFound)?;
            eligible(&locked)?;
            ensure_orphaned_owner(&mut *tx, &context.inspection).await?;
            if locked.get::<String, _>("dockercontainerid") != context.target.docker_id
                || locked.get::<String, _>("state") != context.source.state
            {
                return Err(DeploymentError::Conflict(
                    "Container changed during adoption.".into(),
                ));
            }
            validate_tags(&mut tx, &input.tag_ids).await?;
            validate_image_references(&mut tx, &input.spec.image).await?;
            let globals: Vec<String> =
                sqlx::query_scalar("SELECT name FROM resourcebindings WHERE scope='Global'")
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(storage)?;
            if !input.import_sensitive_environment_as_secrets {
                for entry in strings(field(field(&context.inspection, "Config"), "Env")) {
                    let name = entry
                        .split_once('=')
                        .map_or(entry.as_str(), |(name, _)| name);
                    if !citadel_deployments::adoption::sensitive(name) {
                        continue;
                    }
                    if !has_resolved_sensitive_environment(
                        input
                            .spec
                            .environment_variables
                            .as_deref()
                            .unwrap_or_default(),
                        name,
                        &globals,
                    ) {
                        return Err(DeploymentError::Validation(format!(
                            "Enter a value or existing binding for sensitive environment variable '{name}'."
                        )));
                    }
                }
            }
            if input
                .spec
                .environment_variables
                .as_ref()
                .into_iter()
                .flatten()
                .any(|e| e.ends_with("=********"))
            {
                return Err(DeploymentError::Validation(
                    "Redacted environment placeholders cannot be saved.".into(),
                ));
            }
            let deployment_id = Uuid::now_v7();
            let now = Utc::now();
            let spec = input.spec.to_storage_value()?;
            let status = match context.source.state.to_ascii_lowercase().as_str() {
                "running" => "Healthy",
                "paused" | "restarting" => "Pending",
                "created" => "Created",
                "dead" | "offline" => "Degraded",
                "exited" => "Stopped",
                _ => "Failed",
            };
            sqlx::query("INSERT INTO deployments (id,name,description,platformid,status,createdat,createdbyactorid,spec,autoupdatestate_lastcheckedat,autoupdatestate_status,controlstate,rowversion) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'-infinity','Unknown','Idle',0)")
                .bind(deployment_id).bind(&input.name).bind(&input.description).bind(context.source.platform_id).bind(status).bind(now).bind(actor.value()).bind(&spec).execute(&mut *tx).await.map_err(database_error)?;
            insert_tags(&mut tx, deployment_id, actor, &input.tag_ids).await?;
            for (name, value) in encrypted {
                let secret_id = Uuid::now_v7();
                let secret_name = format!("ADOPTED_{}_{secret_id}", &name[..name.len().min(80)]);
                sqlx::query("INSERT INTO secretdefinitions(id,name,providertype,createdat,updatedat) VALUES($1,$2,'InternalEncrypted',$3,$3)").bind(secret_id).bind(secret_name).bind(now).execute(&mut *tx).await.map_err(database_error)?;
                sqlx::query("INSERT INTO internalsecretvalues(secretid,encryptedvalue,createdat,updatedat) VALUES($1,$2,$3,$3)").bind(secret_id).bind(value).bind(now).execute(&mut *tx).await.map_err(storage)?;
                sqlx::query("INSERT INTO resourcebindings(id,name,kind,scope,resourceid,secretid,secretdeliverymode,createdat,updatedat) VALUES($1,$2,'Secret','Deployment',$3,$4,'EnvironmentVariable',$5,$5)").bind(Uuid::now_v7()).bind(name).bind(deployment_id).bind(secret_id).bind(now).execute(&mut *tx).await.map_err(database_error)?;
            }
            sqlx::query(
                "UPDATE containers SET deploymentid=$2,rowversion=rowversion+1 WHERE id=$1",
            )
            .bind(id)
            .bind(deployment_id)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
            insert_deployment_activity(
                &mut tx,
                deployment_id,
                &input.name,
                context.source.platform_id,
                actor,
                ActivityEventInfo::DeploymentAdopted {
                    deployment: snapshot(
                        deployment_id,
                        &input.name,
                        context.source.platform_id,
                        input.description,
                        spec,
                    ),
                    container_id: context.target.docker_id.clone(),
                    container_name: context.source.name,
                },
                now,
            )
            .await?;
            sqlx::query("UPDATE alertevents SET resolvedat=$3,resolvedbyactorid=$2,resolutionnote='Container adopted as a deployment.',openincidentkey=NULL,updatedat=$3 WHERE unmanagedcontainerid=$1 AND resolvedat IS NULL AND info->>'PlatformId'=$4")
                .bind(&context.target.docker_id).bind(actor.value()).bind(now).bind(context.source.platform_id.to_string()).execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            get_authorized(&self.pool, actor, administrator, deployment_id).await
        })
    }
}
fn eligible(row: &PgRow) -> Result<(), DeploymentError> {
    if row.get::<bool, _>("issystem") || row.get::<bool, _>("isswarmtask") {
        return Err(DeploymentError::Conflict(
            "System or Swarm task containers cannot be adopted.".into(),
        ));
    }
    if row.get::<Option<Uuid>, _>("deploymentid").is_some()
        || row.get::<Option<Uuid>, _>("stackid").is_some()
    {
        return Err(DeploymentError::Conflict(
            "Container is already managed by Citadel.".into(),
        ));
    }
    if row.get::<Option<String>, _>("controlstate").as_deref() == Some("Processing")
        || row.get::<String, _>("platformstatus") != "Online"
    {
        return Err(DeploymentError::Conflict(
            "Container is processing or its Platform is offline.".into(),
        ));
    }
    if row
        .get::<Option<String>, _>("stack")
        .is_some_and(|name| !name.is_empty())
    {
        return Err(DeploymentError::Validation(
            "Import the Compose project as a Stack instead.".into(),
        ));
    }
    if platform_kind(&row.get::<Value, _>("platformdescriptor")) != "Docker" {
        return Err(DeploymentError::Validation(
            "Only containers on Docker Standalone platforms can be adopted.".into(),
        ));
    }
    Ok(())
}
async fn ensure_orphaned_owner<'e>(
    executor: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    inspection: &Value,
) -> Result<(), DeploymentError> {
    let labels = field(field(inspection, "Config"), "Labels")
        .as_object()
        .map(|labels| {
            labels
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default();
    let Some(owner) =
        owner(&labels).map_err(|message| DeploymentError::Conflict(message.into()))?
    else {
        return Ok(());
    };
    let Owner::Deployment(id) = owner else {
        return Err(DeploymentError::Conflict(
            "Import the former Stack as a Stack instead.".into(),
        ));
    };
    if sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM deployments WHERE id=$1)")
        .bind(id)
        .fetch_one(executor)
        .await
        .map_err(storage)?
    {
        return Err(DeploymentError::Conflict(
            "Container is owned by an existing Citadel Deployment.".into(),
        ));
    }
    Ok(())
}
fn runtime_error(_: citadel_platforms::RuntimeCapabilityError) -> DeploymentError {
    DeploymentError::Runtime(
        "Container adoption inspection failed. Check Platform connectivity.".into(),
    )
}
fn repository(reference: &str) -> Option<String> {
    let mut value = reference
        .trim()
        .trim_start_matches('/')
        .to_ascii_lowercase();
    if value.is_empty() || value.starts_with("sha256:") || value.contains("<none>") {
        return None;
    }
    if let Some(index) = value.rfind('@') {
        value.truncate(index);
    }
    if let Some(index) = value.rfind(':')
        && value.rfind('/').is_none_or(|slash| index > slash)
    {
        value.truncate(index);
    }
    if let Some((host, _)) = value.split_once('/') {
        if host == "index.docker.io" {
            value = value.replacen("index.docker.io", "docker.io", 1);
        } else if !host.contains(['.', ':']) && host != "localhost" {
            value = format!("docker.io/{value}");
        }
    } else {
        value = format!("docker.io/library/{value}");
    }
    Some(value)
}
fn same_repository(context: &Context, references: &[String]) -> bool {
    let originals: Vec<_> = strings(&context.image_tags)
        .into_iter()
        .chain([text(field(&context.inspection, "Config"), "Image").to_owned()])
        .filter_map(|s| repository(&s))
        .collect();
    originals.is_empty()
        || references
            .iter()
            .filter_map(|s| repository(s))
            .any(|repo| originals.contains(&repo))
}

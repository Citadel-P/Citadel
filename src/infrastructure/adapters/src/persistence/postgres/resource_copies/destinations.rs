//! Authorized compatible destinations and immutable reviewed routing identities.
use super::*;

impl PostgresCopyRepository {
    pub(super) async fn list_targets(
        &self,
        actor: ActorId,
        kind: CopyKind,
        id: Uuid,
    ) -> Result<CopyTargets, CopyError> {
        let access = self.access(actor).await?;
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        let source = self.load_source(&mut tx, &access, kind, id).await?;
        let sql = format!(
            r#"{}
            SELECT p.id,p.name FROM platforms p
            WHERE p.platformdescriptor->>'$type'=$5
              AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
                SELECT 1 FROM actor_scope s JOIN resourceaccesses a ON a.actorid=s.actorid
                WHERE a.resourcetype=$2 AND a.resourceid=p.id AND a.permissionlevel=ANY($3)))
            ORDER BY p.name,p.id LIMIT 1001"#,
            super::super::backups::AUTHORIZED_CTE
        );
        let platforms = sqlx::query(AssertSqlSafe(sql.as_str()))
            .bind(actor.value())
            .bind(ResourceType::Platform as i32)
            .bind(PermissionLevel::Write.accepted_database_levels())
            .bind(access.administrator)
            .bind(if source.swarm {
                "DockerSwarm"
            } else {
                "Docker"
            })
            .fetch_all(&mut *tx)
            .await
            .map_err(storage_error)?
            .into_iter()
            .map(|r| {
                Ok(CopyTarget {
                    id: r.try_get("id").map_err(storage_error)?,
                    name: r.try_get("name").map_err(storage_error)?,
                })
            })
            .collect::<Result<Vec<_>, CopyError>>()?;
        if platforms.len() > 1000 {
            return Err(conflict("Destination catalog exceeds 1000 platforms."));
        }
        Ok(CopyTargets {
            source_node_id: match &source.spec {
                CopySpec::SwarmService(s) => planning::pinned_node(s).map(str::to_owned),
                _ => None,
            },
            source_platform_id: source.platform_id,
            source_platform_name: source.platform_name,
            swarm: source.swarm,
            platforms,
        })
    }

    pub(super) async fn prepare_target(
        &self,
        actor: ActorId,
        plan: &mut CopyPlan,
    ) -> Result<(), CopyError> {
        let access = self.access(actor).await?;
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        let mut spec = plan.spec.clone();
        let destination = self
            .resolve_target(&mut tx, &access, plan, &mut spec)
            .await?;
        plan.preview.platform_id = destination.platform_id;
        plan.preview.platform_name = destination.platform_name.clone();
        plan.preview.destination_node_id = destination.docker_node_id.clone();
        plan.destination = Some(destination);
        plan.spec = spec;
        Ok(())
    }

    pub(super) async fn verify_target(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        access: &Access,
        plan: &CopyPlan,
    ) -> Result<(), CopyError> {
        // Older same-platform receipts retain their original execution contract.
        let Some(expected) = &plan.destination else {
            return Ok(());
        };
        let current = self
            .resolve_target(tx, access, plan, &mut plan.spec.clone())
            .await?;
        if current.fingerprint != expected.fingerprint {
            return Err(conflict(
                "Destination identity, placement or referenced resources changed. Review the copy again.",
            ));
        }
        Ok(())
    }

    async fn resolve_target(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        access: &Access,
        plan: &CopyPlan,
        spec: &mut CopySpec,
    ) -> Result<CopyDestination, CopyError> {
        let platform = plan
            .request
            .destination_platform_id
            .unwrap_or(plan.source.platform_id);
        self.require(
            tx,
            access,
            ResourceType::Platform,
            platform,
            PermissionLevel::Write,
            None,
        )
        .await?;
        let row = sqlx::query("SELECT name,platformdescriptor,address,connectortype,clusterid FROM platforms WHERE id=$1 FOR SHARE")
            .bind(platform).fetch_optional(&mut **tx).await.map_err(storage_error)?.ok_or(CopyError::NotFound)?;
        let descriptor: Value = row.try_get("platformdescriptor").map_err(storage_error)?;
        let swarm = match descriptor["$type"].as_str() {
            Some("Docker") => false,
            Some("DockerSwarm") => true,
            _ => {
                return Err(conflict(
                    "The destination is not a supported Docker platform.",
                ));
            }
        };
        let source_node = match &plan.source.spec {
            CopySpec::SwarmService(s) => planning::pinned_node(s),
            _ => None,
        };
        let node = plan.request.destination_node_id.as_deref().or_else(|| {
            (platform == plan.source.platform_id)
                .then_some(source_node)
                .flatten()
        });
        planning::validate_destination(&plan.source, &plan.request, platform, swarm, node)?;
        if platform != plan.source.platform_id || node != source_node {
            if let Some(node) = node {
                let available:bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmnodeprojections WHERE platformid=$1 AND dockernodeid=$2 AND NOT isstale AND lower(status)='ready' AND lower(availability)='active')")
                    .bind(platform).bind(node).fetch_one(&mut **tx).await.map_err(storage_error)?;
                if !available {
                    return Err(conflict(
                        "The destination node is missing, stale, or not Ready and Active.",
                    ));
                }
            }
            remap_references(tx, plan, platform, node, spec).await?;
        }
        let routing = super::super::platforms::descriptor::decode(descriptor.clone())
            .map_err(storage_error)?
            .routing;
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(
                platform,
                node,
                &descriptor["$type"],
                routing.daemon_id,
                row.try_get::<Option<String>, _>("clusterid")
                    .map_err(storage_error)?,
                row.try_get::<String, _>("address").map_err(storage_error)?,
                row.try_get::<String, _>("connectortype")
                    .map_err(storage_error)?,
                &spec,
            ))
            .map_err(storage_error)?,
        )
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
        Ok(CopyDestination {
            platform_id: platform,
            platform_name: row.try_get("name").map_err(storage_error)?,
            docker_node_id: node.map(str::to_owned),
            fingerprint,
        })
    }
}

async fn remap_references(
    tx: &mut Transaction<'_, Postgres>,
    plan: &CopyPlan,
    platform: Uuid,
    node: Option<&str>,
    spec: &mut CopySpec,
) -> Result<(), CopyError> {
    if let (CopySpec::SwarmService(original), CopySpec::SwarmService(destination)) =
        (&plan.source.spec, spec)
    {
        destination.network_ids.clear();
        for id in &original.network_ids {
            destination.network_ids.push(
                match_reference(
                    tx,
                    "swarmnetworkprojections",
                    "dockernetworkid",
                    plan.source.platform_id,
                    platform,
                    id,
                )
                .await?,
            );
        }
        for (target, source) in destination.secrets.iter_mut().zip(&original.secrets) {
            target.secret_id = match_reference(
                tx,
                "swarmsecretprojections",
                "dockersecretid",
                plan.source.platform_id,
                platform,
                &source.secret_id,
            )
            .await?;
        }
        for (target, source) in destination.configs.iter_mut().zip(&original.configs) {
            target.config_id = match_reference(
                tx,
                "swarmconfigprojections",
                "dockerconfigid",
                plan.source.platform_id,
                platform,
                &source.config_id,
            )
            .await?;
        }
        // Only replace the original exact node pin, preserving other user constraints.
        destination.placement_constraints = original
            .placement_constraints
            .iter()
            .map(|c| {
                if c.trim()
                    .strip_prefix("node.id")
                    .is_some_and(|v| v.trim_start().starts_with("=="))
                {
                    format!("node.id == {}", node.unwrap_or_default())
                } else {
                    c.clone()
                }
            })
            .collect();
    }
    Ok(())
}

async fn match_reference(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    key: &str,
    source: Uuid,
    target: Uuid,
    id: &str,
) -> Result<String, CopyError> {
    // Both fragments come only from the closed call sites above.
    let network_filter = if table == "swarmnetworkprojections" {
        " AND target.scope='swarm' AND NOT target.isingress"
    } else {
        ""
    };
    let sql = format!(
        "SELECT target.{key} FROM {table} original JOIN {table} target ON target.name=original.name WHERE original.platformid=$1 AND original.{key}=$2 AND target.platformid=$3 AND NOT original.isstale AND NOT target.isstale{network_filter}"
    );
    let ids: Vec<String> = sqlx::query_scalar(AssertSqlSafe(sql.as_str()))
        .bind(source)
        .bind(id)
        .bind(target)
        .fetch_all(&mut **tx)
        .await
        .map_err(storage_error)?;
    if ids.len() != 1 {
        return Err(conflict(
            "A referenced Swarm network, secret or config has no unique, current counterpart with the same name on the destination.",
        ));
    }
    Ok(ids[0].clone())
}

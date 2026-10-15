use super::*;
impl PostgresCopyRepository {
    pub(super) async fn load_source(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        access: &Access,
        kind: CopyKind,
        id: Uuid,
    ) -> Result<CopySource, CopyError> {
        self.authorize_source(tx, access, kind, id).await?;
        let sql = match kind {
            CopyKind::Deployment => {
                "SELECT d.name,d.description,d.platformid,d.spec,d.rowversion,p.name platformname,p.platformdescriptor,p.address,p.connectortype,p.clusterid FROM deployments d JOIN platforms p ON p.id=d.platformid WHERE d.id=$1 FOR SHARE OF d,p"
            }
            CopyKind::Stack => {
                "SELECT s.name,s.description,r.platformid,r.spec,s.rowversion,p.name platformname,p.platformdescriptor,p.address,p.connectortype,p.clusterid FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1 FOR SHARE OF s,r,p"
            }
            CopyKind::SwarmService => {
                "SELECT s.name,s.description,s.platformid,s.spec,s.rowversion,p.name platformname,p.platformdescriptor,p.address,p.connectortype,p.clusterid FROM swarmservices s JOIN platforms p ON p.id=s.platformid WHERE s.id=$1 FOR SHARE OF s,p"
            }
        };
        let row = sqlx::query(AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(storage_error)?
            .ok_or(CopyError::NotFound)?;
        let platform: Uuid = row.try_get("platformid").map_err(storage_error)?;
        self.require(
            tx,
            access,
            ResourceType::Platform,
            platform,
            PermissionLevel::Write,
            None,
        )
        .await?;
        let raw: Value = row.try_get("spec").map_err(storage_error)?;
        let descriptor: Value = row.try_get("platformdescriptor").map_err(storage_error)?;
        // Inventory counters and health change while restore helpers run. Only
        // target identity and connection configuration invalidate the review.
        let routing = super::super::platforms::descriptor::decode(descriptor.clone())
            .map_err(storage_error)?
            .routing;
        let platform_identity = (
            &descriptor["$type"],
            &routing.daemon_id,
            row.try_get::<Option<String>, _>("clusterid")
                .map_err(storage_error)?,
            row.try_get::<String, _>("address").map_err(storage_error)?,
            row.try_get::<String, _>("connectortype")
                .map_err(storage_error)?,
        );
        let bindings: Value = sqlx::query_scalar("SELECT COALESCE(jsonb_agg(to_jsonb(b) ORDER BY b.id),'[]'::jsonb) FROM resourcebindings b WHERE scope=$1 AND resourceid=$2").bind(kind.as_str()).bind(id).fetch_one(&mut **tx).await.map_err(storage_error)?;
        let tag_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT tagid FROM resourcetags WHERE resourcetype=$1 AND resourceid=$2 ORDER BY tagid",
        )
        .bind(kind.as_str())
        .bind(id)
        .fetch_all(&mut **tx)
        .await
        .map_err(storage_error)?;
        let name: String = row.try_get("name").map_err(storage_error)?;
        let description: Option<String> = row.try_get("description").map_err(storage_error)?;
        // Runtime status updates are not configuration revisions. Hash the
        // reviewed configuration, identity and references rather than health.
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(
                &name,
                &description,
                platform,
                &raw,
                bindings,
                &platform_identity,
                &tag_ids,
            ))
            .map_err(storage_error)?,
        )
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
        let spec = match kind {
            CopyKind::Deployment => CopySpec::Deployment(Box::new(
                citadel_deployments::DeploymentSpec::from_storage_value(raw)
                    .map_err(storage_error)?,
            )),
            CopyKind::Stack => CopySpec::Stack(Box::new(
                citadel_stacks::StackSpec::from_storage_value(raw).map_err(storage_error)?,
            )),
            CopyKind::SwarmService => CopySpec::SwarmService(Box::new(
                citadel_swarm_services::SwarmServiceSpec::from_storage_value(raw)
                    .map_err(storage_error)?,
            )),
        };
        Ok(CopySource {
            kind,
            id,
            platform_id: platform,
            name: row.try_get("name").map_err(storage_error)?,
            description,
            tag_ids,
            platform_name: row.try_get("platformname").map_err(storage_error)?,
            swarm: descriptor["$type"] == "DockerSwarm",
            fingerprint,
            spec,
        })
    }
    pub(super) async fn list_snapshots(
        &self,
        actor: ActorId,
        source: &CopySource,
    ) -> Result<Vec<CopySnapshot>, CopyError> {
        let access = self.access(actor).await?;
        // Keep the same bounded, Read-authorized catalog as Backup Runs, but
        // fetch only restorable snapshot metadata on the source platform.
        let query = format!(
            r#"{}, authorized_runs AS (
            SELECT run.id,run.backuppolicyid,run.backuprepositoryid,
                   run.policynamesnapshot,run.status,run.snapshotavailability
            FROM backupruns run
            WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
                SELECT 1 FROM actor_scope scope
                JOIN resourceaccesses access ON access.actorid=scope.actorid
                WHERE access.resourcetype=$2 AND access.resourceid=run.backuppolicyid
                  AND access.permissionlevel=ANY($3)
            ))
            ORDER BY run.queuedat DESC,run.id DESC LIMIT 1000
        )
        SELECT run.id runid,run.backuppolicyid,run.backuprepositoryid,
               run.policynamesnapshot,item.id itemid,item.volumename,repository.type repositorytype,
               item.dockernodeid,item.completedat
        FROM authorized_runs run
        JOIN backuprunitems item ON item.backuprunid=run.id
        JOIN backuprepositories repository ON repository.id=run.backuprepositoryid
        WHERE run.status IN ('Succeeded','SucceededWithWarnings')
          AND run.snapshotavailability='Available' AND repository.archivedat IS NULL
          AND item.platformid=$5 AND item.hostpath IS NULL
          AND item.status='Succeeded' AND item.resticsnapshotid IS NOT NULL
        ORDER BY item.completedat DESC NULLS LAST,item.id DESC"#,
            super::super::backups::AUTHORIZED_CTE
        );
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        let rows = sqlx::query(AssertSqlSafe(query.as_str()))
            .bind(actor.value())
            .bind(ResourceType::BackupPolicy as i32)
            .bind(PermissionLevel::Read.accepted_database_levels())
            .bind(access.administrator)
            .bind(source.platform_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(storage_error)?;
        let policy_ids = rows
            .iter()
            .map(|row| row.try_get::<Uuid, _>("backuppolicyid"))
            .collect::<Result<std::collections::BTreeSet<_>, _>>()
            .map_err(storage_error)?
            .into_iter()
            .collect::<Vec<_>>();
        let permissions = self
            .authorization
            .resources_in_transaction(
                &mut tx,
                actor,
                ResourceType::BackupPolicy,
                &policy_ids,
                &access.fence,
            )
            .await
            .map_err(storage_error)?;
        let mut snapshots = Vec::new();
        for row in rows {
            let policy_id = row.try_get("backuppolicyid").map_err(storage_error)?;
            if !access.administrator
                && !permissions
                    .get(&policy_id)
                    .copied()
                    .flatten()
                    .is_some_and(|grant| {
                        grant.allows(PermissionRequirement {
                            resource_type: ResourceType::BackupPolicy,
                            level: PermissionLevel::Execute,
                            specific: Some(SpecificPermission::Restore),
                        })
                    })
            {
                continue;
            }
            snapshots.push(CopySnapshot {
                supports_cross_platform_restore: row
                    .try_get::<String, _>("repositorytype")
                    .map_err(storage_error)?
                    == "S3Compatible",
                backup_run_id: row.try_get("runid").map_err(storage_error)?,
                backup_run_item_id: row.try_get("itemid").map_err(storage_error)?,
                policy_id,
                policy_name: row.try_get("policynamesnapshot").map_err(storage_error)?,
                repository_id: row.try_get("backuprepositoryid").map_err(storage_error)?,
                volume: row.try_get("volumename").map_err(storage_error)?,
                docker_node_id: row.try_get("dockernodeid").map_err(storage_error)?,
                completed_at: row.try_get("completedat").map_err(storage_error)?,
            });
        }
        Ok(snapshots)
    }
    pub(super) async fn verify_source(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        access: &Access,
        plan: &CopyPlan,
    ) -> Result<(), CopyError> {
        let current = self
            .load_source(tx, access, plan.source.kind, plan.source.id)
            .await?;
        if current.fingerprint != plan.source.fingerprint {
            return Err(conflict(
                "The source configuration or bindings changed. Review the copy again.",
            ));
        }
        self.verify_target(tx, access, plan).await?;
        Ok(())
    }
}

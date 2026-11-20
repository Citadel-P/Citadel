use super::*;
use std::collections::{BTreeMap, BTreeSet};

impl PostgresCopyRepository {
    pub(super) async fn begin_hint_read(&self) -> Result<Transaction<'_, Postgres>, CopyError> {
        let mut tx = self.pool.begin().await.map_err(storage_error)?;
        // Dropping the optional UI future can precede server-side cancellation.
        // Bound the time its statement can still occupy a pool connection.
        sqlx::query("SET LOCAL statement_timeout = '750ms'")
            .execute(&mut *tx)
            .await
            .map_err(storage_error)?;
        Ok(tx)
    }

    pub(super) async fn read_eligibility_sources(
        &self,
        actor: ActorId,
        kind: CopyKind,
        ids: &[Uuid],
    ) -> Result<Vec<CopyEligibilitySource>, CopyError> {
        let access = self.access(actor).await?;
        let mut tx = self.begin_hint_read().await?;
        // The list caller supplies actor-visible ids. Copying also requires
        // create, binding (when used), and platform write permissions.
        let resource = kind.resource_type();
        let mut workload_ids = ids.to_vec();
        workload_ids.push(Uuid::nil());
        let workload = if access.administrator {
            BTreeMap::new()
        } else {
            self.authorization
                .resources_in_transaction(&mut tx, actor, resource, &workload_ids, &access.fence)
                .await
                .map_err(storage_error)?
        };
        let allowed = |id: Uuid, level, specific| {
            access.administrator
                || workload.get(&id).copied().flatten().is_some_and(|grant| {
                    grant.allows(PermissionRequirement {
                        resource_type: resource,
                        level,
                        specific,
                    })
                })
        };
        if !allowed(Uuid::nil(), PermissionLevel::Write, None) {
            return Ok(Vec::new());
        }
        let readable: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| allowed(*id, PermissionLevel::Read, None))
            .collect();
        let (from, config) = match kind {
            CopyKind::Deployment => ("deployments s", "s"),
            CopyKind::Stack => (
                "stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid",
                "r",
            ),
            CopyKind::SwarmService => ("swarmservices s", "s"),
        };
        let query = format!(
            "SELECT s.id,s.name,{config}.spec,p.id platformid,
            COALESCE(p.platformdescriptor->>'$type'='DockerSwarm',false) swarm,
            EXISTS(SELECT 1 FROM resourcebindings b WHERE b.scope=$2 AND b.resourceid=s.id) bound
            FROM {from} JOIN platforms p ON p.id={config}.platformid WHERE s.id=ANY($1)"
        );
        let rows = sqlx::query(AssertSqlSafe(query.as_str()))
            .bind(readable)
            .bind(kind.as_str())
            .fetch_all(&mut *tx)
            .await
            .map_err(storage_error)?;
        let platforms: BTreeSet<Uuid> = rows.iter().map(|r| r.get("platformid")).collect();
        let grants = if access.administrator {
            BTreeMap::new()
        } else {
            self.authorization
                .resources_in_transaction(
                    &mut tx,
                    actor,
                    ResourceType::Platform,
                    &platforms.into_iter().collect::<Vec<_>>(),
                    &access.fence,
                )
                .await
                .map_err(storage_error)?
        };
        let mut sources = Vec::new();
        for (index, row) in rows.into_iter().enumerate() {
            if index % 32 == 0 {
                tokio::task::yield_now().await;
            }
            let id = row.get("id");
            let binding = Some(SpecificPermission::ResourceBindings);
            if row.get("bound")
                && !(allowed(id, PermissionLevel::Read, binding)
                    && allowed(Uuid::nil(), PermissionLevel::Write, binding))
            {
                continue;
            }
            let platform_id = row.get("platformid");
            let platform_allows = |level, specific| {
                access.administrator
                    || grants
                        .get(&platform_id)
                        .copied()
                        .flatten()
                        .is_some_and(|grant| {
                            grant.allows(PermissionRequirement {
                                resource_type: ResourceType::Platform,
                                level,
                                specific,
                            })
                        })
            };
            if !platform_allows(PermissionLevel::Write, None) {
                continue;
            }
            sources.push(CopyEligibilitySource {
                id,
                name: row.get("name"),
                platform_id,
                swarm: row.get("swarm"),
                spec: sources::decode_spec(kind, row.get("spec"))?,
                current_data_allowed: platform_allows(
                    PermissionLevel::Read,
                    Some(SpecificPermission::Download),
                ),
            });
        }
        tx.commit().await.map_err(storage_error)?;
        Ok(sources)
    }
}

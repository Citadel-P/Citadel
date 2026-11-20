use super::*;
use citadel_identity::IdentityError;
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermissions};
use citadel_runtime::runtime_metrics::RuntimeWork;
use sqlx::{Executor, Postgres, Row, Transaction};
use std::collections::BTreeSet;

#[derive(sqlx::FromRow)]
struct StoredScope {
    actors: Vec<Uuid>,
    role_grants: Vec<(i32, i32, i32)>,
    has_resource_grants: bool,
}

fn storage(error: sqlx::Error) -> IdentityError {
    IdentityError::Storage(error.to_string())
}
fn grant(kind: i32, level: i32, specifics: i32) -> Result<PermissionGrant, IdentityError> {
    Ok(PermissionGrant {
        resource_type: ResourceType::from_i32(kind).ok_or_else(|| {
            IdentityError::Storage(format!("unknown persisted ResourceType value {kind}"))
        })?,
        level: PermissionLevel::from_i32(level).ok_or_else(|| {
            IdentityError::Storage(format!("unknown persisted PermissionLevel value {level}"))
        })?,
        specifics: SpecificPermissions::from_bits_retain(specifics as u32),
    })
}
fn merge(left: Option<PermissionGrant>, right: Option<PermissionGrant>) -> Option<PermissionGrant> {
    match (left, right) {
        (Some(a), Some(b)) => Some(PermissionGrant {
            level: if a.level.grants(b.level) {
                a.level
            } else {
                b.level
            },
            specifics: SpecificPermissions::from_bits_retain(
                a.specifics.bits() | b.specifics.bits(),
            ),
            ..a
        }),
        (a, b) => a.or(b),
    }
}
impl AuthorizationCache {
    async fn scope<'e>(
        &self,
        db: impl Executor<'e, Database = Postgres>,
        actor: ActorId,
    ) -> Result<(Uuid, Arc<ActorScope>), IdentityError> {
        let _lookup = RuntimeWork::AuthorizationScopeLookup.start();
        let generation = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let entry = state.actor(actor.value());
            let generation = *entry.generation.borrow();
            if let Some(scope) = &entry.loaded
                && scope.expires > Instant::now()
            {
                RuntimeWork::AuthorizationScopeLookup.units(1);
                return Ok((generation, scope.clone()));
            }
            generation
        };
        let query_timer = RuntimeWork::AuthorizationScopeQuery.start();
        let scope: StoredScope = sqlx::query_as(include_str!("actor_scope.sql"))
            .bind(actor.value())
            .fetch_one(db)
            .await
            .map_err(storage)?;
        drop(query_timer);
        let mut grants = BTreeMap::new();
        for (kind, level, specifics) in scope.role_grants {
            let value = grant(kind, level, specifics)?;
            grants.insert(
                kind,
                merge(grants.get(&kind).copied(), Some(value)).unwrap(),
            );
        }
        let scope = Arc::new(ActorScope {
            snapshot: AuthorizationSnapshot {
                actor_id: actor,
                enabled: scope.actors.contains(&actor.value()),
                direct_and_team_permissions: grants.into_values().collect(),
            },
            actors: scope.actors,
            has_resource_grants: scope.has_resource_grants,
            expires: Instant::now() + SAFETY_TTL,
        });
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        // Capacity eviction can occur during a read; never publish under a
        // replacement generation. The commit gate fences mutation races.
        if let Some(entry) = state.actors.get_mut(&actor.value())
            && *entry.generation.borrow() == generation
        {
            entry.loaded = Some(scope.clone());
        }
        Ok((generation, scope))
    }
    pub(crate) async fn snapshot(
        &self,
        pool: &PgPool,
        actor: ActorId,
    ) -> Result<AuthorizationSnapshot, IdentityError> {
        let _read = self.gate.read().await;
        Ok(self.scope(pool, actor).await?.1.snapshot.clone())
    }
    pub(crate) async fn resources(
        &self,
        pool: &PgPool,
        actor: ActorId,
        kind: ResourceType,
        ids: &[Uuid],
    ) -> Result<BTreeMap<Uuid, Option<PermissionGrant>>, IdentityError> {
        if ids.is_empty() {
            return Ok(BTreeMap::new());
        }
        let _read = self.gate.read().await;
        let (generation, scope) = self.scope(pool, actor).await?;
        self.resolve_resources(pool, generation, &scope, kind, ids)
            .await
    }

    /// Read a permission batch through the caller's claim transaction while a
    /// read fence is held. This keeps cold cache fills on the same connection
    /// and lets ACL mutations wait until the durable claim commits.
    pub(crate) async fn resources_in_transaction(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: ActorId,
        kind: ResourceType,
        ids: &[Uuid],
        _fence: &super::ReadFence,
    ) -> Result<BTreeMap<Uuid, Option<PermissionGrant>>, IdentityError> {
        if ids.is_empty() {
            return Ok(BTreeMap::new());
        }
        let (generation, scope) = self.scope(&mut **tx, actor).await?;
        self.resolve_resources(&mut **tx, generation, &scope, kind, ids)
            .await
    }

    async fn resolve_resources<'e>(
        &self,
        db: impl Executor<'e, Database = Postgres>,
        generation: Uuid,
        scope: &ActorScope,
        kind: ResourceType,
        ids: &[Uuid],
    ) -> Result<BTreeMap<Uuid, Option<PermissionGrant>>, IdentityError> {
        if !scope.snapshot.enabled {
            return Ok(ids.iter().map(|&id| (id, None)).collect());
        }
        let global = scope
            .snapshot
            .direct_and_team_permissions
            .iter()
            .find(|grant| grant.resource_type == kind)
            .copied();
        if !scope.has_resource_grants {
            return Ok(ids.iter().map(|&id| (id, global)).collect());
        }

        let actor = scope.snapshot.actor_id.value();
        let ids: BTreeSet<_> = ids.iter().copied().collect();
        let mut result = BTreeMap::new();
        let mut misses = Vec::new();
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            for id in ids {
                let _lookup = RuntimeWork::AuthorizationResourceLookup.start();
                match state.resource((actor, kind as i32, id), generation) {
                    Some(grant) => {
                        RuntimeWork::AuthorizationResourceLookup.units(1);
                        if grant.is_none() {
                            RuntimeWork::AuthorizationNegativeHit.units(1);
                        }
                        result.insert(id, merge(global, grant));
                    }
                    _ => misses.push(id),
                }
            }
        }
        if misses.is_empty() {
            return Ok(result);
        }
        // Only ACL contributions: role/membership expansion was loaded once in
        // scope(). Every missing ID (including denials) is cached below.
        let query_timer = RuntimeWork::AuthorizationResourceQuery.start();
        let rows = sqlx::query("SELECT resourceid, permissionlevel, specificpermissions FROM resourceaccesses WHERE actorid=ANY($1) AND resourcetype=$2 AND resourceid=ANY($3)")
            .bind(&scope.actors).bind(kind as i32).bind(&misses).fetch_all(db).await.map_err(storage)?;
        drop(query_timer);
        let mut acl = BTreeMap::new();
        for row in rows {
            let id: Uuid = row.try_get("resourceid").map_err(storage)?;
            let value = grant(
                kind as i32,
                row.try_get("permissionlevel").map_err(storage)?,
                row.try_get("specificpermissions").map_err(storage)?,
            )?;
            acl.insert(id, merge(acl.get(&id).copied(), Some(value)).unwrap());
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        for id in misses {
            let grant = acl.get(&id).copied();
            state.put_resource((actor, kind as i32, id), generation, grant, scope.expires);
            result.insert(id, merge(global, grant));
        }
        Ok(result)
    }
}

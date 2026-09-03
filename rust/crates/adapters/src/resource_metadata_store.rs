use chrono::Utc;
use citadel_domain::{
    ActivityEvent, ActivityEventInfo, ActorId, GitRepositoryActivitySnapshot,
    RegistryActivitySnapshot, ResourceType,
};
use citadel_resources::{
    CatalogMutationKind, ExternalSecretInput, ExternalSecretPatch, GitRepositoryPatch,
    GitRepositorySyncMode, GitRepositoryView, NewGitRepository, NewRegistry, NewResourceBinding,
    NewTag, RegistryPatch, RegistryStatus, RegistryView, ResourceBindingInput, ResourceBindingKind,
    ResourceBindingScope, ResourceBindingView, ResourceBindingsView, ResourceMetadataError,
    ResourceMetadataStore, SecretDefinitionView, SecretDeliveryMode, SecretProviderType,
    SecretProviderView, StoredSecretProviderInput, StoredSecretProviderPatch, TagPatch, TagSummary,
    TagView, TaggableResourceType, registry_type, validate_description, validate_name_identifier,
};
use futures_util::future::BoxFuture;
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::insert_activity as insert_typed_activity;

const READ_MASK: i32 = 1 | 2 | 4;
const DEFAULT_REGISTRY_ID: Uuid = Uuid::from_u128(0x100);
const AUTHORIZED_CTE: &str = r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid
    FROM actors actor
    WHERE actor.id = $1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid = $1
), global_access AS (
    SELECT EXISTS (
        SELECT 1 FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid = scope.actorid
        JOIN permissions permission ON permission.roleid = assignment.roleid
        WHERE permission.resourcetype = $2
          AND (permission.permissionlevel & $3) <> 0
    ) AS allowed
)
"#;

const TAG_SUMMARIES: &str = r#"
COALESCE((
    SELECT jsonb_agg(jsonb_build_object('id', tag.id, 'name', tag.name, 'color', tag.color)
                     ORDER BY tag.name, tag.id)
    FROM resourcetags link
    JOIN tags tag ON tag.id = link.tagid
    WHERE link.resourcetype = $1 AND link.resourceid = resource.id
), '[]'::jsonb) AS tags
"#;

const LATEST_GIT_ACTIVITY: &str = r#"
(
    SELECT jsonb_build_object(
        'id', event.id,
        'resourceType', event.resourcetype,
        'eventType', event.eventtype,
        'status', event.status,
        'info', event.info::jsonb,
        'createdAt', event.createdat)
    FROM activityevents event
    WHERE event.resourceid = resource.id
      AND event.resourcetype = 'GitRepository'
      AND event.eventtype NOT IN ('GitRepoUpdated', 'GitRepoRenamed')
    ORDER BY event.createdat DESC, event.id DESC
    LIMIT 1
) AS latest_activity
"#;

#[derive(Clone)]
pub struct PostgresResourceMetadataStore {
    pool: PgPool,
}

impl PostgresResourceMetadataStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ResourceMetadataStore for PostgresResourceMetadataStore {
    fn list_tags<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<TagView>, ResourceMetadataError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT tag.*,
       (SELECT COUNT(*)::integer FROM resourcetags link WHERE link.tagid = tag.id) AS usage_count
FROM tags tag
WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid = scope.actorid
    WHERE access.resourcetype = $2 AND access.resourceid = tag.id
      AND (access.permissionlevel & $3) <> 0
))
ORDER BY tag.name, tag.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(ResourceType::Tag as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_tag)
                .collect()
        })
    }

    fn create_tag<'a>(
        &'a self,
        actor_id: ActorId,
        tag: &'a NewTag,
    ) -> BoxFuture<'a, Result<TagView, ResourceMetadataError>> {
        Box::pin(async move {
            let now = Utc::now();
            let row = sqlx::query(
                r#"
INSERT INTO tags (id, name, normalizedname, color, createdbyactorid, createdat, updatedat)
VALUES ($1, $2, lower($2), $3, $4, $5, $5)
RETURNING *, 0::integer AS usage_count
"#,
            )
            .bind(Uuid::now_v7())
            .bind(&tag.name)
            .bind(&tag.color)
            .bind(actor_id.value())
            .bind(now)
            .fetch_one(&self.pool)
            .await
            .map_err(database_error)?;
            map_tag(row)
        })
    }

    fn update_tag<'a>(
        &'a self,
        id: Uuid,
        patch: &'a TagPatch,
    ) -> BoxFuture<'a, Result<TagView, ResourceMetadataError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let current = sqlx::query("SELECT name,color FROM tags WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(storage)?
                .ok_or(ResourceMetadataError::NotFound)?;
            let mut tag = NewTag {
                name: patch
                    .name
                    .clone()
                    .unwrap_or(current.try_get("name").map_err(storage)?),
                color: patch
                    .color
                    .clone()
                    .unwrap_or(current.try_get("color").map_err(storage)?),
            };
            tag.validate()?;
            let row = sqlx::query(
                r#"
UPDATE tags
SET name = $2, normalizedname = lower($2), color = $3, updatedat = $4
WHERE id = $1
RETURNING *, (SELECT COUNT(*)::integer FROM resourcetags link WHERE link.tagid = tags.id) AS usage_count
"#,
            )
            .bind(id)
            .bind(&tag.name)
            .bind(&tag.color)
            .bind(Utc::now())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(database_error)?
            .ok_or(ResourceMetadataError::NotFound)?;
            let tag = map_tag(row)?;
            transaction.commit().await.map_err(storage)?;
            Ok(tag)
        })
    }

    fn delete_tag<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), ResourceMetadataError>> {
        Box::pin(async move {
            let affected = sqlx::query("DELETE FROM tags WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(database_error)?
                .rows_affected();
            exactly_one(affected)
        })
    }

    fn get_resource_tags<'a>(
        &'a self,
        resource_type: TaggableResourceType,
        resource_id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<TagSummary>, ResourceMetadataError>> {
        Box::pin(async move {
            ensure_resource_exists(&self.pool, resource_type, resource_id).await?;
            sqlx::query(
                r#"
SELECT tag.id, tag.name, tag.color
FROM resourcetags link
JOIN tags tag ON tag.id = link.tagid
WHERE link.resourcetype = $1 AND link.resourceid = $2
ORDER BY tag.name, tag.id
"#,
            )
            .bind(resource_type.as_database_str())
            .bind(resource_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(map_tag_summary)
            .collect()
        })
    }

    fn replace_resource_tags<'a>(
        &'a self,
        actor_id: ActorId,
        resource_type: TaggableResourceType,
        resource_id: Uuid,
        tag_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<TagSummary>, ResourceMetadataError>> {
        Box::pin(async move {
            let tag_ids = unique_ids(tag_ids);
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            ensure_resource_exists_tx(&mut transaction, resource_type, resource_id).await?;
            let count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tags WHERE id = ANY($1::uuid[])",
            )
            .bind(&tag_ids)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            if count != tag_ids.len() as i64 {
                return Err(ResourceMetadataError::Validation(
                    "One or more Tags do not exist.".to_owned(),
                ));
            }
            sqlx::query("DELETE FROM resourcetags WHERE resourcetype = $1 AND resourceid = $2")
                .bind(resource_type.as_database_str())
                .bind(resource_id)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            if !tag_ids.is_empty() {
                sqlx::query(
                    r#"
INSERT INTO resourcetags (resourcetype, resourceid, tagid, createdbyactorid, createdat)
SELECT $1, $2, tagid, $4, $5 FROM unnest($3::uuid[]) AS tagid
"#,
                )
                .bind(resource_type.as_database_str())
                .bind(resource_id)
                .bind(&tag_ids)
                .bind(actor_id.value())
                .bind(Utc::now())
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
            }
            let tags = load_resource_tags(&mut transaction, resource_type, resource_id).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(tags)
        })
    }

    fn list_registries<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<RegistryView>, ResourceMetadataError>> {
        Box::pin(async move {
            let query = authorized_catalog_query("registries", "registry", ResourceType::Registry);
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(ResourceType::Registry as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .bind(TaggableResourceType::Registry.as_database_str())
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_registry)
                .collect()
        })
    }

    fn get_registry<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<RegistryView, ResourceMetadataError>> {
        Box::pin(async move { get_registry(&self.pool, id).await })
    }

    fn create_registry<'a>(
        &'a self,
        actor_id: ActorId,
        registry: &'a NewRegistry,
    ) -> BoxFuture<'a, Result<RegistryView, ResourceMetadataError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let now = Utc::now();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            validate_tag_ids(&mut transaction, &registry.tag_ids).await?;
            sqlx::query(
                r#"
INSERT INTO registries (id, name, description, registryhost, status, createdat, createdbyactorid, configuration)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
"#,
            )
            .bind(id)
            .bind(&registry.name)
            .bind(registry.description.as_deref())
            .bind(&registry.registry_host)
            .bind(registry.status.as_database_str())
            .bind(now)
            .bind(actor_id.value())
            .bind(&registry.configuration)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;
            insert_resource_tags(
                &mut transaction,
                actor_id,
                TaggableResourceType::Registry,
                id,
                &registry.tag_ids,
            )
            .await?;
            let created = get_registry_tx(&mut transaction, id, false).await?;
            insert_catalog_activity(
                &mut transaction,
                actor_id,
                id,
                &registry.name,
                ActivityEventInfo::registry_created(registry_snapshot(&created)?),
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            get_registry(&self.pool, id).await
        })
    }

    fn update_registry<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        patch: &'a RegistryPatch,
        kind: CatalogMutationKind,
    ) -> BoxFuture<'a, Result<RegistryView, ResourceMetadataError>> {
        Box::pin(async move {
            if id == DEFAULT_REGISTRY_ID {
                return Err(ResourceMetadataError::Conflict(
                    "The default Registry cannot be modified.".to_owned(),
                ));
            }
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let old = get_registry_tx(&mut transaction, id, true).await?;
            let mut updated = NewRegistry {
                name: patch.name.clone().unwrap_or_else(|| old.name.clone()),
                registry_host: patch
                    .registry_host
                    .clone()
                    .unwrap_or_else(|| old.registry_host.clone()),
                status: patch.status.unwrap_or(old.status),
                configuration: merge_json_patch(&patch.configuration, Some(&old.configuration))
                    .unwrap_or(Value::Null),
                description: patch.description.merge_optional(old.description.as_ref()),
                tag_ids: patch
                    .tag_ids
                    .clone()
                    .unwrap_or_else(|| old.tags.iter().map(|tag| tag.id).collect()),
            };
            match kind {
                CatalogMutationKind::Update => updated.validate()?,
                CatalogMutationKind::Metadata => {
                    validate_description(updated.description.as_deref())?;
                }
                CatalogMutationKind::Rename => {
                    validate_name_identifier(&updated.name, "Registry")?;
                    updated.name = updated.name.trim().to_owned();
                }
            }
            if patch.tag_ids.is_some() {
                validate_tag_ids(&mut transaction, &updated.tag_ids).await?;
            }
            let affected = sqlx::query(
                "UPDATE registries SET name=$2, description=$3, registryhost=$4, status=$5, configuration=$6 WHERE id=$1",
            )
            .bind(id)
            .bind(&updated.name)
            .bind(updated.description.as_deref())
            .bind(&updated.registry_host)
            .bind(updated.status.as_database_str())
            .bind(&updated.configuration)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?
            .rows_affected();
            exactly_one(affected)?;
            if patch.tag_ids.is_some() {
                replace_resource_tags_tx(
                    &mut transaction,
                    actor_id,
                    TaggableResourceType::Registry,
                    id,
                    &updated.tag_ids,
                )
                .await?;
            }
            if kind != CatalogMutationKind::Metadata {
                let updated_view = get_registry_tx(&mut transaction, id, false).await?;
                let info = if kind == CatalogMutationKind::Rename {
                    ActivityEventInfo::registry_renamed(old.name.clone(), updated.name.clone())
                } else {
                    ActivityEventInfo::registry_updated(
                        registry_snapshot(&old)?,
                        registry_snapshot(&updated_view)?,
                    )
                };
                insert_catalog_activity(&mut transaction, actor_id, id, &updated.name, info)
                    .await?;
            }
            transaction.commit().await.map_err(storage)?;
            get_registry(&self.pool, id).await
        })
    }

    fn delete_registries<'a>(
        &'a self,
        actor_id: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), ResourceMetadataError>> {
        Box::pin(async move {
            let ids = unique_ids(ids);
            if ids.is_empty() {
                return Err(ResourceMetadataError::Validation(
                    "Ids must not be empty.".to_owned(),
                ));
            }
            if ids.contains(&DEFAULT_REGISTRY_ID) {
                return Err(ResourceMetadataError::Conflict(
                    "The default Registry cannot be deleted.".to_owned(),
                ));
            }
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let rows = load_registries_tx(&mut transaction, &ids).await?;
            if rows.len() != ids.len() {
                return Err(ResourceMetadataError::NotFound);
            }
            sqlx::query("DELETE FROM registries WHERE id = ANY($1::uuid[])")
                .bind(&ids)
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
            for registry in rows {
                insert_catalog_activity(
                    &mut transaction,
                    actor_id,
                    registry.id,
                    &registry.name,
                    ActivityEventInfo::registry_deleted(registry_snapshot(&registry)?),
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn list_git_repositories<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<GitRepositoryView>, ResourceMetadataError>> {
        Box::pin(async move {
            let query = authorized_catalog_query(
                "gitrepositories",
                "repository",
                ResourceType::GitRepository,
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(ResourceType::GitRepository as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .bind(TaggableResourceType::GitRepository.as_database_str())
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_git_repository)
                .collect()
        })
    }

    fn get_git_repository<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<GitRepositoryView, ResourceMetadataError>> {
        Box::pin(async move { get_git_repository(&self.pool, id).await })
    }

    fn create_git_repository<'a>(
        &'a self,
        actor_id: ActorId,
        repository: &'a NewGitRepository,
    ) -> BoxFuture<'a, Result<GitRepositoryView, ResourceMetadataError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let now = Utc::now();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            validate_tag_ids(&mut transaction, &repository.tag_ids).await?;
            validate_git_account(&mut transaction, repository.git_account_id).await?;
            sqlx::query(
                r#"
INSERT INTO gitrepositories (
    id, name, description, url, defaultbranch, gitaccountid, status,
    createdat, createdbyactorid, rowversion, controlstate, syncmode, syncintervalminutes,
    webhook, onclone, onpull)
VALUES ($1,$2,$3,$4,$5,$6,'Created',$7,$8,0,'Idle',$9,$10,$11,$12,$13)
"#,
            )
            .bind(id)
            .bind(&repository.name)
            .bind(repository.description.as_deref())
            .bind(&repository.url)
            .bind(&repository.default_branch)
            .bind(repository.git_account_id)
            .bind(now)
            .bind(actor_id.value())
            .bind(repository.sync_mode.as_database_str())
            .bind(repository.sync_interval_minutes)
            .bind(repository.webhook.as_ref())
            .bind(serialize_optional(&repository.on_clone)?)
            .bind(serialize_optional(&repository.on_pull)?)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;
            insert_resource_tags(
                &mut transaction,
                actor_id,
                TaggableResourceType::GitRepository,
                id,
                &repository.tag_ids,
            )
            .await?;
            let created = get_git_repository_tx(&mut transaction, id, false).await?;
            insert_catalog_activity(
                &mut transaction,
                actor_id,
                id,
                &repository.name,
                ActivityEventInfo::git_repo_created(git_snapshot(&created)?),
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            get_git_repository(&self.pool, id).await
        })
    }

    fn update_git_repository<'a>(
        &'a self,
        actor_id: ActorId,
        id: Uuid,
        patch: &'a GitRepositoryPatch,
        kind: CatalogMutationKind,
    ) -> BoxFuture<'a, Result<GitRepositoryView, ResourceMetadataError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let old = get_git_repository_tx(&mut transaction, id, true).await?;
            let mut updated = NewGitRepository {
                name: patch.name.clone().unwrap_or_else(|| old.name.clone()),
                description: patch.description.merge_optional(old.description.as_ref()),
                url: patch.url.clone().unwrap_or_else(|| old.url.clone()),
                default_branch: patch
                    .default_branch
                    .clone()
                    .unwrap_or_else(|| old.default_branch.clone()),
                git_account_id: patch
                    .git_account_id
                    .merge_optional(old.git_account_id.as_ref()),
                sync_mode: patch.sync_mode.unwrap_or(old.sync_mode),
                sync_interval_minutes: patch
                    .sync_interval_minutes
                    .merge_optional(old.sync_interval_minutes.as_ref()),
                webhook: merge_json_patch(&patch.webhook, old.webhook.as_ref()),
                on_clone: patch.on_clone.merge_optional(old.on_clone.as_ref()),
                on_pull: patch.on_pull.merge_optional(old.on_pull.as_ref()),
                tag_ids: patch
                    .tag_ids
                    .clone()
                    .unwrap_or_else(|| old.tags.iter().map(|tag| tag.id).collect()),
            };
            match kind {
                CatalogMutationKind::Update => updated.validate()?,
                CatalogMutationKind::Metadata => {
                    validate_description(updated.description.as_deref())?;
                }
                CatalogMutationKind::Rename => {
                    validate_name_identifier(&updated.name, "Git repository")?;
                    updated.name = updated.name.trim().to_owned();
                }
            }
            validate_git_account(&mut transaction, updated.git_account_id).await?;
            if patch.tag_ids.is_some() {
                validate_tag_ids(&mut transaction, &updated.tag_ids).await?;
            }
            let affected = sqlx::query(
                "UPDATE gitrepositories SET name=$2, description=$3, url=$4, defaultbranch=$5, gitaccountid=$6, syncmode=$7, syncintervalminutes=$8, webhook=$9, onclone=$10, onpull=$11, rowversion=rowversion+1 WHERE id=$1",
            )
            .bind(id).bind(&updated.name).bind(updated.description.as_deref()).bind(&updated.url).bind(&updated.default_branch).bind(updated.git_account_id)
            .bind(updated.sync_mode.as_database_str()).bind(updated.sync_interval_minutes)
            .bind(updated.webhook.as_ref()).bind(serialize_optional(&updated.on_clone)?).bind(serialize_optional(&updated.on_pull)?)
            .execute(&mut *transaction).await.map_err(database_error)?.rows_affected();
            exactly_one(affected)?;
            if patch.tag_ids.is_some() {
                replace_resource_tags_tx(
                    &mut transaction,
                    actor_id,
                    TaggableResourceType::GitRepository,
                    id,
                    &updated.tag_ids,
                )
                .await?;
            }
            if kind != CatalogMutationKind::Metadata {
                let updated_view = get_git_repository_tx(&mut transaction, id, false).await?;
                let info = if kind == CatalogMutationKind::Rename {
                    ActivityEventInfo::git_repo_renamed(old.name.clone(), updated.name.clone())
                } else {
                    ActivityEventInfo::git_repo_updated(
                        git_snapshot(&old)?,
                        git_snapshot(&updated_view)?,
                    )
                };
                insert_catalog_activity(&mut transaction, actor_id, id, &updated.name, info)
                    .await?;
            }
            transaction.commit().await.map_err(storage)?;
            get_git_repository(&self.pool, id).await
        })
    }

    fn delete_git_repositories<'a>(
        &'a self,
        actor_id: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), ResourceMetadataError>> {
        Box::pin(async move {
            let ids = unique_ids(ids);
            if ids.is_empty() {
                return Err(ResourceMetadataError::Validation(
                    "Ids must not be empty.".into(),
                ));
            }
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let rows = load_git_repositories_tx(&mut transaction, &ids).await?;
            if rows.len() != ids.len() {
                return Err(ResourceMetadataError::NotFound);
            }
            sqlx::query("DELETE FROM gitrepositories WHERE id = ANY($1::uuid[])")
                .bind(&ids)
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
            for repository in rows {
                insert_catalog_activity(
                    &mut transaction,
                    actor_id,
                    repository.id,
                    &repository.name,
                    ActivityEventInfo::git_repo_deleted(git_snapshot(&repository)?),
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn update_platform_description<'a>(
        &'a self,
        platform_id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), ResourceMetadataError>> {
        Box::pin(async move {
            let affected = sqlx::query("UPDATE platforms SET description=$2 WHERE id=$1")
                .bind(platform_id)
                .bind(description)
                .execute(&self.pool)
                .await
                .map_err(storage)?
                .rows_affected();
            exactly_one(affected)
        })
    }

    fn get_bindings<'a>(
        &'a self,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindingsView, ResourceMetadataError>> {
        Box::pin(async move {
            ensure_binding_scope_exists(&self.pool, scope, resource_id).await?;
            load_bindings(&self.pool, scope, resource_id).await
        })
    }

    fn create_binding<'a>(
        &'a self,
        binding: &'a NewResourceBinding,
    ) -> BoxFuture<'a, Result<ResourceBindingsView, ResourceMetadataError>> {
        Box::pin(async move {
            let scope = binding.scope.ok_or_else(|| {
                ResourceMetadataError::Validation("Binding scope is required.".into())
            })?;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            ensure_binding_scope_exists_tx(&mut transaction, scope, binding.resource_id).await?;
            validate_binding_references(&mut transaction, binding.kind, binding.secret_id).await?;
            sqlx::query(r#"INSERT INTO resourcebindings
                (id,name,kind,scope,resourceid,value,secretid,secretdeliverymode,targetpath,createdat,updatedat)
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$10)"#)
                .bind(Uuid::now_v7()).bind(binding.name.trim()).bind(binding_kind(binding.kind))
                .bind(scope.as_database_str()).bind(binding.resource_id).bind(binding.value.as_deref())
                .bind(binding.secret_id).bind(binding.secret_delivery_mode.map(delivery_mode))
                .bind(binding.target_path.as_deref()).bind(Utc::now())
                .execute(&mut *transaction).await.map_err(database_error)?;
            transaction.commit().await.map_err(storage)?;
            load_bindings(&self.pool, scope, binding.resource_id).await
        })
    }

    fn update_binding<'a>(
        &'a self,
        binding: &'a ResourceBindingInput,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindingsView, ResourceMetadataError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            ensure_binding_scope_exists_tx(&mut transaction, scope, resource_id).await?;
            validate_binding_references(&mut transaction, binding.kind, binding.secret_id).await?;
            let affected = sqlx::query(
                r#"UPDATE resourcebindings SET name=$4,kind=$5,value=$6,
                secretid=$7,secretdeliverymode=$8,targetpath=$9,updatedat=$10
                WHERE id=$1 AND scope=$2 AND resourceid IS NOT DISTINCT FROM $3"#,
            )
            .bind(binding.id)
            .bind(scope.as_database_str())
            .bind(resource_id)
            .bind(binding.name.trim())
            .bind(binding_kind(binding.kind))
            .bind(binding.value.as_deref())
            .bind(binding.secret_id)
            .bind(binding.secret_delivery_mode.map(delivery_mode))
            .bind(binding.target_path.as_deref())
            .bind(Utc::now())
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?
            .rows_affected();
            exactly_one(affected)?;
            transaction.commit().await.map_err(storage)?;
            load_bindings(&self.pool, scope, resource_id).await
        })
    }

    fn delete_binding<'a>(
        &'a self,
        id: Uuid,
        scope: ResourceBindingScope,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<ResourceBindingsView, ResourceMetadataError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            ensure_binding_scope_exists_tx(&mut transaction, scope, resource_id).await?;
            let secret_id = sqlx::query_scalar::<_, Option<Uuid>>(r#"DELETE FROM resourcebindings
                WHERE id=$1 AND scope=$2 AND resourceid IS NOT DISTINCT FROM $3 RETURNING secretid"#)
                .bind(id).bind(scope.as_database_str()).bind(resource_id)
                .fetch_optional(&mut *transaction).await.map_err(storage)?
                .ok_or(ResourceMetadataError::NotFound)?;
            if let Some(secret_id) = secret_id {
                sqlx::query(r#"DELETE FROM secretdefinitions secret WHERE secret.id=$1
                    AND NOT EXISTS (SELECT 1 FROM resourcebindings binding WHERE binding.secretid=secret.id)"#)
                    .bind(secret_id).execute(&mut *transaction).await.map_err(storage)?;
            }
            transaction.commit().await.map_err(storage)?;
            load_bindings(&self.pool, scope, resource_id).await
        })
    }

    fn list_secret_definitions<'a>(
        &'a self,
        scope: Option<ResourceBindingScope>,
        resource_id: Option<Uuid>,
    ) -> BoxFuture<'a, Result<Vec<SecretDefinitionView>, ResourceMetadataError>> {
        Box::pin(async move {
            if scope.is_some() != resource_id.is_some() {
                return Err(ResourceMetadataError::Validation(
                    "scope and resourceId must be provided together.".into(),
                ));
            }
            let rows = if let (Some(scope), Some(resource_id)) = (scope, resource_id) {
                sqlx::query(
                    r#"SELECT DISTINCT secret.* FROM secretdefinitions secret
                    JOIN resourcebindings binding ON binding.secretid=secret.id
                    WHERE (binding.scope='Global' AND binding.resourceid IS NULL)
                       OR (binding.scope=$1 AND binding.resourceid=$2)
                    ORDER BY secret.name, secret.id"#,
                )
                .bind(scope.as_database_str())
                .bind(resource_id)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
            } else {
                sqlx::query(
                    r#"SELECT DISTINCT secret.* FROM secretdefinitions secret
                    JOIN resourcebindings binding ON binding.secretid=secret.id
                    WHERE binding.scope='Global' AND binding.resourceid IS NULL
                    ORDER BY secret.name, secret.id"#,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
            };
            rows.into_iter().map(map_secret_definition).collect()
        })
    }

    fn create_internal_secret<'a>(
        &'a self,
        name: &'a str,
        protected_value: &'a str,
    ) -> BoxFuture<'a, Result<SecretDefinitionView, ResourceMetadataError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let now = Utc::now();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            sqlx::query("INSERT INTO secretdefinitions (id,name,providertype,createdat,updatedat) VALUES ($1,$2,'InternalEncrypted',$3,$3)")
                .bind(id).bind(name).bind(now).execute(&mut *transaction).await.map_err(database_error)?;
            sqlx::query("INSERT INTO internalsecretvalues (secretid,encryptedvalue,createdat,updatedat) VALUES ($1,$2,$3,$3)")
                .bind(id).bind(protected_value).bind(now).execute(&mut *transaction).await.map_err(storage)?;
            transaction.commit().await.map_err(storage)?;
            get_secret_definition(&self.pool, id).await
        })
    }

    fn create_external_secret<'a>(
        &'a self,
        input: &'a ExternalSecretInput,
    ) -> BoxFuture<'a, Result<SecretDefinitionView, ResourceMetadataError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let now = Utc::now();
            let affected=sqlx::query(r#"INSERT INTO secretdefinitions
                (id,name,providertype,providerid,externalpath,externalkey,externalversion,createdat,updatedat)
                SELECT $1,$2,'VaultCompatibleKvV2',$3,$4,$5,$6,$7,$7
                WHERE EXISTS (SELECT 1 FROM secretproviders WHERE id=$3)"#)
                .bind(id).bind(input.name.trim()).bind(input.provider_id).bind(input.external_path.trim())
                .bind(input.external_key.trim()).bind(input.external_version).bind(now)
                .execute(&self.pool).await.map_err(database_error)?.rows_affected();
            if affected == 0 {
                return Err(ResourceMetadataError::Validation(
                    "Secret provider does not exist.".into(),
                ));
            }
            get_secret_definition(&self.pool, id).await
        })
    }

    fn update_external_secret<'a>(
        &'a self,
        id: Uuid,
        input: &'a ExternalSecretPatch,
    ) -> BoxFuture<'a, Result<SecretDefinitionView, ResourceMetadataError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let current = sqlx::query("SELECT * FROM secretdefinitions WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(storage)?
                .ok_or(ResourceMetadataError::NotFound)
                .and_then(map_secret_definition)?;
            if current.provider_type != SecretProviderType::VaultCompatibleKvV2 {
                return Err(ResourceMetadataError::Validation(
                    "Only external Vault-compatible Secrets can be updated with this operation."
                        .to_owned(),
                ));
            }
            let merged = input.merge(&current)?;
            let provider_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM secretproviders WHERE id=$1 AND providertype='VaultCompatibleKvV2')",
            )
            .bind(merged.provider_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            if !provider_exists {
                return Err(ResourceMetadataError::NotFound);
            }
            let affected = sqlx::query(
                r#"UPDATE secretdefinitions SET
                name=$2, providerid=$3, externalpath=$4, externalkey=$5,
                externalversion=$6, updatedat=$7
                WHERE id=$1 AND providertype='VaultCompatibleKvV2'"#,
            )
            .bind(id)
            .bind(merged.name.trim())
            .bind(merged.provider_id)
            .bind(merged.external_path.trim_matches('/'))
            .bind(merged.external_key.trim())
            .bind(merged.external_version)
            .bind(Utc::now())
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?
            .rows_affected();
            exactly_one(affected)?;
            transaction.commit().await.map_err(storage)?;
            get_secret_definition(&self.pool, id).await
        })
    }

    fn delete_secret_definition<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<(), ResourceMetadataError>> {
        Box::pin(async move {
            exactly_one(
                sqlx::query("DELETE FROM secretdefinitions WHERE id=$1")
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(database_error)?
                    .rows_affected(),
            )
        })
    }

    fn list_secret_providers<'a>(
        &'a self,
    ) -> BoxFuture<'a, Result<Vec<SecretProviderView>, ResourceMetadataError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM secretproviders ORDER BY name,id")
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_secret_provider)
                .collect()
        })
    }

    fn get_secret_provider<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<SecretProviderView, ResourceMetadataError>> {
        Box::pin(async move { get_secret_provider(&self.pool, id).await })
    }

    fn create_secret_provider<'a>(
        &'a self,
        input: &'a StoredSecretProviderInput,
    ) -> BoxFuture<'a, Result<SecretProviderView, ResourceMetadataError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let now = Utc::now();
            let config = json!({
                "Address":input.address,"MountPath":input.mount_path,"ProtectedToken":input.protected_token});
            sqlx::query("INSERT INTO secretproviders (id,name,providertype,configuration,createdat,updatedat) VALUES ($1,$2,'VaultCompatibleKvV2',$3,$4,$4)")
                .bind(id).bind(&input.name).bind(config.to_string()).bind(now).execute(&self.pool).await.map_err(database_error)?;
            get_secret_provider(&self.pool, id).await
        })
    }

    fn update_secret_provider<'a>(
        &'a self,
        id: Uuid,
        input: &'a StoredSecretProviderPatch,
    ) -> BoxFuture<'a, Result<SecretProviderView, ResourceMetadataError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let current = get_secret_provider_row_tx(&mut transaction, id).await?;
            let mut config = current.1;
            let object = config.as_object_mut().ok_or_else(|| {
                ResourceMetadataError::Storage("Secret provider configuration is invalid.".into())
            })?;
            if let Some(value) = input.address.as_ref() {
                object.insert("Address".into(), Value::String(value.clone()));
            }
            if let Some(value) = input.mount_path.as_ref() {
                object.insert("MountPath".into(), Value::String(value.clone()));
            }
            if let Some(value) = input.protected_token.as_ref() {
                object.insert("ProtectedToken".into(), Value::String(value.clone()));
            }
            let affected=sqlx::query("UPDATE secretproviders SET name=COALESCE($2,name),configuration=$3,updatedat=$4 WHERE id=$1")
                .bind(id).bind(input.name.as_deref()).bind(config.to_string()).bind(Utc::now()).execute(&mut *transaction).await.map_err(database_error)?.rows_affected();
            exactly_one(affected)?;
            transaction.commit().await.map_err(storage)?;
            get_secret_provider(&self.pool, id).await
        })
    }

    fn delete_secret_provider<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<(), ResourceMetadataError>> {
        Box::pin(async move {
            exactly_one(
                sqlx::query("DELETE FROM secretproviders WHERE id=$1")
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    .map_err(database_error)?
                    .rows_affected(),
            )
        })
    }
}

fn map_tag(row: PgRow) -> Result<TagView, ResourceMetadataError> {
    Ok(TagView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        normalized_name: row.try_get("normalizedname").map_err(storage)?,
        color: row.try_get("color").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        updated_at: row.try_get("updatedat").map_err(storage)?,
        usage_count: row.try_get("usage_count").map_err(storage)?,
    })
}

fn map_tag_summary(row: PgRow) -> Result<TagSummary, ResourceMetadataError> {
    Ok(TagSummary {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        color: row.try_get("color").map_err(storage)?,
    })
}

fn map_registry(row: PgRow) -> Result<RegistryView, ResourceMetadataError> {
    let configuration: Value = row.try_get("configuration").map_err(storage)?;
    Ok(RegistryView {
        id: row.try_get("id").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        status: parse_registry_status(&row.try_get::<String, _>("status").map_err(storage)?)?,
        description: row.try_get("description").map_err(storage)?,
        registry_host: row.try_get("registryhost").map_err(storage)?,
        registry_type: registry_type(&configuration)?,
        configuration,
        created_at: row.try_get("createdat").map_err(storage)?,
        tags: serde_json::from_value(row.try_get("tags").map_err(storage)?).map_err(storage)?,
    })
}

fn map_git_repository(row: PgRow) -> Result<GitRepositoryView, ResourceMetadataError> {
    Ok(GitRepositoryView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        url: row.try_get("url").map_err(storage)?,
        default_branch: row.try_get("defaultbranch").map_err(storage)?,
        git_account_id: row.try_get("gitaccountid").map_err(storage)?,
        sync_mode: parse_git_sync_mode(&row.try_get::<String, _>("syncmode").map_err(storage)?)?,
        sync_interval_minutes: row.try_get("syncintervalminutes").map_err(storage)?,
        webhook: row.try_get("webhook").map_err(storage)?,
        on_clone: deserialize_optional(row.try_get("onclone").map_err(storage)?)?,
        on_pull: deserialize_optional(row.try_get("onpull").map_err(storage)?)?,
        status: row.try_get("status").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        control_state: row
            .try_get::<Option<String>, _>("controlstate")
            .map_err(storage)?
            .unwrap_or_else(|| "Idle".to_owned()),
        latest_activity_view: row.try_get("latest_activity").map_err(storage)?,
        tags: serde_json::from_value(row.try_get("tags").map_err(storage)?).map_err(storage)?,
    })
}

fn serialize_optional<T: Serialize>(
    value: &Option<T>,
) -> Result<Option<String>, ResourceMetadataError> {
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(storage)
}

fn deserialize_optional<T: serde::de::DeserializeOwned>(
    value: Option<String>,
) -> Result<Option<T>, ResourceMetadataError> {
    value
        .map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(storage)
}

fn map_binding(row: PgRow, inherited: bool) -> Result<ResourceBindingView, ResourceMetadataError> {
    let kind = parse_binding_kind(&row.try_get::<String, _>("kind").map_err(storage)?)?;
    Ok(ResourceBindingView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        kind,
        scope: parse_scope(&row.try_get::<String, _>("scope").map_err(storage)?)?,
        resource_id: row.try_get("resourceid").map_err(storage)?,
        value: if kind == ResourceBindingKind::Variable {
            row.try_get("value").map_err(storage)?
        } else {
            None
        },
        secret_id: row.try_get("secretid").map_err(storage)?,
        secret_delivery_mode: row
            .try_get::<Option<String>, _>("secretdeliverymode")
            .map_err(storage)?
            .map(|v| parse_delivery(&v))
            .transpose()?,
        target_path: row.try_get("targetpath").map_err(storage)?,
        is_inherited: inherited,
    })
}

fn map_secret_definition(row: PgRow) -> Result<SecretDefinitionView, ResourceMetadataError> {
    Ok(SecretDefinitionView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        provider_type: parse_provider_type(
            &row.try_get::<String, _>("providertype").map_err(storage)?,
        )?,
        provider_id: row.try_get("providerid").map_err(storage)?,
        external_path: row.try_get("externalpath").map_err(storage)?,
        external_key: row.try_get("externalkey").map_err(storage)?,
        external_version: row.try_get("externalversion").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}

fn map_secret_provider(row: PgRow) -> Result<SecretProviderView, ResourceMetadataError> {
    let config: Value =
        serde_json::from_str(&row.try_get::<String, _>("configuration").map_err(storage)?)
            .map_err(storage)?;
    Ok(SecretProviderView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        provider_type: SecretProviderType::VaultCompatibleKvV2,
        address: config
            .get("Address")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        mount_path: config
            .get("MountPath")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}

async fn get_registry(pool: &PgPool, id: Uuid) -> Result<RegistryView, ResourceMetadataError> {
    let q =
        format!("SELECT resource.*, {TAG_SUMMARIES} FROM registries resource WHERE resource.id=$2");
    sqlx::query(AssertSqlSafe(q.as_str()))
        .bind(TaggableResourceType::Registry.as_database_str())
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(ResourceMetadataError::NotFound)
        .and_then(map_registry)
}

async fn get_registry_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    lock: bool,
) -> Result<RegistryView, ResourceMetadataError> {
    let lock_clause = if lock { " FOR UPDATE OF resource" } else { "" };
    let query = format!(
        "SELECT resource.*, {TAG_SUMMARIES} FROM registries resource WHERE resource.id=$2{lock_clause}"
    );
    sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(TaggableResourceType::Registry.as_database_str())
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(ResourceMetadataError::NotFound)
        .and_then(map_registry)
}

async fn get_git_repository(
    pool: &PgPool,
    id: Uuid,
) -> Result<GitRepositoryView, ResourceMetadataError> {
    let q = format!(
        "SELECT resource.*, {TAG_SUMMARIES}, {LATEST_GIT_ACTIVITY} FROM gitrepositories resource WHERE resource.id=$2"
    );
    sqlx::query(AssertSqlSafe(q.as_str()))
        .bind(TaggableResourceType::GitRepository.as_database_str())
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(ResourceMetadataError::NotFound)
        .and_then(map_git_repository)
}

async fn get_git_repository_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    lock: bool,
) -> Result<GitRepositoryView, ResourceMetadataError> {
    let lock_clause = if lock { " FOR UPDATE OF resource" } else { "" };
    let query = format!(
        "SELECT resource.*, {TAG_SUMMARIES}, {LATEST_GIT_ACTIVITY} FROM gitrepositories resource WHERE resource.id=$2{lock_clause}"
    );
    sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(TaggableResourceType::GitRepository.as_database_str())
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(ResourceMetadataError::NotFound)
        .and_then(map_git_repository)
}

fn authorized_catalog_query(table: &str, alias: &str, resource_type: ResourceType) -> String {
    let extra_columns = if resource_type == ResourceType::GitRepository {
        ", NULL::jsonb AS latest_activity"
    } else {
        ""
    };
    format!(
        r#"{AUTHORIZED_CTE}
SELECT resource.*, COALESCE((SELECT jsonb_agg(jsonb_build_object('id',tag.id,'name',tag.name,'color',tag.color) ORDER BY tag.name,tag.id)
 FROM resourcetags link JOIN tags tag ON tag.id=link.tagid WHERE link.resourcetype=$5 AND link.resourceid=resource.id),'[]'::jsonb) AS tags{extra_columns}
FROM {table} resource WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
 SELECT 1 FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
 WHERE access.resourcetype=$2 AND access.resourceid=resource.id AND (access.permissionlevel & $3)<>0))
ORDER BY resource.name,resource.id -- {alias}"#
    )
}

async fn load_bindings(
    pool: &PgPool,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
) -> Result<ResourceBindingsView, ResourceMetadataError> {
    let entries=sqlx::query("SELECT * FROM resourcebindings WHERE scope=$1 AND resourceid IS NOT DISTINCT FROM $2 ORDER BY name,id")
        .bind(scope.as_database_str()).bind(resource_id).fetch_all(pool).await.map_err(storage)?.into_iter().map(|r|map_binding(r,false)).collect::<Result<Vec<_>,_>>()?;
    let mut effective = if scope == ResourceBindingScope::Global {
        Vec::new()
    } else {
        sqlx::query("SELECT * FROM resourcebindings WHERE scope='Global' AND resourceid IS NULL ORDER BY name,id")
        .fetch_all(pool).await.map_err(storage)?.into_iter().map(|r|map_binding(r,true)).collect::<Result<Vec<_>,_>>()?
    };
    for entry in &entries {
        effective.retain(|global| global.name != entry.name);
    }
    effective.extend(entries.iter().cloned());
    effective.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    Ok(ResourceBindingsView {
        entries,
        effective_entries: effective,
    })
}

async fn ensure_binding_scope_exists(
    pool: &PgPool,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
) -> Result<(), ResourceMetadataError> {
    let mut transaction = pool.begin().await.map_err(storage)?;
    ensure_binding_scope_exists_tx(&mut transaction, scope, resource_id).await?;
    transaction.rollback().await.map_err(storage)
}

async fn ensure_binding_scope_exists_tx(
    transaction: &mut Transaction<'_, Postgres>,
    scope: ResourceBindingScope,
    resource_id: Option<Uuid>,
) -> Result<(), ResourceMetadataError> {
    let Some(resource_id) = resource_id else {
        return if scope == ResourceBindingScope::Global {
            Ok(())
        } else {
            Err(ResourceMetadataError::Validation(
                "Resource-scoped bindings require a resource ID.".to_owned(),
            ))
        };
    };
    let table = match scope {
        ResourceBindingScope::Global => {
            return Err(ResourceMetadataError::Validation(
                "Global bindings cannot target a resource.".to_owned(),
            ));
        }
        ResourceBindingScope::Stack => "stacks",
        ResourceBindingScope::Deployment => "deployments",
        ResourceBindingScope::SwarmService => "swarmservices",
    };
    let query = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=$1)");
    let exists = sqlx::query_scalar::<_, bool>(AssertSqlSafe(query.as_str()))
        .bind(resource_id)
        .fetch_one(&mut **transaction)
        .await
        .map_err(storage)?;
    if exists {
        Ok(())
    } else {
        Err(ResourceMetadataError::NotFound)
    }
}

async fn get_secret_definition(
    pool: &PgPool,
    id: Uuid,
) -> Result<SecretDefinitionView, ResourceMetadataError> {
    sqlx::query("SELECT * FROM secretdefinitions WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(ResourceMetadataError::NotFound)
        .and_then(map_secret_definition)
}
async fn get_secret_provider(
    pool: &PgPool,
    id: Uuid,
) -> Result<SecretProviderView, ResourceMetadataError> {
    sqlx::query("SELECT * FROM secretproviders WHERE id=$1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .ok_or(ResourceMetadataError::NotFound)
        .and_then(map_secret_provider)
}
async fn get_secret_provider_row_tx(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<(String, Value), ResourceMetadataError> {
    let row = sqlx::query("SELECT name,configuration FROM secretproviders WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?
        .ok_or(ResourceMetadataError::NotFound)?;
    let name = row.try_get("name").map_err(storage)?;
    let config = serde_json::from_str(&row.try_get::<String, _>("configuration").map_err(storage)?)
        .map_err(storage)?;
    Ok((name, config))
}

async fn ensure_resource_exists(
    pool: &PgPool,
    t: TaggableResourceType,
    id: Uuid,
) -> Result<(), ResourceMetadataError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    ensure_resource_exists_tx(&mut tx, t, id).await?;
    tx.rollback().await.map_err(storage)
}
async fn ensure_resource_exists_tx(
    tx: &mut Transaction<'_, Postgres>,
    t: TaggableResourceType,
    id: Uuid,
) -> Result<(), ResourceMetadataError> {
    let table = match t {
        TaggableResourceType::Deployment => "deployments",
        TaggableResourceType::Stack => "stacks",
        TaggableResourceType::Platform => "platforms",
        TaggableResourceType::GitRepository => "gitrepositories",
        TaggableResourceType::Registry => "registries",
        TaggableResourceType::AutomationAction => "actions",
        TaggableResourceType::BackupPolicy => "backuppolicies",
        TaggableResourceType::Build => "buildprojects",
        TaggableResourceType::BuildAgentPool => "buildagentpools",
        TaggableResourceType::SwarmService => "swarmservices",
    };
    let q = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=$1)");
    if !sqlx::query_scalar::<_, bool>(AssertSqlSafe(q.as_str()))
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?
    {
        return Err(ResourceMetadataError::NotFound);
    }
    Ok(())
}
async fn load_resource_tags(
    tx: &mut Transaction<'_, Postgres>,
    t: TaggableResourceType,
    id: Uuid,
) -> Result<Vec<TagSummary>, ResourceMetadataError> {
    sqlx::query("SELECT tag.id,tag.name,tag.color FROM resourcetags link JOIN tags tag ON tag.id=link.tagid WHERE link.resourcetype=$1 AND link.resourceid=$2 ORDER BY tag.name,tag.id")
    .bind(t.as_database_str()).bind(id).fetch_all(&mut **tx).await.map_err(storage)?.into_iter().map(map_tag_summary).collect()
}
async fn validate_tag_ids(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<(), ResourceMetadataError> {
    let ids = unique_ids(ids);
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tags WHERE id=ANY($1::uuid[])")
        .bind(&ids)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    if count != ids.len() as i64 {
        return Err(ResourceMetadataError::Validation(
            "One or more Tags do not exist.".into(),
        ));
    }
    Ok(())
}

async fn validate_git_account(
    transaction: &mut Transaction<'_, Postgres>,
    id: Option<Uuid>,
) -> Result<(), ResourceMetadataError> {
    let Some(id) = id else {
        return Ok(());
    };
    let exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM gitaccounts WHERE id=$1)")
            .bind(id)
            .fetch_one(&mut **transaction)
            .await
            .map_err(storage)?;
    if exists {
        Ok(())
    } else {
        Err(ResourceMetadataError::NotFound)
    }
}
async fn insert_resource_tags(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    t: TaggableResourceType,
    id: Uuid,
    ids: &[Uuid],
) -> Result<(), ResourceMetadataError> {
    let ids = unique_ids(ids);
    if ids.is_empty() {
        return Ok(());
    }
    sqlx::query("INSERT INTO resourcetags (resourcetype,resourceid,tagid,createdbyactorid,createdat) SELECT $1,$2,tagid,$4,$5 FROM unnest($3::uuid[]) tagid")
    .bind(t.as_database_str()).bind(id).bind(ids).bind(actor.value()).bind(Utc::now()).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}

async fn replace_resource_tags_tx(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    resource_type: TaggableResourceType,
    resource_id: Uuid,
    ids: &[Uuid],
) -> Result<(), ResourceMetadataError> {
    sqlx::query("DELETE FROM resourcetags WHERE resourcetype=$1 AND resourceid=$2")
        .bind(resource_type.as_database_str())
        .bind(resource_id)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    insert_resource_tags(tx, actor, resource_type, resource_id, ids).await
}
async fn validate_binding_references(
    tx: &mut Transaction<'_, Postgres>,
    kind: ResourceBindingKind,
    secret: Option<Uuid>,
) -> Result<(), ResourceMetadataError> {
    if kind == ResourceBindingKind::Secret {
        let Some(id) = secret else {
            return Err(ResourceMetadataError::Validation(
                "Secret binding requires a Secret.".into(),
            ));
        };
        if !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM secretdefinitions WHERE id=$1)",
        )
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?
        {
            return Err(ResourceMetadataError::Validation(
                "Secret does not exist.".into(),
            ));
        }
    }
    Ok(())
}

async fn load_registries_tx(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<Vec<RegistryView>, ResourceMetadataError> {
    let q = format!(
        "SELECT resource.*, {TAG_SUMMARIES} FROM registries resource WHERE resource.id=ANY($2::uuid[]) ORDER BY resource.id"
    );
    sqlx::query(AssertSqlSafe(q.as_str()))
        .bind(TaggableResourceType::Registry.as_database_str())
        .bind(ids)
        .fetch_all(&mut **tx)
        .await
        .map_err(storage)?
        .into_iter()
        .map(map_registry)
        .collect()
}
async fn load_git_repositories_tx(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<Vec<GitRepositoryView>, ResourceMetadataError> {
    let q = format!(
        "SELECT resource.*, {TAG_SUMMARIES}, NULL::jsonb AS latest_activity FROM gitrepositories resource WHERE resource.id=ANY($2::uuid[]) ORDER BY resource.id"
    );
    sqlx::query(AssertSqlSafe(q.as_str()))
        .bind(TaggableResourceType::GitRepository.as_database_str())
        .bind(ids)
        .fetch_all(&mut **tx)
        .await
        .map_err(storage)?
        .into_iter()
        .map(map_git_repository)
        .collect()
}
async fn insert_catalog_activity(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    id: Uuid,
    name: &str,
    info: ActivityEventInfo,
) -> Result<(), ResourceMetadataError> {
    let event = match info.event_type().resource_type() {
        citadel_domain::ActivityResourceType::Registry => {
            ActivityEvent::new_registry_event(id, name.to_owned(), actor, info, Utc::now())
        }
        citadel_domain::ActivityResourceType::GitRepository => {
            ActivityEvent::new_git_repository_event(id, name.to_owned(), actor, info, Utc::now())
        }
        _ => unreachable!("catalog activities have a closed resource type"),
    }
    .map_err(|error| ResourceMetadataError::Storage(error.to_string()))?;
    insert_typed_activity(tx, &event)
        .await
        .map_err(|error| ResourceMetadataError::Storage(error.to_string()))
}

fn registry_snapshot(
    value: &RegistryView,
) -> Result<RegistryActivitySnapshot, ResourceMetadataError> {
    Ok(RegistryActivitySnapshot {
        id: value.id,
        name: value.name.clone(),
        description: value.description.clone().unwrap_or_default(),
        registry_host: value.registry_host.clone(),
        status: value.status.as_database_str().to_owned(),
        configuration: masked_registry_configuration(&value.configuration)?,
    })
}

fn git_snapshot(
    value: &GitRepositoryView,
) -> Result<GitRepositoryActivitySnapshot, ResourceMetadataError> {
    Ok(GitRepositoryActivitySnapshot {
        id: value.id,
        name: value.name.clone(),
        description: value.description.clone(),
        url: value.url.clone(),
        default_branch: value.default_branch.clone(),
        git_account_id: value.git_account_id,
        sync_mode: value.sync_mode.as_database_str().to_owned(),
        sync_interval_minutes: value.sync_interval_minutes,
        webhook: value.webhook.as_ref().map(mask_webhook),
        on_clone: serialize_activity_value(value.on_clone.as_ref())?,
        on_pull: serialize_activity_value(value.on_pull.as_ref())?,
        resolved_commit_sha: None,
    })
}

fn masked_registry_configuration(value: &Value) -> Result<Value, ResourceMetadataError> {
    let mut masked = value.clone();
    let kind = registry_type(&masked)?;
    let keys: &[&str] = match kind.as_str() {
        "Gitlab" | "GitHub" | "DockerHub" => &["PAT"],
        "Custom" | "Azure" => &["Password"],
        "AWS" => &["AccessKey", "SecretAccessKey"],
        _ => &[],
    };
    if let Some(object) = masked.as_object_mut() {
        for key in keys {
            mask_property(object, key);
        }
    }
    Ok(masked)
}

fn mask_webhook(value: &Value) -> Value {
    let mut masked = value.clone();
    if let Some(object) = masked.as_object_mut() {
        mask_property(object, "Secret");
    }
    masked
}

fn mask_property(object: &mut serde_json::Map<String, Value>, pascal_name: &str) {
    let camel_name = format!(
        "{}{}",
        pascal_name[..1].to_ascii_lowercase(),
        &pascal_name[1..]
    );
    for key in [pascal_name, camel_name.as_str()] {
        if let Some(value) = object.get_mut(key) {
            *value = if value.as_str().is_some_and(|value| !value.is_empty()) {
                Value::String("****************".to_owned())
            } else {
                Value::Null
            };
        }
    }
}

fn serialize_activity_value<T: Serialize>(
    value: Option<&T>,
) -> Result<Option<Value>, ResourceMetadataError> {
    value.map(serde_json::to_value).transpose().map_err(storage)
}

fn merge_json_patch(
    patch: &citadel_resources::MetadataPatch<Value>,
    current: Option<&Value>,
) -> Option<Value> {
    match patch {
        citadel_resources::MetadataPatch::Missing => current.cloned(),
        citadel_resources::MetadataPatch::Null => None,
        citadel_resources::MetadataPatch::Value(patch) => {
            let mut value = current.cloned().unwrap_or(Value::Null);
            apply_json_merge_patch(&mut value, patch);
            Some(value)
        }
    }
}

fn apply_json_merge_patch(target: &mut Value, patch: &Value) {
    let Value::Object(patch) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = Value::Object(serde_json::Map::new());
    }
    let target = target
        .as_object_mut()
        .expect("target was initialized as an object");
    for (key, value) in patch {
        if value.is_null() {
            target.remove(key);
        } else {
            apply_json_merge_patch(target.entry(key.clone()).or_insert(Value::Null), value);
        }
    }
}
fn unique_ids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    ids
}
fn exactly_one(value: u64) -> Result<(), ResourceMetadataError> {
    match value {
        1 => Ok(()),
        0 => Err(ResourceMetadataError::NotFound),
        _ => Err(ResourceMetadataError::Storage(
            "Mutation affected an unexpected number of rows.".into(),
        )),
    }
}
fn database_error(error: sqlx::Error) -> ResourceMetadataError {
    if let sqlx::Error::Database(db) = &error {
        if db.code().as_deref() == Some("23505") {
            return ResourceMetadataError::Conflict(
                "A resource with the same unique value already exists.".into(),
            );
        }
        if db.code().as_deref() == Some("23503") {
            return ResourceMetadataError::Conflict("The resource is still in use.".into());
        }
    }
    storage(error)
}
fn storage(error: impl std::fmt::Display) -> ResourceMetadataError {
    ResourceMetadataError::Storage(error.to_string())
}
fn parse_registry_status(v: &str) -> Result<RegistryStatus, ResourceMetadataError> {
    match v {
        "Active" => Ok(RegistryStatus::Active),
        "Disabled" => Ok(RegistryStatus::Disabled),
        "Deprecated" => Ok(RegistryStatus::Deprecated),
        _ => Err(ResourceMetadataError::Storage(format!(
            "Unknown Registry status '{v}'."
        ))),
    }
}
fn parse_git_sync_mode(v: &str) -> Result<GitRepositorySyncMode, ResourceMetadataError> {
    match v {
        "Manual" => Ok(GitRepositorySyncMode::Manual),
        "PullInterval" => Ok(GitRepositorySyncMode::PullInterval),
        _ => Err(ResourceMetadataError::Storage(format!(
            "Unknown Git repository sync mode '{v}'."
        ))),
    }
}
fn binding_kind(v: ResourceBindingKind) -> &'static str {
    match v {
        ResourceBindingKind::Variable => "Variable",
        ResourceBindingKind::Secret => "Secret",
    }
}
fn parse_binding_kind(v: &str) -> Result<ResourceBindingKind, ResourceMetadataError> {
    match v {
        "Variable" => Ok(ResourceBindingKind::Variable),
        "Secret" => Ok(ResourceBindingKind::Secret),
        _ => Err(ResourceMetadataError::Storage(format!(
            "Unknown binding kind '{v}'."
        ))),
    }
}
fn parse_scope(v: &str) -> Result<ResourceBindingScope, ResourceMetadataError> {
    match v {
        "Global" => Ok(ResourceBindingScope::Global),
        "Stack" => Ok(ResourceBindingScope::Stack),
        "Deployment" => Ok(ResourceBindingScope::Deployment),
        "SwarmService" => Ok(ResourceBindingScope::SwarmService),
        _ => Err(ResourceMetadataError::Storage(format!(
            "Unknown binding scope '{v}'."
        ))),
    }
}
fn delivery_mode(v: SecretDeliveryMode) -> &'static str {
    match v {
        SecretDeliveryMode::EnvironmentVariable => "EnvironmentVariable",
        SecretDeliveryMode::MountedFile => "MountedFile",
        SecretDeliveryMode::NativePlatformSecret => "NativePlatformSecret",
    }
}
fn parse_delivery(v: &str) -> Result<SecretDeliveryMode, ResourceMetadataError> {
    match v {
        "EnvironmentVariable" => Ok(SecretDeliveryMode::EnvironmentVariable),
        "MountedFile" => Ok(SecretDeliveryMode::MountedFile),
        "NativePlatformSecret" => Ok(SecretDeliveryMode::NativePlatformSecret),
        _ => Err(ResourceMetadataError::Storage(format!(
            "Unknown Secret delivery mode '{v}'."
        ))),
    }
}
fn parse_provider_type(v: &str) -> Result<SecretProviderType, ResourceMetadataError> {
    match v {
        "InternalEncrypted" => Ok(SecretProviderType::InternalEncrypted),
        "VaultCompatibleKvV2" => Ok(SecretProviderType::VaultCompatibleKvV2),
        _ => Err(ResourceMetadataError::Storage(format!(
            "Unknown Secret provider type '{v}'."
        ))),
    }
}

#[cfg(test)]
mod tests {
    use citadel_resources::MetadataPatch;
    use serde_json::json;

    use super::merge_json_patch;

    #[test]
    fn nested_catalog_configuration_uses_json_merge_patch_semantics() {
        let current = json!({
            "$type":"Custom",
            "authEnabled":true,
            "userName":"operator",
            "password":"old-secret"
        });
        let patch = MetadataPatch::Value(json!({"password":"new-secret"}));
        assert_eq!(
            merge_json_patch(&patch, Some(&current)).unwrap(),
            json!({
                "$type":"Custom",
                "authEnabled":true,
                "userName":"operator",
                "password":"new-secret"
            })
        );

        let patch = MetadataPatch::Value(json!({"password":null}));
        assert!(
            merge_json_patch(&patch, Some(&current))
                .unwrap()
                .get("password")
                .is_none()
        );
    }
}

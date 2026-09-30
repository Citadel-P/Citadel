mod queries;

mod rows;

use rows::*;

mod persistence;

mod validation;

use persistence::*;

mod tag_links;

use crate::persistence::postgres::tags::TAG_SUMMARIES;

use tag_links::*;

use validation::*;

use chrono::Utc;

use citadel_activities::{ActivityEventInfo, GitRepositoryActivitySnapshot};

use citadel_primitives::{ActorId, ResourceType};

use citadel_git::{
    CreateGitRepository, GitRepository, GitRepositoryError, GitRepositoryMutationKind,
    GitRepositoryPatch, GitRepositoryPersistence, GitRepositorySyncMode,
};

use citadel_git::repositories::{validate_description, validate_name_identifier};

use citadel_tags::TaggableResourceType;

use futures_util::future::BoxFuture;

use serde_json::Value;

use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction, postgres::PgRow};

use uuid::Uuid;

impl GitRepositoryPersistence for PostgresGitRepositoryPersistence {
    fn list_git_repositories<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<GitRepository>, GitRepositoryError>> {
        Box::pin(async move {
            let query = queries::authorized_list();
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(ResourceType::GitRepository as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
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
    ) -> BoxFuture<'a, Result<GitRepository, GitRepositoryError>> {
        Box::pin(async move { get_git_repository(&self.pool, id).await })
    }
    fn create_git_repository<'a>(
        &'a self,
        actor_id: ActorId,
        repository: &'a CreateGitRepository,
    ) -> BoxFuture<'a, Result<GitRepository, GitRepositoryError>> {
        Box::pin(async move {
            let id = Uuid::now_v7();
            let now = Utc::now();
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            validate_tag_ids(&mut transaction, &repository.tag_ids).await?;
            validate_git_account(&mut transaction, repository.git_account_id, &repository.url)
                .await?;
            sqlx::query(
                r#"
INSERT INTO gitrepositories (
    id, name, description, url, defaultbranch, gitaccountid, status,
    createdat, createdbyactorid, rowversion, controlstate, controltriggeredby,
    syncmode, syncintervalminutes, webhook, onclone, onpull)
VALUES ($1,$2,$3,$4,$5,$6,'Pending',$7,$8,0,'Queued',$8,$9,$10,$11,$12,$13)
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
            sqlx::query(
                "INSERT INTO gitrepositoryrefs(id,gitrepositoryid,branch,resolvedcommitsha,status,lasterror,lastsyncedat) VALUES($1,$2,$3,NULL,'Pending',NULL,$4)",
            )
            .bind(Uuid::now_v7())
            .bind(id)
            .bind(&repository.default_branch)
            .bind(now)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            insert_resource_tags(
                &mut transaction,
                actor_id,
                TaggableResourceType::GitRepository,
                id,
                &repository.tag_ids,
            )
            .await?;
            let created = get_git_repository_tx(&mut transaction, id, false).await?;
            insert_git_activity(
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
        kind: GitRepositoryMutationKind,
    ) -> BoxFuture<'a, Result<GitRepository, GitRepositoryError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let old = get_git_repository_tx(&mut transaction, id, true).await?;
            let mut updated = CreateGitRepository {
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
                GitRepositoryMutationKind::Update => updated.validate()?,
                GitRepositoryMutationKind::Metadata => {
                    validate_description(updated.description.as_deref())?;
                }
                GitRepositoryMutationKind::Rename => {
                    validate_name_identifier(&updated.name, "Git repository")?;
                    updated.name = updated.name.trim().to_owned();
                }
            }
            validate_git_account(&mut transaction, updated.git_account_id, &updated.url).await?;
            if patch.tag_ids.is_some() {
                validate_tag_ids(&mut transaction, &updated.tag_ids).await?;
            }
            let source_changed = old.url != updated.url
                || old.default_branch != updated.default_branch
                || old.git_account_id != updated.git_account_id;
            let affected = sqlx::query(
                "UPDATE gitrepositories SET name=$2, description=$3, url=$4, defaultbranch=$5, gitaccountid=$6, syncmode=$7, syncintervalminutes=$8, webhook=$9, onclone=$10, onpull=$11, status=CASE WHEN $12 THEN 'Pending' ELSE status END, controlstate=CASE WHEN $12 AND controlstate<>'Processing' THEN 'Queued' ELSE controlstate END, controltriggeredby=CASE WHEN $12 THEN $13 ELSE controltriggeredby END, controlstartedat=CASE WHEN $12 AND controlstate<>'Processing' THEN NULL ELSE controlstartedat END, rowversion=rowversion+1 WHERE id=$1",
            )
            .bind(id).bind(&updated.name).bind(updated.description.as_deref()).bind(&updated.url).bind(&updated.default_branch).bind(updated.git_account_id)
            .bind(updated.sync_mode.as_database_str()).bind(updated.sync_interval_minutes)
            .bind(updated.webhook.as_ref()).bind(serialize_optional(&updated.on_clone)?).bind(serialize_optional(&updated.on_pull)?).bind(source_changed).bind(actor_id.value())
            .execute(&mut *transaction).await.map_err(database_error)?.rows_affected();
            exactly_one(affected)?;
            if source_changed {
                sqlx::query(
                    r#"INSERT INTO gitrepositoryrefs(id,gitrepositoryid,branch,resolvedcommitsha,status,lasterror,lastsyncedat)
VALUES($1,$2,$3,NULL,'Pending',NULL,CURRENT_TIMESTAMP)
ON CONFLICT(gitrepositoryid,branch) DO UPDATE SET status='Pending',lasterror=NULL,synctrigger='Manual'"#,
                )
                .bind(Uuid::now_v7())
                .bind(id)
                .bind(&updated.default_branch)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            }
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
            if kind != GitRepositoryMutationKind::Metadata {
                let updated_view = get_git_repository_tx(&mut transaction, id, false).await?;
                let info = if kind == GitRepositoryMutationKind::Rename {
                    ActivityEventInfo::git_repo_renamed(old.name.clone(), updated.name.clone())
                } else {
                    ActivityEventInfo::git_repo_updated(
                        git_snapshot(&old)?,
                        git_snapshot(&updated_view)?,
                    )
                };
                insert_git_activity(&mut transaction, actor_id, id, &updated.name, info).await?;
            }
            transaction.commit().await.map_err(storage)?;
            get_git_repository(&self.pool, id).await
        })
    }
    fn delete_git_repositories<'a>(
        &'a self,
        actor_id: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), GitRepositoryError>> {
        Box::pin(async move {
            let ids = unique_ids(ids);
            if ids.is_empty() {
                return Err(GitRepositoryError::Validation(
                    "Ids must not be empty.".into(),
                ));
            }
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let rows = load_git_repositories_tx(&mut transaction, &ids).await?;
            if rows.len() != ids.len() {
                return Err(GitRepositoryError::NotFound);
            }
            sqlx::query("DELETE FROM gitrepositories WHERE id = ANY($1::uuid[])")
                .bind(&ids)
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
            for repository in rows {
                insert_git_activity(
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
}

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
pub struct PostgresGitRepositoryPersistence {
    pool: PgPool,
}

impl PostgresGitRepositoryPersistence {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

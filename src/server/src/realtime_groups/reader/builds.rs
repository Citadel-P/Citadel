use super::*;
use crate::api::resources::builds::{authorized::authorized_projects, views::BuildRunView};
use crate::realtime::AuthorizationLease;
use citadel_builds::BuildError;

impl ApplicationGroupReader {
    pub(super) async fn build_project_patch(
        &self,
        principal: &ActorPrincipal,
        project: Uuid,
        lease: Option<&AuthorizationLease>,
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        match self
            .permission_with_lease(principal, ResourceType::Build, Some(project), None, lease)
            .await
        {
            Ok(()) => {}
            Err(RealtimeReadError::Authorization) => {
                return rows(
                    "BuildProjectInfoUpdated",
                    Vec::<Value>::new(),
                    RowStyle::OwnerPatch(project),
                );
            }
            Err(error) => return Err(error),
        }
        let owner = project;
        let project = match self.builds.get(project).await {
            Ok(project) => project,
            Err(BuildError::NotFound) => {
                return rows(
                    "BuildProjectInfoUpdated",
                    Vec::<Value>::new(),
                    RowStyle::OwnerPatch(project),
                );
            }
            Err(error) => return Err(failure(error)),
        };
        rows(
            "BuildProjectInfoUpdated",
            authorized_projects(self.builds.as_ref(), principal, vec![project])
                .await
                .map_err(failure)?,
            RowStyle::OwnerPatch(owner),
        )
    }

    pub(super) async fn build_run_snapshot(
        &self,
        principal: &ActorPrincipal,
        group: &Group,
        run: Uuid,
        change: Option<&PublishedRuntimeEvent>,
        lease: Option<&AuthorizationLease>,
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        let changed_project = change.and_then(PublishedRuntimeEvent::changed_build_project);
        let (owner, present) = match self.builds.run_project_id(run).await {
            Ok(owner) => (owner, true),
            // Retention may have removed a run after the client joined it.
            Err(BuildError::NotFound) if change.is_some() => {
                let previous_owner = lease
                    .and_then(|lease| lease.scope(&group.name))
                    .ok_or(RealtimeReadError::Authorization)?;
                (previous_owner, false)
            }
            Err(error) => return Err(failure(error)),
        };
        if changed_project.is_some_and(|project| project != owner) {
            return Ok(GroupSnapshot::default());
        }
        self.permission_with_lease(principal, ResourceType::Build, Some(owner), None, lease)
            .await?;
        if !present {
            return rows(
                "BuildRunInfoUpdated",
                Vec::<BuildRunView>::new(),
                RowStyle::Update,
            );
        }
        if change.is_none()
            && let Some(lease) = lease
        {
            lease.set_scope(&group.name, owner);
        }
        if let Some(change) =
            change.filter(|event| event.event_kind == "buildLogs" && event.resource_id == run)
        {
            return Ok(GroupSnapshot {
                rows: vec![],
                events: vec![ClientEvent::new(
                    "BuildRunLogsAppended",
                    vec![json!(run), change.payload["entries"].clone()],
                )],
            });
        }
        let values = match self.builds.get_run(run).await {
            Ok(value) => vec![BuildRunView::try_from(value).map_err(failure)?],
            Err(BuildError::NotFound) if changed_project.is_some() => vec![],
            Err(error) => return Err(failure(error)),
        };
        rows("BuildRunInfoUpdated", values, RowStyle::Update)
    }
}

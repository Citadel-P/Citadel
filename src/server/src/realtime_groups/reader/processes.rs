use super::*;
use crate::api::resources::platforms::descriptor_views::ContainerProcessesView;
use tokio_util::sync::CancellationToken;

impl ApplicationGroupReader {
    pub(super) async fn processes(
        &self,
        principal: &ActorPrincipal,
        id: Uuid,
        change: Option<&PublishedRuntimeEvent>,
        lease: Option<&crate::realtime::AuthorizationLease>,
    ) -> Result<GroupSnapshot, RealtimeReadError> {
        let container = self
            .platforms
            .get_container(id)
            .await
            .map_err(failure)?
            .ok_or(RealtimeReadError::Authorization)?;
        self.permission_with_lease(
            principal,
            ResourceType::Platform,
            Some(container.platform_id),
            None,
            lease,
        )
        .await?;
        self.permission_with_lease(
            principal,
            ResourceType::Platform,
            Some(container.platform_id),
            Some(SpecificPermission::Inspect),
            lease,
        )
        .await?;
        if let Some(change) = change {
            if change.platform_id.unwrap_or(change.resource_id) != container.platform_id
                || change
                    .container_ids()
                    .is_some_and(|ids| !ids.iter().any(|id| id == &container.container_id))
            {
                return Ok(GroupSnapshot::default());
            }
            if change.payload["dockerResourceType"] == "containerStats"
                && (change.payload["dockerNodeId"].as_str() != container.docker_node_id.as_deref()
                    || !change.payload["stats"].as_array().is_some_and(|stats| {
                        stats
                            .iter()
                            .any(|stat| stat["dockerContainerId"] == container.container_id)
                    }))
            {
                return Ok(GroupSnapshot::default());
            }
        } else {
            // Initial data and reconnect recovery use HTTP, independent of WS.
            return event("ContainerProcessesRefresh", json!({"resourceId": id}));
        }
        let read = || async {
            let cancel = CancellationToken::new();
            let _guard = cancel.clone().drop_guard();
            let data = self
                .platforms
                .container_processes(self.runtime.as_ref(), &container, &cancel)
                .await
                .ok()
                .map(ContainerProcessesView::from);
            json!({"resourceId": id, "data": data})
        };
        // Share one bounded runtime read per container per statistics/state event,
        // including failed reads. Each viewer is authorized above before access.
        let change = change.expect("event checked above");
        let cell = change
            .reads
            .processes
            .lock()
            .await
            .entry(id)
            .or_default()
            .clone();
        let value = cell.get_or_init(read).await.clone();
        event("ContainerProcessesUpdated", value)
    }
}

use crate::ContainerStatSnapshot;
use crate::PlatformStatSnapshot;
use crate::RuntimeCapabilityError;
use futures_util::future::BoxFuture;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatsWindow(u16);
impl StatsWindow {
    pub fn new(hours: u16) -> Option<Self> {
        matches!(hours, 24 | 48 | 72).then_some(Self(hours))
    }
    pub fn since(self, now: i64) -> i64 {
        now.saturating_sub(i64::from(self.0) * 3600)
    }
    pub fn bucket_seconds(self) -> i64 {
        if self.0 == 24 { 60 } else { 300 }
    }
}

pub struct ServiceStatIdentity<'a> {
    pub platform_id: Uuid,
    pub docker_service_id: &'a str,
    pub managed_service_id: Option<Uuid>,
    pub stack_id: Option<Uuid>,
    pub service_name: &'a str,
}

pub struct StatisticsContainer {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub docker_id: String,
    pub name: String,
}

#[derive(Clone, Copy)]
pub enum StatisticsWorkload {
    Deployment,
    Stack,
}

pub struct ServiceTaskSample {
    pub node_id: String,
    pub container_id: Option<Uuid>,
    pub created: Option<i64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatistics {
    pub docker_service_id: String,
    pub observed_tasks: usize,
    pub expected_tasks: usize,
    pub complete: bool,
    pub observed_container_projection_ids: Vec<Uuid>,
    pub missing_docker_node_ids: Vec<String>,
    pub oldest_sample_at: Option<chrono::DateTime<chrono::Utc>>,
    pub newest_sample_at: Option<chrono::DateTime<chrono::Utc>>,
    pub stats: Vec<ContainerStatSnapshot>,
}

impl ServiceStatistics {
    pub fn new(
        service: &crate::SwarmServiceSummary,
        tasks: Vec<ServiceTaskSample>,
        stats: Vec<ContainerStatSnapshot>,
        fresh_after: i64,
    ) -> Self {
        let mut ids = Vec::with_capacity(tasks.len());
        let mut missing = std::collections::BTreeSet::new();
        let mut times = Vec::with_capacity(tasks.len());
        for task in tasks {
            if let Some(id) = task.container_id {
                ids.push(id);
            }
            if task.container_id.is_some()
                && task.created.is_some_and(|t| t >= fresh_after)
                && !service.is_stale
            {
                times.extend(task.created);
            } else {
                missing.insert(task.node_id);
            }
        }
        let expected = service.desired_task_count.max(0) as usize;
        Self {
            docker_service_id: service.id.clone(),
            observed_tasks: times.len(),
            expected_tasks: expected,
            complete: !service.is_stale && times.len() == expected && missing.is_empty(),
            observed_container_projection_ids: ids,
            missing_docker_node_ids: missing.into_iter().collect(),
            oldest_sample_at: times
                .iter()
                .min()
                .and_then(|t| chrono::DateTime::from_timestamp(*t, 0)),
            newest_sample_at: times
                .iter()
                .max()
                .and_then(|t| chrono::DateTime::from_timestamp(*t, 0)),
            stats,
        }
    }
}

pub trait StatisticsReader: Send + Sync {
    fn task_container<'a>(
        &'a self,
        platform_id: Uuid,
        node_id: &'a str,
        container_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<StatisticsContainer>, RuntimeCapabilityError>>;
    fn service_current_tasks<'a>(
        &'a self,
        platform_id: Uuid,
        service_id: &'a str,
        now: i64,
    ) -> BoxFuture<'a, Result<Vec<ServiceTaskSample>, RuntimeCapabilityError>>;
    fn find_container<'a>(
        &'a self,
        reference: &'a str,
    ) -> BoxFuture<'a, Result<Option<StatisticsContainer>, RuntimeCapabilityError>>;
    fn workload_containers(
        &self,
        workload: StatisticsWorkload,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<Vec<StatisticsContainer>>, RuntimeCapabilityError>>;
    fn containers<'a>(
        &'a self,
        ids: &'a [Uuid],
        window: StatsWindow,
        now: i64,
    ) -> BoxFuture<'a, Result<Vec<ContainerStatSnapshot>, RuntimeCapabilityError>>;
    fn platform(
        &self,
        id: Uuid,
        window: StatsWindow,
        now: i64,
    ) -> BoxFuture<'_, Result<Vec<PlatformStatSnapshot>, RuntimeCapabilityError>>;
    fn service<'a>(
        &'a self,
        identity: ServiceStatIdentity<'a>,
        window: StatsWindow,
        now: i64,
    ) -> BoxFuture<'a, Result<Vec<ContainerStatSnapshot>, RuntimeCapabilityError>>;
}

pub trait SwarmTaskRuntimePort: Send + Sync {
    fn inspect_task<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<crate::RuntimeSwarmTask, RuntimeCapabilityError>>;
}

pub fn validate_running_task<'a>(
    projection: &crate::SwarmTaskSummary,
    live: &'a crate::RuntimeSwarmTask,
) -> Result<&'a str, RuntimeCapabilityError> {
    if projection.is_stale
        || projection.id != live.id
        || projection.node_id != live.node_id
        || !projection.state.eq_ignore_ascii_case("running")
        || !live.state.eq_ignore_ascii_case("running")
        || live.container_id.as_deref().is_none_or(str::is_empty)
    {
        return Err(RuntimeCapabilityError::new(
            crate::RuntimeErrorKind::Conflict,
            "Task is no longer running or its Container projection has not synchronized.",
            false,
        ));
    }
    Ok(live.container_id.as_deref().expect("checked above"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_and_bucket_sizes_match_dotnet() {
        for invalid in [0, 1, 23, 25, 49, 73, u16::MAX] {
            assert!(StatsWindow::new(invalid).is_none());
        }
        for (hours, bucket) in [(24, 60), (48, 300), (72, 300)] {
            let window = StatsWindow::new(hours).unwrap();
            assert_eq!(window.bucket_seconds(), bucket);
            assert_eq!(window.since(1_000_000), 1_000_000 - i64::from(hours) * 3600);
        }
    }

    #[test]
    fn task_target_rejects_stopped_stale_missing_container_and_changed_identity() {
        let mut projection = crate::SwarmTaskSummary {
            id: "task".into(),
            version_index: 1,
            name: "service.1".into(),
            service_id: "service".into(),
            service_name: "service".into(),
            slot: Some(1),
            node_id: "node".into(),
            node_hostname: "node".into(),
            desired_state: "running".into(),
            state: "running".into(),
            status_message: None,
            error: None,
            image: "nginx".into(),
            ports: vec![],
            status_timestamp: None,
            created_at: None,
            updated_at: None,
            observed_at: chrono::Utc::now(),
            is_stale: false,
        };
        let live = crate::RuntimeSwarmTask {
            id: "task".into(),
            node_id: "node".into(),
            state: "running".into(),
            container_id: Some("container".into()),
            ..Default::default()
        };
        assert_eq!(
            validate_running_task(&projection, &live).unwrap(),
            "container"
        );
        for changed in [
            crate::RuntimeSwarmTask {
                state: "shutdown".into(),
                ..live.clone()
            },
            crate::RuntimeSwarmTask {
                node_id: "other".into(),
                ..live.clone()
            },
            crate::RuntimeSwarmTask {
                id: "other".into(),
                ..live.clone()
            },
            crate::RuntimeSwarmTask {
                container_id: None,
                ..live.clone()
            },
            crate::RuntimeSwarmTask {
                container_id: Some(String::new()),
                ..live.clone()
            },
        ] {
            assert_eq!(
                validate_running_task(&projection, &changed)
                    .unwrap_err()
                    .kind,
                crate::RuntimeErrorKind::Conflict
            );
        }
        projection.is_stale = true;
        assert!(validate_running_task(&projection, &live).is_err());
        projection.is_stale = false;
        projection.state = "shutdown".into();
        assert!(validate_running_task(&projection, &live).is_err());
    }
}

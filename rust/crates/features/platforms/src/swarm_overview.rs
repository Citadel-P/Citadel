use crate::WorkloadStatusCounts;

#[derive(Debug, Default)]
pub struct SwarmSummary {
    pub is_stale: bool,
    pub node_count: i64,
    pub manager_count: i64,
    pub reachable_managers: i64,
    pub has_leader: bool,
    pub managers_stale: bool,
    pub service_counts: WorkloadStatusCounts,
    pub running_tasks: i64,
    pub desired_tasks: i64,
    pub network_count: i64,
    pub local_network_count: i64,
    pub volume_count: i64,
    pub image_count: i64,
}

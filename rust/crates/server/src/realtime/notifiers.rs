use super::RealtimeHub;
use citadel_deployments::DeploymentChangeNotifier;
use citadel_stacks::StackChangeNotifier;
use citadel_swarm_services::SwarmServiceChangeNotifier;
use uuid::Uuid;

pub struct DeploymentsRealtimeNotifier {
    realtime: Option<RealtimeHub>,
}

impl DeploymentsRealtimeNotifier {
    #[must_use]
    pub const fn new(realtime: Option<RealtimeHub>) -> Self {
        Self { realtime }
    }
}

impl DeploymentChangeNotifier for DeploymentsRealtimeNotifier {
    fn adopted(&self, deployment: &citadel_deployments::Deployment) {
        self.changed(deployment.id, "created");
        if let Some(realtime) = &self.realtime {
            if let Some(container) = &deployment.docker_container_id {
                realtime.publish_runtime_change(
                    deployment.platform_id,
                    "container",
                    "update",
                    container,
                );
            }
            realtime.publish_resource_change("Platform", deployment.platform_id, "updated");
        }
    }
    fn changed(&self, deployment_id: Uuid, event: &'static str) {
        if let Some(realtime) = &self.realtime {
            realtime.publish_resource_change("Deployment", deployment_id, event);
        }
    }
}

pub struct StacksRealtimeNotifier {
    realtime: Option<RealtimeHub>,
}

impl StacksRealtimeNotifier {
    #[must_use]
    pub const fn new(realtime: Option<RealtimeHub>) -> Self {
        Self { realtime }
    }
}

impl StackChangeNotifier for StacksRealtimeNotifier {
    fn update_check_duplicate(&self) {
        citadel_runtime::runtime_metrics::RuntimeWork::StackUpdateDuplicate.units(1);
    }

    fn changed(&self, id: Uuid, event: &'static str) {
        if let Some(realtime) = &self.realtime {
            realtime.publish_resource_change("Stack", id, event);
        }
    }
}

pub struct SwarmServicesRealtimeNotifier {
    realtime: Option<RealtimeHub>,
}

impl SwarmServicesRealtimeNotifier {
    #[must_use]
    pub const fn new(realtime: Option<RealtimeHub>) -> Self {
        Self { realtime }
    }
}

impl SwarmServiceChangeNotifier for SwarmServicesRealtimeNotifier {
    fn changed(&self, id: Uuid, event: &'static str) {
        if let Some(realtime) = &self.realtime {
            realtime.publish_resource_change("SwarmService", id, event);
        }
    }
}

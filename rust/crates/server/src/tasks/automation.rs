use citadel_automation::{AutomationError, AutomationTaskSpawner};
use citadel_runtime::DynamicTasks;
use futures_util::future::BoxFuture;

pub struct TrackedAutomationTasks(DynamicTasks);

impl TrackedAutomationTasks {
    pub fn new(tasks: DynamicTasks) -> Self {
        Self(tasks)
    }
}

impl AutomationTaskSpawner for TrackedAutomationTasks {
    fn spawn(
        &self,
        name: &'static str,
        task: BoxFuture<'static, Result<(), AutomationError>>,
    ) -> bool {
        self.0.spawn(name, task)
    }
}

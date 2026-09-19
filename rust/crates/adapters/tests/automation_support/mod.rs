use citadel_application::DynamicTasks;
use citadel_automation::{AutomationError, AutomationTaskSpawner};
use futures_util::future::BoxFuture;
pub struct Tasks(pub DynamicTasks);
impl AutomationTaskSpawner for Tasks {
    fn spawn(
        &self,
        name: &'static str,
        task: BoxFuture<'static, Result<(), AutomationError>>,
    ) -> bool {
        self.0.spawn(name, task)
    }
}

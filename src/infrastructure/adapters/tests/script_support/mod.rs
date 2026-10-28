use citadel_runtime::DynamicTasks;
use citadel_scripts::{ScriptError, ScriptTaskSpawner};
use futures_util::future::BoxFuture;
pub struct Tasks(pub DynamicTasks);
impl ScriptTaskSpawner for Tasks {
    fn spawn(&self, name: &'static str, task: BoxFuture<'static, Result<(), ScriptError>>) -> bool {
        self.0.spawn(name, task)
    }
}

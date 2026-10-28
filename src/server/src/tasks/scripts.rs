use citadel_runtime::DynamicTasks;
use citadel_scripts::{ScriptError, ScriptTaskSpawner};
use futures_util::future::BoxFuture;

pub struct TrackedScriptTasks(DynamicTasks);

impl TrackedScriptTasks {
    pub fn new(tasks: DynamicTasks) -> Self {
        Self(tasks)
    }
}

impl ScriptTaskSpawner for TrackedScriptTasks {
    fn spawn(&self, name: &'static str, task: BoxFuture<'static, Result<(), ScriptError>>) -> bool {
        self.0.spawn(name, task)
    }
}

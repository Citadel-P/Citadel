use citadel_application::DynamicTasks;
use citadel_builds::{BuildError, BuildTaskSpawner};
use futures_util::future::BoxFuture;

pub struct TrackedBuildTasks(DynamicTasks);

impl TrackedBuildTasks {
    pub fn new(tasks: DynamicTasks) -> Self {
        Self(tasks)
    }
}

impl BuildTaskSpawner for TrackedBuildTasks {
    fn spawn(&self, name: &'static str, task: BoxFuture<'static, Result<(), BuildError>>) -> bool {
        self.0.spawn(name, task)
    }
}

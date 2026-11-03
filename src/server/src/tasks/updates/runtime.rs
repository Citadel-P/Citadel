//! Process observation for the feature-owned update drain coordinator.
use citadel_runtime::{DynamicTasks, admission::AdmissionGate};
use citadel_updates::{UpdateDrainBlocker, UpdateDrainRuntime, UpdateError};
use uuid::Uuid;

pub struct TrackedUpdateDrain {
    admission: AdmissionGate,
    tasks: DynamicTasks,
}

impl TrackedUpdateDrain {
    pub fn new(admission: AdmissionGate, tasks: DynamicTasks) -> Self {
        Self { admission, tasks }
    }
}

impl UpdateDrainRuntime for TrackedUpdateDrain {
    fn close(&self, fence: Uuid) -> Result<(), UpdateError> {
        self.admission
            .close(fence)
            .map_err(|_| UpdateError::Conflict("Maintenance belongs to another update.".into()))
    }

    fn blockers(&self, fence: Uuid) -> Result<Vec<UpdateDrainBlocker>, UpdateError> {
        let mut blockers: Vec<_> = self
            .admission
            .blockers_for(fence)
            .map_err(|_| UpdateError::Conflict("Maintenance ownership changed.".into()))?
            .into_iter()
            .map(|(kind, count)| UpdateDrainBlocker {
                kind: kind.into(),
                count: count as u64,
            })
            .collect();
        let count = self.tasks.active() as u64;
        if count > 0 {
            blockers.push(UpdateDrainBlocker {
                kind: "Detached work and cleanup".into(),
                count,
            });
        }
        // Do not call DynamicTasks::drain: it permanently closes task admission
        // and would prevent admitted operations from scheduling required cleanup.
        Ok(blockers)
    }
}

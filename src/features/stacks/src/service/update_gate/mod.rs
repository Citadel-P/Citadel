//! Only live Stack checks occupy keys. The existing operation semaphore bounds
//! the set to four entries; release on future drop also covers cancellation.
use std::{collections::BTreeSet, sync::Mutex};
use uuid::Uuid;

#[derive(Default)]
pub(super) struct UpdateCheckGate(Mutex<BTreeSet<Uuid>>);

impl UpdateCheckGate {
    pub fn try_enter(&self, id: Uuid) -> Option<UpdateCheckLease<'_>> {
        let mut active = self.0.lock().unwrap();
        active
            .insert(id)
            .then(|| UpdateCheckLease { gate: self, id })
    }
}

pub(super) struct UpdateCheckLease<'a> {
    gate: &'a UpdateCheckGate,
    id: Uuid,
}
impl Drop for UpdateCheckLease<'_> {
    fn drop(&mut self) {
        self.gate.0.lock().unwrap().remove(&self.id);
    }
}

#[cfg(test)]
mod tests;

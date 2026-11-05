use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdmissionError {
    #[error("Admission is owned by another maintenance attempt")]
    Ownership,
    #[error("Maintenance drain was cancelled; admission remains closed")]
    Cancelled,
    #[error("Maintenance drain timed out waiting for: {0:?}")]
    Busy(BTreeMap<&'static str, usize>),
}

#[derive(Default)]
struct State {
    owner: Option<Uuid>,
    active: BTreeMap<&'static str, usize>,
}

struct Inner {
    state: Mutex<State>,
    changed: tokio::sync::Notify,
}

/// Shared by request and dispatch boundaries. Closing admission does not cancel
/// admitted work. Dropping a waiter or timing out never reopens the gate.
#[derive(Clone)]
pub struct AdmissionGate(Arc<Inner>);

impl Default for AdmissionGate {
    fn default() -> Self {
        Self(Arc::new(Inner {
            state: Mutex::new(State::default()),
            changed: tokio::sync::Notify::new(),
        }))
    }
}

/// Keep the permit through execution and finalization, including owned child
/// controllers. No borrowed request lifetime may own a durable operation.
pub struct AdmissionPermit {
    gate: AdmissionGate,
    kind: &'static str,
}

impl Drop for AdmissionPermit {
    fn drop(&mut self) {
        let mut state = self.gate.0.state.lock().expect("admission poisoned");
        let count = state
            .active
            .get_mut(self.kind)
            .expect("registered admission");
        *count -= 1;
        if *count == 0 {
            state.active.remove(self.kind);
        }
        drop(state);
        self.gate.0.changed.notify_waiters();
    }
}

impl AdmissionGate {
    pub fn try_enter(&self, kind: &'static str) -> Option<AdmissionPermit> {
        let mut state = self.0.state.lock().expect("admission poisoned");
        if state.owner.is_some() {
            return None;
        }
        *state.active.entry(kind).or_default() += 1;
        Some(AdmissionPermit {
            gate: self.clone(),
            kind,
        })
    }

    /// Queue workers wait without polling their claim future or losing queued
    /// work. Process shutdown is separate from maintenance and can end this wait.
    pub async fn enter(
        &self,
        kind: &'static str,
        cancel: &CancellationToken,
    ) -> Option<AdmissionPermit> {
        loop {
            let changed = self.0.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if cancel.is_cancelled() {
                return None;
            }
            if let Some(permit) = self.try_enter(kind) {
                return Some(permit);
            }
            tokio::select! { biased; () = cancel.cancelled() => return None, () = changed => {} }
        }
    }

    /// The coordinator must bind this fence to durable update ownership before
    /// claiming any drain as successful. There is deliberately no automatic Drop resume.
    pub fn close(&self, fence: Uuid) -> Result<(), AdmissionError> {
        let mut state = self.0.state.lock().expect("admission poisoned");
        if fence.is_nil() || state.owner.is_some_and(|owner| owner != fence) {
            return Err(AdmissionError::Ownership);
        }
        state.owner = Some(fence);
        drop(state);
        self.0.changed.notify_waiters();
        Ok(())
    }

    /// Only the owning coordinator may resume, after verifying persisted outcome
    /// and unchanged source. This process-local gate is not restart authority.
    pub fn reopen(&self, fence: Uuid) -> Result<(), AdmissionError> {
        let mut state = self.0.state.lock().expect("admission poisoned");
        if state.owner != Some(fence) {
            return Err(AdmissionError::Ownership);
        }
        state.owner = None;
        drop(state);
        self.0.changed.notify_waiters();
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.0
            .state
            .lock()
            .expect("admission poisoned")
            .owner
            .is_some()
    }

    pub fn blockers(&self) -> BTreeMap<&'static str, usize> {
        self.0
            .state
            .lock()
            .expect("admission poisoned")
            .active
            .clone()
    }

    pub fn blockers_for(
        &self,
        fence: Uuid,
    ) -> Result<BTreeMap<&'static str, usize>, AdmissionError> {
        let state = self.0.state.lock().expect("admission poisoned");
        if state.owner != Some(fence) {
            return Err(AdmissionError::Ownership);
        }
        Ok(state.active.clone())
    }

    /// This drains registered admissions only. Detached tasks, durable execution
    /// reservations and remaining writers require separate verification before a checkpoint.
    pub async fn drain(
        &self,
        fence: Uuid,
        timeout: Duration,
        cancel: &CancellationToken,
    ) -> Result<(), AdmissionError> {
        let wait = async {
            loop {
                let changed = self.0.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if cancel.is_cancelled() {
                    return Err(AdmissionError::Cancelled);
                }
                {
                    let state = self.0.state.lock().expect("admission poisoned");
                    if state.owner != Some(fence) {
                        return Err(AdmissionError::Ownership);
                    }
                    if state.active.is_empty() {
                        return Ok(());
                    }
                }
                tokio::select! { biased; () = cancel.cancelled() => return Err(AdmissionError::Cancelled), () = changed => {} }
            }
        };
        tokio::time::timeout(timeout, wait)
            .await
            .unwrap_or_else(|_| Err(AdmissionError::Busy(self.blockers())))
    }
}

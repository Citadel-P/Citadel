//! One Agent-lifetime ledger for complete build + push attempts.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tonic::Status;
use uuid::Uuid;

#[derive(Clone)]
pub(super) struct BuildAdmission(Arc<Mutex<State>>);
struct State {
    authority: Uuid,
    generation: u64,
    limit: usize,
    attempts: HashMap<Uuid, Attempt>,
    heavy: Arc<tokio::sync::Semaphore>,
}
struct Attempt {
    _heavy: tokio::sync::OwnedSemaphorePermit,
    generation: u64,
    run: Uuid,
    pushes: Vec<String>,
    phase: Phase,
}
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Building,
    WaitingPush(usize),
    Pushing(usize),
    Uncertain,
}

#[derive(Debug, PartialEq)]
pub(super) enum AdmissionRefusal {
    CapacityBusy,
    Fenced,
}

pub(super) struct Permit {
    owner: BuildAdmission,
    id: Uuid,
    phase: Phase,
    started: bool,
    finished: bool,
}

fn identity(run: &str, attempt: &str) -> Result<(Uuid, Uuid), Status> {
    let parse = |value: &str| Uuid::parse_str(value).ok().filter(|id| !id.is_nil());
    match (parse(run), parse(attempt)) {
        (Some(run), Some(attempt)) => Ok((run, attempt)),
        _ => Err(Status::invalid_argument(
            "Build run and attempt IDs are required",
        )),
    }
}
impl BuildAdmission {
    #[cfg(test)]
    pub(super) fn new(limit: usize) -> Self {
        Self::with_heavy(limit, Arc::new(tokio::sync::Semaphore::new(limit)))
    }
    pub(super) fn with_heavy(limit: usize, heavy: Arc<tokio::sync::Semaphore>) -> Self {
        Self(Arc::new(Mutex::new(State {
            authority: Uuid::now_v7(),
            generation: 1,
            limit,
            attempts: HashMap::new(),
            heavy,
        })))
    }
    pub(super) fn authority(&self) -> Result<String, Status> {
        Ok(self
            .0
            .lock()
            .map_err(|_| Status::internal("Build admission unavailable"))?
            .authority
            .to_string())
    }
    pub(super) fn capacity(&self) -> Result<(usize, usize, usize), Status> {
        let state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Build admission unavailable"))?;
        Ok((
            state.limit,
            state.attempts.len(),
            state
                .attempts
                .values()
                .filter(|v| v.phase == Phase::Uncertain)
                .count(),
        ))
    }
    pub(super) fn generation(&self) -> Result<u64, Status> {
        Ok(self
            .0
            .lock()
            .map_err(|_| Status::internal("Build admission unavailable"))?
            .generation)
    }

    pub(super) fn reserve_automation(
        &self,
        ledger: &super::automation::Admission,
        request: &citadel_contracts::citadel::automation::v1::AutomationAttempt,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<bool, Status> {
        let state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Admission unavailable"))?;
        if request.authority_id != state.authority.to_string()
            || request.generation != state.generation
        {
            return Ok(false);
        }
        ledger.reserve(request, cancel)
    }

    pub(super) fn reserve_data(
        &self,
        data: &super::data_admission::DataAdmission,
        request: &citadel_contracts::citadel::images::v1::DataOperationRequest,
    ) -> Result<citadel_contracts::citadel::images::v1::BuildAdmissionResult, Status> {
        use citadel_contracts::citadel::images::v1::BuildAdmissionResult;
        let state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Admission unavailable"))?;
        if request.admission_generation != state.generation
            || request.admission_authority_id != state.authority.to_string()
        {
            return Ok(BuildAdmissionResult::BuildAdmissionFencedNotStarted);
        }
        // Hold the generation lock through insertion. Otherwise reconciliation
        // could confirm absence between this check and the data reservation.
        data.reserve(request)
    }

    pub(super) fn fence_generation(&self, generation: u64) -> Result<(), Status> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Admission unavailable"))?;
        if generation == 0 || generation > state.generation {
            return Err(Status::failed_precondition("Invalid admission generation"));
        }
        if generation == state.generation {
            state.generation = state
                .generation
                .checked_add(1)
                .ok_or_else(|| Status::resource_exhausted("Admission generation exhausted"))?;
        }
        Ok(())
    }

    // Fence all requests issued before this cleanup check. Already admitted
    // work remains owned; no tombstone collection grows with completed builds.
    pub(super) fn reconcile(
        &self,
        run: &str,
        id: &str,
        authority: &str,
        generation: u64,
    ) -> Result<citadel_contracts::citadel::images::v1::BuildCleanupState, Status> {
        use citadel_contracts::citadel::images::v1::BuildCleanupState as Cleanup;
        let (run, id) = identity(run, id)?;
        let mut state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Build admission unavailable"))?;
        if authority != state.authority.to_string()
            || generation == 0
            || generation > state.generation
        {
            return Err(Status::failed_precondition(
                "Build admission identity changed",
            ));
        }
        if let Some(attempt) = state.attempts.get(&id) {
            if attempt.run != run || attempt.generation != generation {
                return Err(Status::failed_precondition(
                    "Build attempt identity does not match",
                ));
            }
            match attempt.phase {
                Phase::Building | Phase::Pushing(_) => return Ok(Cleanup::BuildCleanupActive),
                Phase::Uncertain => return Ok(Cleanup::BuildCleanupUncertain),
                Phase::WaitingPush(_) => {}
            }
        } else if state.attempts.values().any(|attempt| attempt.run == run) {
            return Err(Status::failed_precondition("Another attempt owns this run"));
        }
        // A retry of an acknowledged/lost cleanup response is idempotent and
        // does not invalidate newly issued admissions again.
        if state.generation == generation {
            state.generation = state.generation.checked_add(1).ok_or_else(|| {
                Status::resource_exhausted("Build admission generation exhausted")
            })?;
        }
        state.attempts.remove(&id);
        Ok(Cleanup::BuildCleanupConfirmed)
    }

    // Refusals prove non-execution. An existing
    // attempt is a conflict, never a fresh admission or a replayable busy result.
    pub(super) fn build(
        &self,
        run: &str,
        id: &str,
        pushes: Vec<String>,
        authority: &str,
        generation: u64,
    ) -> Result<Result<Permit, AdmissionRefusal>, Status> {
        let (run, id) = identity(run, id)?;
        if pushes.len() > 64 || pushes.iter().any(|v| v.is_empty() || v.len() > 256) {
            return Err(Status::invalid_argument("Invalid build push plan"));
        }
        let mut state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Build admission unavailable"))?;
        if authority != state.authority.to_string() || generation == 0 {
            return Err(Status::failed_precondition(
                "Build admission identity changed",
            ));
        }
        if state.attempts.contains_key(&id) || state.attempts.values().any(|v| v.run == run) {
            return Err(Status::already_exists(
                "Build attempt is active or awaiting reconciliation",
            ));
        }
        if generation != state.generation {
            return Ok(Err(AdmissionRefusal::Fenced));
        }
        if state.attempts.len() >= state.limit {
            return Ok(Err(AdmissionRefusal::CapacityBusy));
        }
        let Ok(heavy) = state.heavy.clone().try_acquire_owned() else {
            return Ok(Err(AdmissionRefusal::CapacityBusy));
        };
        state.attempts.insert(
            id,
            Attempt {
                _heavy: heavy,
                generation,
                run,
                pushes,
                phase: Phase::Building,
            },
        );
        Ok(Ok(Permit {
            owner: self.clone(),
            id,
            phase: Phase::Building,
            started: false,
            finished: false,
        }))
    }
    pub(super) fn push(
        &self,
        run: &str,
        id: &str,
        reference: &str,
        authority: &str,
    ) -> Result<Permit, Status> {
        let (run, id) = identity(run, id)?;
        let mut state = self
            .0
            .lock()
            .map_err(|_| Status::internal("Build admission unavailable"))?;
        if authority != state.authority.to_string() {
            return Err(Status::failed_precondition(
                "Build admission identity changed",
            ));
        }
        let attempt = state
            .attempts
            .get_mut(&id)
            .filter(|v| v.run == run)
            .ok_or_else(|| Status::failed_precondition("Build attempt is not admitted"))?;
        let Phase::WaitingPush(index) = attempt.phase else {
            return Err(Status::failed_precondition(
                "Build attempt is not ready for a push",
            ));
        };
        if attempt.pushes.get(index).map(String::as_str) != Some(reference) {
            return Err(Status::failed_precondition(
                "Push does not match the admitted build plan",
            ));
        }
        attempt.phase = Phase::Pushing(index);
        Ok(Permit {
            owner: self.clone(),
            id,
            phase: attempt.phase,
            started: false,
            finished: false,
        })
    }
}
impl Permit {
    pub(super) fn start(&mut self) {
        self.started = true;
    }
    pub(super) fn complete(&mut self, succeeded: bool) {
        let Ok(mut state) = self.owner.0.lock() else {
            return;
        };
        let Some(attempt) = state.attempts.get_mut(&self.id) else {
            return;
        };
        if attempt.phase != self.phase {
            return;
        }
        let next = match self.phase {
            Phase::Building => 0,
            Phase::Pushing(index) => index + 1,
            _ => return,
        };
        if succeeded && next < attempt.pushes.len() {
            attempt.phase = Phase::WaitingPush(next);
        } else {
            state.attempts.remove(&self.id);
        }
        self.finished = true;
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let Ok(mut state) = self.owner.0.lock() else {
            return;
        };
        let Some(attempt) = state.attempts.get_mut(&self.id) else {
            return;
        };
        if attempt.phase != self.phase {
            return;
        }
        if self.started {
            // Dropping a stream/process is not evidence that daemon work stopped.
            attempt.phase = Phase::Uncertain;
        } else if let Phase::Pushing(index) = self.phase {
            attempt.phase = Phase::WaitingPush(index);
        } else {
            state.attempts.remove(&self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ids() -> (String, String) {
        (Uuid::now_v7().to_string(), Uuid::now_v7().to_string())
    }
    #[test]
    fn cleanup_fences_delayed_requests_without_interrupting_other_work() {
        use citadel_contracts::citadel::images::v1::BuildCleanupState as Cleanup;
        let ledger = BuildAdmission::new(2);
        let authority = ledger.authority().unwrap();
        let (run, attempt) = ids();
        let (other_run, other_attempt) = ids();
        let mut other = ledger
            .build(
                &other_run,
                &other_attempt,
                vec!["other".into()],
                &authority,
                1,
            )
            .unwrap()
            .unwrap();
        other.start();
        assert_eq!(
            ledger
                .reconcile(&other_run, &other_attempt, &authority, 1)
                .unwrap(),
            Cleanup::BuildCleanupActive
        );
        let mut permit = ledger
            .build(&run, &attempt, vec!["image".into()], &authority, 1)
            .unwrap()
            .unwrap();
        permit.start();
        permit.complete(true);
        assert_eq!(
            ledger.reconcile(&run, &attempt, &authority, 1).unwrap(),
            Cleanup::BuildCleanupConfirmed
        );
        assert_eq!(ledger.capacity().unwrap(), (2, 1, 0));
        assert_eq!(ledger.generation().unwrap(), 2);
        assert!(ledger.push(&run, &attempt, "image", &authority).is_err());
        assert_eq!(
            ledger
                .build(&run, &attempt, vec![], &authority, 1)
                .unwrap()
                .err()
                .unwrap(),
            AdmissionRefusal::Fenced
        );
        // Lost cleanup acknowledgment is safe to repeat and does not keep
        // invalidating newly advertised admissions.
        assert_eq!(
            ledger.reconcile(&run, &attempt, &authority, 1).unwrap(),
            Cleanup::BuildCleanupConfirmed
        );
        assert_eq!(ledger.generation().unwrap(), 2);
        other.complete(true);
        let mut push = ledger
            .push(&other_run, &other_attempt, "other", &authority)
            .unwrap();
        push.start();
        push.complete(true);
        let (fresh_run, fresh_attempt) = ids();
        assert!(
            ledger
                .build(&fresh_run, &fresh_attempt, vec![], &authority, 2)
                .unwrap()
                .is_ok()
        );
    }

    #[test]
    fn unknown_work_requires_matching_authority_and_uncertain_work_stays_held() {
        use citadel_contracts::citadel::images::v1::BuildCleanupState as Cleanup;
        let ledger = BuildAdmission::new(1);
        let authority = ledger.authority().unwrap();
        let (run, attempt) = ids();
        assert!(
            ledger
                .reconcile(&run, &attempt, &Uuid::now_v7().to_string(), 1)
                .is_err()
        );
        assert!(ledger.reconcile(&run, &attempt, &authority, 0).is_err());
        assert!(ledger.reconcile(&run, &attempt, &authority, 2).is_err());
        assert_eq!(
            ledger.reconcile(&run, &attempt, &authority, 1).unwrap(),
            Cleanup::BuildCleanupConfirmed
        );
        let mut permit = ledger
            .build(&run, &attempt, vec![], &authority, 2)
            .unwrap()
            .unwrap();
        permit.start();
        drop(permit);
        assert!(ledger.reconcile(&run, &attempt, &authority, 1).is_err());
        assert!(
            ledger
                .reconcile(&Uuid::now_v7().to_string(), &attempt, &authority, 2)
                .is_err()
        );
        assert!(
            ledger
                .reconcile(&run, &Uuid::now_v7().to_string(), &authority, 2)
                .is_err()
        );
        assert_eq!(
            ledger.reconcile(&run, &attempt, &authority, 2).unwrap(),
            Cleanup::BuildCleanupUncertain
        );
        assert_eq!(ledger.capacity().unwrap(), (1, 1, 1));
        // Process replacement must reject old delayed build AND cleanup calls.
        let restarted = BuildAdmission::new(1);
        assert!(
            restarted
                .build(&run, &attempt, vec![], &authority, 2)
                .is_err()
        );
        assert!(restarted.reconcile(&run, &attempt, &authority, 2).is_err());
    }

    #[test]
    fn slot_covers_build_all_pushes_and_rejects_duplicate_phases() {
        let ledger = BuildAdmission::new(1);
        let (run, attempt) = ids();
        let mut build = ledger
            .build(
                &run,
                &attempt,
                vec!["one".into(), "two".into()],
                &ledger.authority().unwrap(),
                1,
            )
            .unwrap()
            .unwrap();
        assert!(
            ledger
                .build(&run, &attempt, vec![], &ledger.authority().unwrap(), 1)
                .is_err()
        );
        assert!(
            ledger
                .push(&run, &attempt, "one", &ledger.authority().unwrap())
                .is_err()
        );
        build.start();
        build.complete(true);
        let (other, other_attempt) = ids();
        assert!(
            ledger
                .build(
                    &other,
                    &other_attempt,
                    vec![],
                    &ledger.authority().unwrap(),
                    1
                )
                .unwrap()
                .is_err()
        );
        assert!(
            ledger
                .push(&run, &attempt, "two", &ledger.authority().unwrap())
                .is_err()
        );
        let mut push = ledger
            .push(&run, &attempt, "one", &ledger.authority().unwrap())
            .unwrap();
        assert!(
            ledger
                .push(&run, &attempt, "one", &ledger.authority().unwrap())
                .is_err()
        );
        push.start();
        push.complete(true);
        assert_eq!(ledger.capacity().unwrap(), (1, 1, 0));
        let mut push = ledger
            .push(&run, &attempt, "two", &ledger.authority().unwrap())
            .unwrap();
        push.start();
        push.complete(true);
        assert_eq!(ledger.capacity().unwrap(), (1, 0, 0));
        assert!(
            ledger
                .push(&run, &attempt, "two", &ledger.authority().unwrap())
                .is_err()
        );
    }
    #[test]
    fn unpolled_requests_release_but_started_dropped_work_stays_quarantined() {
        let ledger = BuildAdmission::new(1);
        let (run, attempt) = ids();
        drop(
            ledger
                .build(&run, &attempt, vec![], &ledger.authority().unwrap(), 1)
                .unwrap()
                .unwrap(),
        );
        assert_eq!(ledger.capacity().unwrap(), (1, 0, 0));
        let mut permit = ledger
            .build(&run, &attempt, vec![], &ledger.authority().unwrap(), 1)
            .unwrap()
            .unwrap();
        permit.start();
        drop(permit);
        assert_eq!(ledger.clone().capacity().unwrap(), (1, 1, 1));
        assert!(
            ledger
                .build(&run, &attempt, vec![], &ledger.authority().unwrap(), 1)
                .is_err()
        );
        let (run, attempt) = ids();
        assert!(
            ledger
                .build(&run, &attempt, vec![], &ledger.authority().unwrap(), 1)
                .unwrap()
                .is_err()
        );
    }
}

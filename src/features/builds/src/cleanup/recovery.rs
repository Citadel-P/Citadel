use super::*;

impl BuildCleanupService {
    pub async fn resolve(
        &self,
        run_id: Uuid,
        attempt_id: Uuid,
        actor: ActorId,
        reason: &str,
        acknowledged: bool,
        cancellation: &CancellationToken,
    ) -> Result<(), BuildError> {
        let reason = reason.trim();
        if !acknowledged || reason.is_empty() || reason.chars().count() > 1000 {
            return Err(BuildError::Validation(
                "Confirm that the old build and pushes have stopped and provide a reason (1–1000 characters).".into(),
            ));
        }
        let reservation = self
            .store
            .retained_reservation(run_id)
            .await?
            .filter(|reservation| reservation.attempt_id == attempt_id)
            .ok_or_else(|| {
                BuildError::Conflict(
                    "This run no longer has the selected retained reservation. Refresh the run."
                        .into(),
                )
            })?;
        let host = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(BuildError::Conflict("Recovery was interrupted. Refresh the run before retrying.".into())),
            result = tokio::time::timeout(Duration::from_secs(5), self.checker.recovery_host(&reservation.target, cancellation)) =>
                result.map_err(|_| BuildError::Conflict("The original Agent did not respond. Restore its connection before resolving cleanup.".into()))??,
        };
        validate_recovery_host(&reservation, &host)?;
        if !self
            .store
            .resolve_reservation(&reservation, actor, reason, host.authority_id)
            .await?
        {
            return Err(BuildError::Conflict(
                "The reservation changed or was already resolved. Refresh the run.".into(),
            ));
        }
        Ok(())
    }
}

fn validate_recovery_host(
    reservation: &BuildRetainedReservation,
    host: &BuildExecutorCapacity,
) -> Result<(), BuildError> {
    if host.daemon_id != reservation.daemon_id {
        return Err(BuildError::Conflict("The Agent is connected to a different Docker daemon. Restore the original target before resolving cleanup.".into()));
    }
    if host.authority_id.is_nil() || host.authority_id == reservation.authority_id {
        return Err(BuildError::Conflict("Stop the old build and pushes, then restart the original Agent before resolving cleanup.".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct Store {
        reservation: BuildRetainedReservation,
        writes: Mutex<Vec<String>>,
        changed: bool,
    }
    impl BuildCleanupStore for Store {
        fn retained_reservation(
            &self,
            _: Uuid,
        ) -> BoxFuture<'_, Result<Option<BuildRetainedReservation>, BuildError>> {
            Box::pin(async { Ok(Some(self.reservation.clone())) })
        }
        fn resolve_reservation<'a>(
            &'a self,
            _: &'a BuildRetainedReservation,
            _: ActorId,
            reason: &'a str,
            _: Uuid,
        ) -> BoxFuture<'a, Result<bool, BuildError>> {
            Box::pin(async move {
                self.writes.lock().unwrap().push(reason.into());
                Ok(!self.changed)
            })
        }
        fn pending_cleanup(
            &self,
            _: Option<Uuid>,
            _: usize,
        ) -> BoxFuture<'_, Result<Vec<BuildCleanupAttempt>, BuildError>> {
            unimplemented!()
        }
        fn finish_cleanup<'a>(
            &'a self,
            _: &'a BuildCleanupAttempt,
        ) -> BoxFuture<'a, Result<bool, BuildError>> {
            unimplemented!()
        }
    }
    struct Checker(Result<BuildExecutorCapacity, BuildError>);
    impl BuildCleanupChecker for Checker {
        fn recovery_host<'a>(
            &'a self,
            _: &'a BuildPlatformSnapshot,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<BuildExecutorCapacity, BuildError>> {
            Box::pin(async { self.0.clone() })
        }
        fn confirmed<'a>(
            &'a self,
            _: &'a BuildCleanupAttempt,
            _: &'a CancellationToken,
        ) -> BoxFuture<'a, Result<bool, BuildError>> {
            unimplemented!()
        }
    }
    fn fixture() -> (Store, BuildExecutorCapacity) {
        (
            Store {
                reservation: BuildRetainedReservation {
                    run_id: Uuid::now_v7(),
                    attempt_id: Uuid::now_v7(),
                    daemon_id: "original".into(),
                    authority_id: Uuid::now_v7(),
                    target: super::super::tests::attempt(1).target,
                },
                writes: Mutex::default(),
                changed: false,
            },
            BuildExecutorCapacity {
                daemon_id: "original".into(),
                authority_id: Uuid::now_v7(),
                admission_generation: 1,
                build_slots: 2,
                heavy_slots: 2,
            },
        )
    }
    #[tokio::test]
    async fn requires_acknowledgment_reason_current_attempt_and_fenced_host() {
        let (store, host) = fixture();
        let store = Arc::new(store);
        let service = BuildCleanupService {
            store: store.clone(),
            checker: Arc::new(Checker(Ok(host.clone()))),
        };
        let r = &store.reservation;
        let actor = ActorId::new(Uuid::now_v7());
        let cancel = CancellationToken::new();
        for (id, reason, ack) in [
            (r.attempt_id, "Checked", false),
            (r.attempt_id, "   ", true),
            (Uuid::now_v7(), "Checked", true),
            (r.attempt_id, &"a".repeat(1001), true),
        ] {
            assert!(
                service
                    .resolve(r.run_id, id, actor, reason, ack, &cancel)
                    .await
                    .is_err()
            );
        }
        for host in [
            Err(BuildError::Conflict("offline".into())),
            Ok(BuildExecutorCapacity {
                daemon_id: "other".into(),
                ..host.clone()
            }),
            Ok(BuildExecutorCapacity {
                authority_id: r.authority_id,
                ..host.clone()
            }),
            Ok(BuildExecutorCapacity {
                authority_id: Uuid::nil(),
                ..host
            }),
        ] {
            let service = BuildCleanupService {
                store: store.clone(),
                checker: Arc::new(Checker(host)),
            };
            assert!(
                service
                    .resolve(r.run_id, r.attempt_id, actor, "Checked", true, &cancel)
                    .await
                    .is_err()
            );
        }
        cancel.cancel();
        assert!(
            service
                .resolve(r.run_id, r.attempt_id, actor, "Checked", true, &cancel)
                .await
                .is_err()
        );
        assert!(store.writes.lock().unwrap().is_empty());
        service
            .resolve(
                r.run_id,
                r.attempt_id,
                actor,
                "  Verified cleanup  ",
                true,
                &CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(*store.writes.lock().unwrap(), vec!["Verified cleanup"]);
    }
    #[tokio::test]
    async fn does_not_report_success_when_reservation_changed_during_the_host_check() {
        let (mut store, host) = fixture();
        store.changed = true;
        let r = store.reservation.clone();
        let service = BuildCleanupService {
            store: Arc::new(store),
            checker: Arc::new(Checker(Ok(host))),
        };
        assert!(matches!(
            service
                .resolve(
                    r.run_id,
                    r.attempt_id,
                    ActorId::new(Uuid::now_v7()),
                    "Checked",
                    true,
                    &CancellationToken::new()
                )
                .await,
            Err(BuildError::Conflict(_))
        ));
    }
}

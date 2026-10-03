//! Independent workload budgets keep slow inventory from consuming health capacity.
use crate::runtime_metrics::{RuntimeTimer, RuntimeWork};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct IoBudget {
    permits: Arc<Semaphore>,
    family: RuntimeWork,
}

impl IoBudget {
    pub fn new(limit: std::num::NonZeroUsize, family: RuntimeWork) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(limit.get())),
            family,
        }
    }

    /// Reuse the same semaphore while attributing work to a narrower fixed family.
    pub fn measured_as(&self, family: RuntimeWork) -> Self {
        Self {
            permits: self.permits.clone(),
            family,
        }
    }

    pub async fn enter(&self, cancellation: &CancellationToken) -> Option<IoPermit> {
        let started = std::time::Instant::now();
        let saturated = self.permits.available_permits() == 0;
        let permit = tokio::select! {
            biased;
            () = cancellation.cancelled() => return None,
            permit = self.permits.clone().acquire_owned() => permit.ok()?,
        };
        self.family.permit_wait(started.elapsed(), saturated);
        Some(IoPermit {
            _permit: permit,
            _timer: self.family.start(),
        })
    }
}

pub struct IoPermit {
    _permit: OwnedSemaphorePermit,
    _timer: RuntimeTimer,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn bounds_concurrent_calls_and_keeps_health_independent() {
        let inventory = IoBudget::new(2.try_into().unwrap(), RuntimeWork::Inventory);
        let health = IoBudget::new(1.try_into().unwrap(), RuntimeWork::Health);
        let token = CancellationToken::new();
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut tasks = tokio::task::JoinSet::new();
        for index in 0..20 {
            let (budget, token, active, peak) = (
                if index % 2 == 0 {
                    inventory.measured_as(RuntimeWork::EventContainersRefresh)
                } else {
                    inventory.clone()
                },
                token.clone(),
                active.clone(),
                peak.clone(),
            );
            tasks.spawn(async move {
                let _permit = budget.enter(&token).await.unwrap();
                let in_flight = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(in_flight, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                active.fetch_sub(1, Ordering::SeqCst);
            });
        }
        tokio::time::timeout(std::time::Duration::from_secs(1), health.enter(&token))
            .await
            .unwrap()
            .unwrap();
        while let Some(result) = tasks.join_next().await {
            result.unwrap();
        }
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn cancellation_interrupts_a_saturated_budget() {
        let budget = IoBudget::new(1.try_into().unwrap(), RuntimeWork::Health);
        let token = CancellationToken::new();
        let _held = budget.enter(&token).await.unwrap();
        token.cancel();
        assert!(budget.enter(&token).await.is_none());
    }
}

use crate::*;
use chrono::Utc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct AlertDeliveryService {
    store: std::sync::Arc<dyn AlertRepository>,
    delivery: std::sync::Arc<dyn AlertDelivery>,
    owner: Uuid,
    stale_after: chrono::Duration,
    maximum_attempts: i32,
    next_maintenance_at: AtomicI64,
}

impl AlertDeliveryService {
    pub fn new(
        store: std::sync::Arc<dyn AlertRepository>,
        delivery: std::sync::Arc<dyn AlertDelivery>,
    ) -> Self {
        Self {
            store,
            delivery,
            owner: Uuid::now_v7(),
            stale_after: chrono::Duration::minutes(2),
            maximum_attempts: 8,
            next_maintenance_at: AtomicI64::new(0),
        }
    }

    pub async fn process_one(&self, cancellation: &CancellationToken) -> Result<bool, AlertError> {
        self.maintain_if_due().await?;
        let Some(claim) = self
            .store
            .claim_delivery(self.owner, Utc::now() - self.stale_after)
            .await?
        else {
            return Ok(false);
        };
        match self
            .delivery
            .send(&claim.channel, &claim.event, cancellation)
            .await
        {
            Ok(()) => {
                self.store.complete_delivery(claim.id, self.owner).await?;
            }
            Err(error) => {
                let attempt = claim.attempt_count.saturating_add(1);
                let dead_letter = attempt >= self.maximum_attempts;
                self.store
                    .retry_delivery(
                        claim.id,
                        self.owner,
                        Utc::now() + retry_delay(attempt),
                        dead_letter,
                        &error.to_string(),
                    )
                    .await?;
            }
        }
        Ok(true)
    }

    async fn maintain_if_due(&self) -> Result<(), AlertError> {
        let now = Utc::now();
        let timestamp = now.timestamp();
        let due = self.next_maintenance_at.load(Ordering::Relaxed);
        if timestamp < due
            || self
                .next_maintenance_at
                .compare_exchange(due, timestamp + 60, Ordering::AcqRel, Ordering::Relaxed)
                .is_err()
        {
            return Ok(());
        }
        if let Err(error) = self
            .store
            .maintain_deliveries(now - self.stale_after, now - chrono::Duration::days(30))
            .await
        {
            self.next_maintenance_at.store(0, Ordering::Release);
            return Err(error);
        }
        Ok(())
    }
}

pub(crate) fn retry_delay(attempt: i32) -> chrono::Duration {
    const DELAYS: [i64; 8] = [1, 5, 30, 120, 600, 1_800, 1_800, 1_800];
    let index = usize::try_from(attempt.saturating_sub(1))
        .unwrap_or(DELAYS.len() - 1)
        .min(DELAYS.len() - 1);
    chrono::Duration::seconds(DELAYS[index])
}

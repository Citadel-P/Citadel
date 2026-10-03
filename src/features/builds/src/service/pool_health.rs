use crate::BuildError;
use crate::BuildPoolCheck;
use crate::BuildService;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

impl BuildService {
    /// Read-only capability checks; never provisions builders or replays builds.
    pub async fn monitor_pool_health(
        &self,
        shutdown: &CancellationToken,
    ) -> Result<usize, BuildError> {
        let Some(checker) = &self.pool_checker else {
            return Ok(0);
        };
        let mut after = Uuid::nil();
        let mut updated = 0;
        loop {
            if shutdown.is_cancelled() {
                break;
            }
            let pools = self.store.health_pools(after, 64).await?;
            if pools.is_empty() {
                break;
            }
            for pool in pools {
                after = pool.id;
                let cancellation = shutdown.child_token();
                let result = tokio::select! {
                    () = shutdown.cancelled() => return Ok(updated),
                    result = tokio::time::timeout(std::time::Duration::from_secs(30),checker.check(&pool,&cancellation)) => match result {
                        Ok(Ok(result)) => result,
                        Ok(Err(error)) => BuildPoolCheck {ready:false,message:format!("Citadel Agent build capability check failed: {error}")},
                        Err(_) => { cancellation.cancel(); BuildPoolCheck {ready:false,message:"Citadel Agent build capability check timed out.".into()} }
                    }
                };
                if self.store.record_pool_health(&pool, &result).await? {
                    updated += 1;
                    if let Some(notifier) = &self.pool_change {
                        notifier();
                    }
                }
            }
        }
        Ok(updated)
    }
}

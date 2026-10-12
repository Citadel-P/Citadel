//! Immutable rule definitions only; evaluation state and entitlements stay live.
use super::*;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::{sync::Mutex, time::Instant};

type ConfigurationEntry = (u64, Instant, Arc<Vec<AlertRule>>);

#[derive(Default)]
pub(super) struct ConfigurationCache {
    epoch: AtomicU64,
    entries: Mutex<HashMap<String, ConfigurationEntry>>,
}
impl PostgresAlertRepository {
    pub fn invalidate_configuration(&self) {
        self.configuration.epoch.fetch_add(1, Ordering::AcqRel);
    }
    pub async fn watch_configuration(
        self: Arc<Self>,
        mut wake: tokio::sync::watch::Receiver<()>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<(), std::convert::Infallible> {
        self.invalidate_configuration();
        loop {
            tokio::select! {
                () = cancel.cancelled() => return Ok(()),
                result = wake.changed() => {
                    self.invalidate_configuration();
                    if result.is_err() { return Ok(()); }
                }
            }
        }
    }
    pub(super) async fn commit_configuration(
        &self,
        mut tx: Transaction<'_, Postgres>,
    ) -> Result<(), AlertError> {
        sqlx::query("SELECT pg_notify('citadel_alert_rules','')")
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        self.invalidate_configuration();
        Ok(())
    }
    pub(super) async fn configured_rules(
        &self,
        alert_type: &str,
    ) -> Result<Arc<Vec<AlertRule>>, AlertError> {
        let mut entries = self.configuration.entries.lock().await;
        let epoch = self.configuration.epoch.load(Ordering::Acquire);
        if let Some((version, at, rules)) = entries.get(alert_type)
            && *version == epoch
            && at.elapsed() < Duration::from_secs(60)
        {
            return Ok(rules.clone());
        }
        let rows = sqlx::query(
                r#"SELECT rule.*,
COALESCE(array_agg(relation.alertchannelid) FILTER (WHERE relation.alertchannelid IS NOT NULL),'{}') AS channelids
FROM alertrules rule
LEFT JOIN alertrulechannels relation ON relation.alertruleid=rule.id
WHERE rule.status='Enabled' AND rule.type=$1
GROUP BY rule.id
ORDER BY CASE rule.severity WHEN 'Critical' THEN 3 WHEN 'Warning' THEN 2 ELSE 1 END DESC,
         rule.createdat,rule.id"#,
            )
            .bind(alert_type)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
        let rules = rows
            .into_iter()
            .map(map_rule)
            .collect::<Result<Vec<_>, _>>()?;

        let rules = Arc::new(rules);
        // Oversized configurations still evaluate completely but are not retained.
        if rules.len() <= 1024
            && alert_type.len() <= 128
            && self.configuration.epoch.load(Ordering::Acquire) == epoch
        {
            if entries.len() >= 64 {
                entries.clear();
            }
            entries.insert(
                alert_type.to_owned(),
                (epoch, Instant::now(), rules.clone()),
            );
        }
        Ok(rules)
    }
}

#[cfg(test)]
mod tests;

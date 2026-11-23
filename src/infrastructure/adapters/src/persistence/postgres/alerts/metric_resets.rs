//! Distinct healthy rule/resource pairs usually have a zero counter and no open incident.
use super::rule_evaluation::{RuleEvaluation, incident_key, rule_resource_key};
use super::*;

impl PostgresAlertRepository {
    pub(super) async fn process_healthy_resets(
        &self,
        evaluation: &mut RuleEvaluation<'_, '_>,
        resets: &[(&AlertRule, &AlertObservation)],
    ) -> Result<(), sqlx::Error> {
        let tx = &mut *evaluation.transaction;
        let locked = lock_pairs(tx, resets).await?;
        if locked.is_empty() {
            return Ok(());
        }
        let locked: Vec<_> = locked
            .into_iter()
            .map(|n| resets[(n - 1) as usize])
            .collect();
        let rules: Vec<_> = locked.iter().map(|(rule, _)| rule.id).collect();
        let resources: Vec<_> = locked.iter().map(|(_, obs)| obs.resource_id).collect();
        let incidents: Vec<_> = locked
            .iter()
            .map(|(rule, obs)| incident_key(rule, obs))
            .collect();
        // A separate statement gets a fresh snapshot after any lock wait. A
        // triggered incident can have a zero counter and still need resolution.
        let pending: Vec<i64> = sqlx::query_scalar(r#"
            SELECT pair.ordinal
            FROM unnest($1::uuid[],$2::uuid[],$3::text[]) WITH ORDINALITY pair(ruleid,resourceid,key,ordinal)
            LEFT JOIN alertrulestates state ON state.alertruleid=pair.ruleid AND state.resourceid=pair.resourceid
            WHERE state.consecutivematches IS DISTINCT FROM 0 OR EXISTS (
                SELECT 1 FROM alertevents WHERE openincidentkey=pair.key AND resolvedat IS NULL
            )"#)
            .bind(&rules)
            .bind(&resources)
            .bind(&incidents)
            .fetch_all(&mut **tx)
            .await?;
        for ordinal in pending {
            let (rule, observation) = locked[(ordinal - 1) as usize];
            self.process_rule_in(evaluation, rule, observation, None)
                .await
                .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        }
        Ok(())
    }
}

pub(super) async fn lock_pairs(
    tx: &mut Transaction<'_, Postgres>,
    resets: &[(&AlertRule, &AlertObservation)],
) -> Result<Vec<i64>, sqlx::Error> {
    let rules: Vec<_> = resets.iter().map(|(rule, _)| rule.id).collect();
    let keys: Vec<_> = resets
        .iter()
        .map(|(rule, obs)| rule_resource_key(rule, obs))
        .collect();
    // Lock enabled parents first, then the same resource keys as the single
    // observation path. Retain only pairs with a live, locked parent.
    sqlx::query_scalar(r#"
            WITH enabled AS MATERIALIZED (
                SELECT id FROM alertrules WHERE id=ANY($1) AND status='Enabled' ORDER BY id FOR SHARE
            )
            SELECT pair.ordinal, pg_advisory_xact_lock(hashtextextended(pair.key,0))
            FROM unnest($1::uuid[],$2::text[]) WITH ORDINALITY pair(ruleid,key,ordinal)
            JOIN enabled ON enabled.id=pair.ruleid ORDER BY pair.key"#)
            .bind(&rules)
            .bind(&keys)
            .fetch_all(&mut **tx)
            .await
}

impl PostgresAlertRepository {
    pub(super) async fn reset_stack_drift_batch(
        &self,
        observations: &[AlertObservation],
    ) -> Result<(), AlertError> {
        if observations.is_empty() {
            return Ok(());
        }
        // This fast path assumes distinct reset-only observations. Keep the
        // ordinary reducer's order and special resolution semantics otherwise.
        let mut seen = std::collections::HashSet::new();
        if observations.iter().any(|obs| {
            obs.alert_type != "StackDriftDetected"
                || obs.matched
                || obs.value.is_some()
                || !seen.insert(obs.resource_id)
        }) {
            for obs in observations {
                self.process_event_impl(obs).await?;
            }
            return Ok(());
        }
        let rules = self.configured_rules("StackDriftDetected").await?;
        let advanced = !rules
            .iter()
            .any(|rule| rule.audit.created_by_actor_id.value() != Uuid::from_u128(1))
            || self.entitlements.advanced_alerting().await?;
        let mut pairs = Vec::new();
        for obs in observations {
            let (resets, _) =
                citadel_alerts::rules::evaluation::select_observation_rules(&rules, obs, advanced);
            for rule in resets {
                pairs.push((rule, obs));
                if pairs.len() == 512 {
                    self.commit_resets(&pairs).await?;
                    pairs.clear();
                }
            }
        }
        if !pairs.is_empty() {
            self.commit_resets(&pairs).await?;
        }
        Ok(())
    }

    async fn commit_resets(
        &self,
        pairs: &[(&AlertRule, &AlertObservation)],
    ) -> Result<(), AlertError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let mut evaluation = RuleEvaluation::new(&mut tx);
        self.process_healthy_resets(&mut evaluation, pairs)
            .await
            .map_err(storage)?;
        let changed = evaluation.changed;
        tx.commit().await.map_err(storage)?;
        if changed {
            self.changed();
        }
        Ok(())
    }
}

//! A metric flush observes each rule/platform pair at most once. Most healthy
//! pairs already have a zero counter and no incident to resolve.
use super::rule_evaluation::{RuleEvaluation, incident_key, rule_resource_key};
use super::*;

impl PostgresAlertRepository {
    pub(super) async fn process_metric_resets(
        &self,
        evaluation: &mut RuleEvaluation<'_, '_>,
        resets: &[(&AlertRule, &AlertObservation)],
    ) -> Result<(), sqlx::Error> {
        let tx = &mut *evaluation.transaction;
        let rules: Vec<_> = resets.iter().map(|(rule, _)| rule.id).collect();
        let keys: Vec<_> = resets
            .iter()
            .map(|(rule, obs)| rule_resource_key(rule, obs))
            .collect();
        // Lock enabled parents first, then the same resource keys as the single
        // observation path. Retain only pairs with a live, locked parent.
        let locked: Vec<i64> = sqlx::query_scalar(r#"
            WITH enabled AS MATERIALIZED (
                SELECT id FROM alertrules WHERE id=ANY($1) AND status='Enabled' ORDER BY id FOR SHARE
            )
            SELECT pair.ordinal, pg_advisory_xact_lock(hashtextextended(pair.key,0))
            FROM unnest($1::uuid[],$2::text[]) WITH ORDINALITY pair(ruleid,key,ordinal)
            JOIN enabled ON enabled.id=pair.ruleid ORDER BY pair.key"#)
            .bind(&rules)
            .bind(&keys)
            .fetch_all(&mut **tx)
            .await?;
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

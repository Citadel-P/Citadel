//! A platform flush selects at most one observation per rule/resource pair.
use super::{
    metric_resets::lock_pairs,
    rule_evaluation::{RuleEvaluation, incident_key},
    *,
};

pub(super) async fn apply(
    evaluation: &mut RuleEvaluation<'_, '_>,
    pairs: &[(&AlertRule, &AlertObservation)],
) -> Result<(), sqlx::Error> {
    let tx = &mut *evaluation.transaction;
    let locked = lock_pairs(tx, pairs).await?;
    if locked.is_empty() {
        return Ok(());
    }
    let mut by_pair = HashMap::with_capacity(locked.len());
    let mut rules = Vec::with_capacity(locked.len());
    let mut resources = Vec::with_capacity(locked.len());
    let mut actors = Vec::with_capacity(locked.len());
    for ordinal in locked {
        let (rule, obs) = pairs[(ordinal - 1) as usize];
        rules.push(rule.id);
        resources.push(obs.resource_id);
        actors.push(rule.audit.created_by_actor_id.value());
        by_pair.insert((rule.id, obs.resource_id), (rule, obs));
    }
    // New statement after the advisory locks. PostgreSQL reads and advances
    // the live counter atomically, including after a concurrent writer commits.
    let states: Vec<(Uuid,Uuid,i32,Option<DateTime<Utc>>)> = sqlx::query_as(r#"INSERT INTO alertrulestates(alertruleid,resourceid,consecutivematches,createdbyactorid)
SELECT ruleid,resourceid,1,actor FROM unnest($1::uuid[],$2::uuid[],$3::uuid[]) p(ruleid,resourceid,actor)
ON CONFLICT(alertruleid,resourceid) DO UPDATE SET
    consecutivematches=least(alertrulestates.consecutivematches::bigint+1,2147483647)::integer,lastobservedat=NULL
RETURNING alertruleid,resourceid,consecutivematches,lasttriggeredat"#)
        .bind(rules).bind(resources).bind(actors).fetch_all(&mut **tx).await?;
    let mut events = Vec::new();
    for (rule_id, resource, count, last) in states {
        let (rule, obs) = by_pair[&(rule_id, resource)];
        if citadel_alerts::rules::evaluation::trigger_due(rule, count, last, obs.observed_at) {
            events.push(serde_json::json!({
                "id": Uuid::now_v7(),
                "ruleid": rule_id,
                "resourceid": resource,
                "name": obs.resource_name,
                "kind": obs.alert_type,
                "severity": rule.severity.as_str(),
                "info": obs.info,
                "key": incident_key(rule, obs),
                "observed": obs.observed_at,
            }));
        }
    }
    if events.is_empty() {
        return Ok(());
    }
    let (created, deliveries): (i64, i64) = sqlx::query_as(include_str!("metric_incidents.sql"))
        .bind(Value::Array(events))
        .fetch_one(&mut **tx)
        .await?;
    if deliveries > 0 {
        sqlx::query("SELECT pg_notify($1,'')")
            .bind(citadel_runtime::RuntimeSignal::AlertDelivery.channel())
            .execute(&mut **tx)
            .await?;
    }
    evaluation.changed |= created > 0;
    Ok(())
}

// A resolver takes precedence over the acknowledger. Older automatic resolutions
// have no resolver ID; they belong to System, not to the person who acknowledged.
pub(super) const EVENT_SELECT: &str = r#"
SELECT event.*, responsible.id AS actor_id,
       COALESCE(actor_user.name, service_account.name,
                CASE WHEN actor.type='System' THEN 'System' END,
                CASE WHEN responsible.id IS NOT NULL THEN 'Unknown' END) AS actor_name,
       actor.type AS actor_type
FROM alertevents event
LEFT JOIN LATERAL (
    SELECT CASE WHEN event.resolvedat IS NOT NULL
                THEN COALESCE(event.resolvedbyactorid,'00000000-0000-0000-0000-000000000001'::uuid)
                ELSE event.acknowledgedbyactorid END AS id
) responsible ON true
LEFT JOIN actors actor ON actor.id=responsible.id
LEFT JOIN users actor_user ON actor_user.actorid=actor.id
LEFT JOIN serviceaccounts service_account ON service_account.actorid=actor.id
"#;

pub(super) const AUTHORIZED_CTE: &str = r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid FROM actors actor
    WHERE actor.id=$1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors team_actor ON team_actor.id=team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid=$1
), global_access AS (
    SELECT EXISTS (
        SELECT 1 FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid=scope.actorid
        JOIN permissions permission ON permission.roleid=assignment.roleid
        WHERE permission.resourcetype=$2
          AND permission.permissionlevel = ANY($3)
    ) AS allowed
)
"#;

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

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

// List and latest-run projections deliberately do not load retained source/log payloads.
pub(super) const RUN_SUMMARY_COLUMNS: &str = "id,actionid,actionname,trigger,status,runasactorid,triggeredbyactorid,argsjson,NULL::text AS codesnapshot,codehash,timeoutseconds,queuedat,startedat,finishedat,durationms,exitcode,NULL::text AS logs,errormessage";

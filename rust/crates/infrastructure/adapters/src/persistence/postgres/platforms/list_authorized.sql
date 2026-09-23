WITH ActorScope AS (
    SELECT principalActor.Id AS ActorId
    FROM Actors principalActor
    LEFT JOIN Users principalUser ON principalUser.ActorId = principalActor.Id
    WHERE (principalActor.Id = $1 OR principalUser.Id = $1)
      AND principalActor.IsEnabled

    UNION

    SELECT t.ActorId
    FROM Teams t
    JOIN ActorTeamMemberships membership ON membership.TeamId = t.Id
    JOIN Actors principalActor ON principalActor.Id = membership.MemberActorId
    LEFT JOIN Users principalUser ON principalUser.ActorId = principalActor.Id
    JOIN Actors teamActor ON teamActor.Id = t.ActorId
    WHERE (principalActor.Id = $1 OR principalUser.Id = $1)
      AND principalActor.IsEnabled
      AND teamActor.IsEnabled
),
GlobalAccess AS (
    SELECT 1 AS HasAccess
    FROM ActorRoles ar
    JOIN Permissions p ON p.RoleId = ar.RoleId
    JOIN ActorScope actorScope ON actorScope.ActorId = ar.ActorId
    WHERE p.ResourceType = $2
      AND (p.PermissionLevel & $3) <> 0
      AND ($4 = 0 OR (p.SpecificPermissions & $4) = $4)
    LIMIT 1
)
SELECT p.Id, p.Name, p.Address, p.Status, p.ConnectorType
FROM Platforms p
WHERE EXISTS (SELECT 1 FROM GlobalAccess)
   OR EXISTS (
        SELECT 1
        FROM ResourceAccesses ra
        JOIN ActorScope actorScope ON actorScope.ActorId = ra.ActorId
        WHERE ra.ResourceType = $2
          AND (ra.PermissionLevel & $3) <> 0
          AND ($4 = 0 OR (ra.SpecificPermissions & $4) = $4)
          AND ra.ResourceId = p.Id
   )
ORDER BY p.Name

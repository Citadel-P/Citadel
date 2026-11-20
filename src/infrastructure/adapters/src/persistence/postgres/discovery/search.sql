-- Port of GlobalSearchRepository: bound candidate limits before projection joins.
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
GlobalResourceTypes AS (
    SELECT permissions.ResourceType
    FROM ActorRoles actorRoles
    JOIN Permissions permissions ON permissions.RoleId = actorRoles.RoleId
    JOIN ActorScope actorScope ON actorScope.ActorId = actorRoles.ActorId
    WHERE (permissions.PermissionLevel & $9) <> 0
),
ScopedResources AS (
    SELECT resourceAccesses.ResourceType, resourceAccesses.ResourceId
    FROM ResourceAccesses resourceAccesses
    JOIN ActorScope actorScope ON actorScope.ActorId = resourceAccesses.ActorId
    WHERE (resourceAccesses.PermissionLevel & $9) <> 0
),
PlatformCandidates AS (
    SELECT
        platform.Id,
        CASE
            WHEN lower(platform.Name) = lower($4) THEN 0
            WHEN platform.Name ILIKE $5 ESCAPE E'\\' THEN 1
            WHEN platform.Name ILIKE $6 ESCAPE E'\\' THEN 2
            WHEN platform.Address ILIKE $5 ESCAPE E'\\' THEN 3
            ELSE 4
        END AS MatchRank
    FROM Platforms platform
    WHERE $10 = ANY($3)
      AND (
          platform.Name ILIKE $6 ESCAPE E'\\'
          OR platform.Address ILIKE $6 ESCAPE E'\\')
      AND ($2 OR $10 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR platform.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $10))
    ORDER BY MatchRank, lower(platform.Name), platform.Id
    LIMIT $7
),
StackCandidates AS (
    SELECT
        stack.Id,
        CASE
            WHEN lower(stack.Name) = lower($4) THEN 0
            WHEN stack.Name ILIKE $5 ESCAPE E'\\' THEN 1
            ELSE 2
        END AS MatchRank
    FROM Stacks stack
    WHERE $11 = ANY($3)
      AND stack.Name ILIKE $6 ESCAPE E'\\'
      AND ($2 OR $11 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR stack.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $11))
    ORDER BY MatchRank, lower(stack.Name), stack.Id
    LIMIT $7
),
DeploymentCandidates AS (
    SELECT
        deployment.Id,
        CASE
            WHEN lower(deployment.Name) = lower($4) THEN 0
            WHEN deployment.Name ILIKE $5 ESCAPE E'\\' THEN 1
            ELSE 2
        END AS MatchRank
    FROM Deployments deployment
    WHERE $12 = ANY($3)
      AND deployment.Name ILIKE $6 ESCAPE E'\\'
      AND ($2 OR $12 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR deployment.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $12))
    ORDER BY MatchRank, lower(deployment.Name), deployment.Id
    LIMIT $7
),
SwarmServiceCandidates AS (
    SELECT
        service.Id,
        CASE
            WHEN lower(service.Name) = lower($4) THEN 0
            WHEN service.Name ILIKE $5 ESCAPE E'\\' THEN 1
            ELSE 2
        END AS MatchRank
    FROM SwarmServices service
    WHERE $20 = ANY($3)
      AND service.Name ILIKE $6 ESCAPE E'\\'
      AND ($2 OR $20 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR service.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $20))
      AND ($2 OR $10 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR service.PlatformId IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $10))
    ORDER BY MatchRank, lower(service.Name), service.Id
    LIMIT $7
),
GitRepositoryCandidates AS (
    SELECT
        repository.Id,
        CASE
            WHEN lower(repository.Name) = lower($4) THEN 0
            WHEN repository.Name ILIKE $5 ESCAPE E'\\' THEN 1
            WHEN repository.Name ILIKE $6 ESCAPE E'\\' THEN 2
            WHEN repository.Url ILIKE $5 ESCAPE E'\\' THEN 3
            ELSE 4
        END AS MatchRank
    FROM GitRepositories repository
    WHERE $13 = ANY($3)
      AND (
          repository.Name ILIKE $6 ESCAPE E'\\'
          OR repository.Url ILIKE $6 ESCAPE E'\\')
      AND ($2 OR $13 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR repository.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $13))
    ORDER BY MatchRank, lower(repository.Name), repository.Id
    LIMIT $7
),
RegistryCandidates AS (
    SELECT
        registry.Id,
        CASE
            WHEN lower(registry.Name) = lower($4) THEN 0
            WHEN registry.Name ILIKE $5 ESCAPE E'\\' THEN 1
            WHEN registry.Name ILIKE $6 ESCAPE E'\\' THEN 2
            WHEN registry.RegistryHost ILIKE $5 ESCAPE E'\\' THEN 3
            ELSE 4
        END AS MatchRank
    FROM Registries registry
    WHERE $14 = ANY($3)
      AND (
          registry.Name ILIKE $6 ESCAPE E'\\'
          OR registry.RegistryHost ILIKE $6 ESCAPE E'\\')
      AND ($2 OR $14 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR registry.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $14))
    ORDER BY MatchRank, lower(registry.Name), registry.Id
    LIMIT $7
),
ScriptCandidates AS (
    SELECT
        action.Id,
        CASE
            WHEN lower(action.Name) = lower($4) THEN 0
            WHEN action.Name ILIKE $5 ESCAPE E'\\' THEN 1
            ELSE 2
        END AS MatchRank
    FROM scripts action
    WHERE $15 = ANY($3)
      AND action.Name ILIKE $6 ESCAPE E'\\'
      AND ($2 OR $15 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR action.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $15))
    ORDER BY MatchRank, lower(action.Name), action.Id
    LIMIT $7
),
BackupRepositoryCandidates AS (
    SELECT
        repository.Id,
        CASE
            WHEN lower(repository.Name) = lower($4) THEN 0
            WHEN repository.Name ILIKE $5 ESCAPE E'\\' THEN 1
            WHEN repository.Name ILIKE $6 ESCAPE E'\\' THEN 2
            WHEN repository.Type ILIKE $5 ESCAPE E'\\' THEN 3
            ELSE 4
        END AS MatchRank
    FROM BackupRepositories repository
    WHERE $17 = ANY($3)
      AND repository.ArchivedAt IS NULL
      AND (
          repository.Name ILIKE $6 ESCAPE E'\\'
          OR repository.Type ILIKE $6 ESCAPE E'\\')
      AND ($2 OR $17 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR repository.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $17))
    ORDER BY MatchRank, lower(repository.Name), repository.Id
    LIMIT $7
),
BackupPolicyCandidates AS (
    SELECT
        policy.Id,
        CASE
            WHEN lower(policy.Name) = lower($4) THEN 0
            WHEN policy.Name ILIKE $5 ESCAPE E'\\' THEN 1
            ELSE 2
        END AS MatchRank
    FROM BackupPolicies policy
    WHERE $16 = ANY($3)
      AND policy.ArchivedAt IS NULL
      AND policy.Name ILIKE $6 ESCAPE E'\\'
      AND ($2 OR $16 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR policy.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $16))
    ORDER BY MatchRank, lower(policy.Name), policy.Id
    LIMIT $7
),
BuildCandidates AS (
    SELECT
        project.Id,
        CASE
            WHEN lower(project.Name) = lower($4) THEN 0
            WHEN project.Name ILIKE $5 ESCAPE E'\\' THEN 1
            WHEN project.Name ILIKE $6 ESCAPE E'\\' THEN 2
            WHEN project.Branch ILIKE $5 ESCAPE E'\\' THEN 3
            ELSE 4
        END AS MatchRank
    FROM BuildProjects project
    WHERE $18 = ANY($3)
      AND project.ArchivedAt IS NULL
      AND (
          project.Name ILIKE $6 ESCAPE E'\\'
          OR project.Branch ILIKE $6 ESCAPE E'\\')
      AND ($2 OR $18 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR project.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $18))
    ORDER BY MatchRank, lower(project.Name), project.Id
    LIMIT $7
),
BuildAgentPoolCandidates AS (
    SELECT
        pool.Id,
        CASE
            WHEN lower(pool.Name) = lower($4) THEN 0
            WHEN pool.Name ILIKE $5 ESCAPE E'\\' THEN 1
            WHEN pool.Name ILIKE $6 ESCAPE E'\\' THEN 2
            WHEN pool.Provider ILIKE $5 ESCAPE E'\\' THEN 3
            ELSE 4
        END AS MatchRank
    FROM BuildAgentPools pool
    WHERE $19 = ANY($3)
      AND pool.ArchivedAt IS NULL
      AND (
          pool.Name ILIKE $6 ESCAPE E'\\'
          OR pool.Provider ILIKE $6 ESCAPE E'\\')
      AND ($2 OR $19 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR pool.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $19))
    ORDER BY MatchRank, lower(pool.Name), pool.Id
    LIMIT $7
),
Matches AS (
    SELECT
        platform.Id,
        $10 AS ResourceType,
        platform.Name,
        platform.Address AS SecondaryText,
        platform.Status,
        NULL::uuid AS ParentId,
        NULL::integer AS ParentResourceType,
        NULL::text AS ParentName,
        candidate.MatchRank
    FROM PlatformCandidates candidate
    JOIN Platforms platform ON platform.Id = candidate.Id

    UNION ALL

    SELECT
        stack.Id,
        $11 AS ResourceType,
        stack.Name,
        platform.Name AS SecondaryText,
        release.Status,
        platform.Id AS ParentId,
        CASE WHEN platform.Id IS NOT NULL THEN $10 END AS ParentResourceType,
        platform.Name AS ParentName,
        candidate.MatchRank
    FROM StackCandidates candidate
    JOIN Stacks stack ON stack.Id = candidate.Id
    LEFT JOIN StackReleases release ON release.Id = stack.CurrentStackReleaseId
    LEFT JOIN Platforms platform ON platform.Id = release.PlatformId
      AND ($2 OR $10 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR platform.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $10))

    UNION ALL

    SELECT
        deployment.Id,
        $12 AS ResourceType,
        deployment.Name,
        platform.Name AS SecondaryText,
        deployment.Status,
        platform.Id AS ParentId,
        CASE WHEN platform.Id IS NOT NULL THEN $10 END AS ParentResourceType,
        platform.Name AS ParentName,
        candidate.MatchRank
    FROM DeploymentCandidates candidate
    JOIN Deployments deployment ON deployment.Id = candidate.Id
    LEFT JOIN Platforms platform ON platform.Id = deployment.PlatformId
      AND ($2 OR $10 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR platform.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $10))

    UNION ALL

    SELECT
        service.Id,
        $20 AS ResourceType,
        service.Name,
        platform.Name AS SecondaryText,
        service.Health AS Status,
        platform.Id AS ParentId,
        $10 AS ParentResourceType,
        platform.Name AS ParentName,
        candidate.MatchRank
    FROM SwarmServiceCandidates candidate
    JOIN SwarmServices service ON service.Id = candidate.Id
    JOIN Platforms platform ON platform.Id = service.PlatformId

    UNION ALL

    SELECT
        repository.Id,
        $13 AS ResourceType,
        repository.Name,
        repository.Url AS SecondaryText,
        repository.Status,
        NULL::uuid AS ParentId,
        NULL::integer AS ParentResourceType,
        NULL::text AS ParentName,
        candidate.MatchRank
    FROM GitRepositoryCandidates candidate
    JOIN GitRepositories repository ON repository.Id = candidate.Id

    UNION ALL

    SELECT
        registry.Id,
        $14 AS ResourceType,
        registry.Name,
        registry.RegistryHost AS SecondaryText,
        registry.Status,
        NULL::uuid AS ParentId,
        NULL::integer AS ParentResourceType,
        NULL::text AS ParentName,
        candidate.MatchRank
    FROM RegistryCandidates candidate
    JOIN Registries registry ON registry.Id = candidate.Id

    UNION ALL

    SELECT
        action.Id,
        $15 AS ResourceType,
        action.Name,
        NULL::text AS SecondaryText,
        COALESCE(currentRun.Status, latestRun.Status, CASE WHEN action.Enabled THEN 'Enabled' ELSE 'Disabled' END) AS Status,
        NULL::uuid AS ParentId,
        NULL::integer AS ParentResourceType,
        NULL::text AS ParentName,
        candidate.MatchRank
    FROM ScriptCandidates candidate
    JOIN scripts action ON action.Id = candidate.Id
    LEFT JOIN ScriptRuns currentRun ON currentRun.Id = action.CurrentRunId
    LEFT JOIN LATERAL (
        SELECT runs.Status
        FROM ScriptRuns runs
        WHERE runs.ScriptId = action.Id
        ORDER BY runs.QueuedAt DESC, runs.Id DESC
        LIMIT 1
    ) latestRun ON TRUE

    UNION ALL

    SELECT
        repository.Id,
        $17 AS ResourceType,
        repository.Name,
        repository.Type AS SecondaryText,
        repository.Status,
        NULL::uuid AS ParentId,
        NULL::integer AS ParentResourceType,
        NULL::text AS ParentName,
        candidate.MatchRank
    FROM BackupRepositoryCandidates candidate
    JOIN BackupRepositories repository ON repository.Id = candidate.Id

    UNION ALL

    SELECT
        policy.Id,
        $16 AS ResourceType,
        policy.Name,
        COALESCE(
            parentStack.Name,
            parentDeployment.Name,
            policy.Source ->> '$type') AS SecondaryText,
        COALESCE(currentRun.Status, latestRun.Status, CASE WHEN policy.Enabled THEN 'Enabled' ELSE 'Disabled' END) AS Status,
        COALESCE(parentStack.Id, parentDeployment.Id) AS ParentId,
        CASE
            WHEN parentStack.Id IS NOT NULL
                THEN $11
            WHEN parentDeployment.Id IS NOT NULL
                THEN $12
            ELSE NULL
        END AS ParentResourceType,
        COALESCE(parentStack.Name, parentDeployment.Name) AS ParentName,
        candidate.MatchRank
    FROM BackupPolicyCandidates candidate
    JOIN BackupPolicies policy ON policy.Id = candidate.Id
    LEFT JOIN BackupRuns currentRun ON currentRun.Id = policy.CurrentRunId
    LEFT JOIN LATERAL (
        SELECT runs.Status
        FROM BackupRuns runs
        WHERE runs.BackupPolicyId = policy.Id
        ORDER BY runs.QueuedAt DESC, runs.Id DESC
        LIMIT 1
    ) latestRun ON TRUE
    LEFT JOIN Stacks parentStack
        ON policy.Source ->> '$type' = 'Stack'
        AND parentStack.Id = NULLIF(
            COALESCE(policy.Source ->> 'StackId', policy.Source ->> 'stackId'),
            '')::uuid
        AND ($2 OR $11 IN (SELECT ResourceType FROM GlobalResourceTypes)
            OR parentStack.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $11))
    LEFT JOIN Deployments parentDeployment
        ON policy.Source ->> '$type' = 'Deployment'
        AND parentDeployment.Id = NULLIF(
            COALESCE(policy.Source ->> 'DeploymentId', policy.Source ->> 'deploymentId'),
            '')::uuid
        AND ($2 OR $12 IN (SELECT ResourceType FROM GlobalResourceTypes)
            OR parentDeployment.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $12))

    UNION ALL

    SELECT
        project.Id,
        $18 AS ResourceType,
        project.Name,
        project.Branch AS SecondaryText,
        COALESCE(currentRun.Status, latestRun.Status, CASE WHEN project.Enabled THEN 'Enabled' ELSE 'Disabled' END) AS Status,
        CASE
            WHEN project.BuilderKind = 'Platform' AND platform.Id IS NOT NULL
                THEN platform.Id
            WHEN project.BuilderKind = 'BuildAgentPool' AND pool.Id IS NOT NULL
                THEN pool.Id
            ELSE NULL
        END AS ParentId,
        CASE
            WHEN project.BuilderKind = 'Platform' AND platform.Id IS NOT NULL
                THEN $10
            WHEN project.BuilderKind = 'BuildAgentPool' AND pool.Id IS NOT NULL
                THEN $19
            ELSE NULL
        END AS ParentResourceType,
        CASE
            WHEN project.BuilderKind = 'Platform' AND platform.Id IS NOT NULL
                THEN platform.Name
            WHEN project.BuilderKind = 'BuildAgentPool' AND pool.Id IS NOT NULL
                THEN pool.Name
            ELSE NULL
        END AS ParentName,
        candidate.MatchRank
    FROM BuildCandidates candidate
    JOIN BuildProjects project ON project.Id = candidate.Id
    LEFT JOIN BuildRuns currentRun ON currentRun.Id = project.CurrentRunId
    LEFT JOIN LATERAL (
        SELECT runs.Status
        FROM BuildRuns runs
        WHERE runs.BuildProjectId = project.Id
        ORDER BY runs.QueuedAt DESC, runs.Id DESC
        LIMIT 1
    ) latestRun ON TRUE
    LEFT JOIN Platforms platform ON platform.Id = project.PlatformId
      AND ($2 OR $10 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR platform.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $10))
    LEFT JOIN BuildAgentPools pool ON pool.Id = project.BuildAgentPoolId
      AND ($2 OR $19 IN (SELECT ResourceType FROM GlobalResourceTypes)
          OR pool.Id IN (SELECT ResourceId FROM ScopedResources WHERE ResourceType = $19))

    UNION ALL

    SELECT
        pool.Id,
        $19 AS ResourceType,
        pool.Name,
        pool.Provider AS SecondaryText,
        pool.LastValidationStatus AS Status,
        NULL::uuid AS ParentId,
        NULL::integer AS ParentResourceType,
        NULL::text AS ParentName,
        candidate.MatchRank
    FROM BuildAgentPoolCandidates candidate
    JOIN BuildAgentPools pool ON pool.Id = candidate.Id
)
SELECT
    Id,
    ResourceType,
    Name,
    SecondaryText,
    Status,
    ParentId,
    ParentResourceType,
    ParentName,
    MatchRank
FROM Matches
ORDER BY MatchRank, lower(Name), ResourceType, Id
LIMIT $8

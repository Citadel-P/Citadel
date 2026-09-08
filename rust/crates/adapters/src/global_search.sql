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
AuthorizedResources AS (
    SELECT DISTINCT permissions.ResourceType, NULL::uuid AS ResourceId
    FROM ActorRoles actorRoles
    JOIN Permissions permissions ON permissions.RoleId = actorRoles.RoleId
    JOIN ActorScope actorScope ON actorScope.ActorId = actorRoles.ActorId
    WHERE (permissions.PermissionLevel & $9) <> 0

    UNION

    SELECT DISTINCT resourceAccesses.ResourceType, resourceAccesses.ResourceId
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
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $10
                AND (access.ResourceId IS NULL OR access.ResourceId = platform.Id)))
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
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $11
                AND (access.ResourceId IS NULL OR access.ResourceId = stack.Id)))
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
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $12
                AND (access.ResourceId IS NULL OR access.ResourceId = deployment.Id)))
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
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $20
                AND (access.ResourceId IS NULL OR access.ResourceId = service.Id)))
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources platformAccess
              WHERE platformAccess.ResourceType = $10
                AND (platformAccess.ResourceId IS NULL OR platformAccess.ResourceId = service.PlatformId)))
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
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $13
                AND (access.ResourceId IS NULL OR access.ResourceId = repository.Id)))
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
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $14
                AND (access.ResourceId IS NULL OR access.ResourceId = registry.Id)))
    ORDER BY MatchRank, lower(registry.Name), registry.Id
    LIMIT $7
),
AutomationActionCandidates AS (
    SELECT
        action.Id,
        CASE
            WHEN lower(action.Name) = lower($4) THEN 0
            WHEN action.Name ILIKE $5 ESCAPE E'\\' THEN 1
            ELSE 2
        END AS MatchRank
    FROM Actions action
    WHERE $15 = ANY($3)
      AND action.Name ILIKE $6 ESCAPE E'\\'
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $15
                AND (access.ResourceId IS NULL OR access.ResourceId = action.Id)))
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
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $17
                AND (access.ResourceId IS NULL OR access.ResourceId = repository.Id)))
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
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $16
                AND (access.ResourceId IS NULL OR access.ResourceId = policy.Id)))
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
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $18
                AND (access.ResourceId IS NULL OR access.ResourceId = project.Id)))
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
      AND (
          $2
          OR EXISTS (
              SELECT 1
              FROM AuthorizedResources access
              WHERE access.ResourceType = $19
                AND (access.ResourceId IS NULL OR access.ResourceId = pool.Id)))
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
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1
                FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN platform.Name
            ELSE NULL
        END AS SecondaryText,
        release.Status,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1
                FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN platform.Id
            ELSE NULL
        END AS ParentId,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1
                FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN $10
            ELSE NULL
        END AS ParentResourceType,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1
                FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN platform.Name
            ELSE NULL
        END AS ParentName,
        candidate.MatchRank
    FROM StackCandidates candidate
    JOIN Stacks stack ON stack.Id = candidate.Id
    LEFT JOIN StackReleases release ON release.Id = stack.CurrentStackReleaseId
    LEFT JOIN Platforms platform ON platform.Id = release.PlatformId

    UNION ALL

    SELECT
        deployment.Id,
        $12 AS ResourceType,
        deployment.Name,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1
                FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN platform.Name
            ELSE NULL
        END AS SecondaryText,
        deployment.Status,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1
                FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN platform.Id
            ELSE NULL
        END AS ParentId,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1
                FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN $10
            ELSE NULL
        END AS ParentResourceType,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1
                FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN platform.Name
            ELSE NULL
        END AS ParentName,
        candidate.MatchRank
    FROM DeploymentCandidates candidate
    JOIN Deployments deployment ON deployment.Id = candidate.Id
    LEFT JOIN Platforms platform ON platform.Id = deployment.PlatformId

    UNION ALL

    SELECT
        service.Id,
        $20 AS ResourceType,
        service.Name,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1 FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN platform.Name ELSE NULL END AS SecondaryText,
        service.Health AS Status,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1 FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN platform.Id ELSE NULL END AS ParentId,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1 FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN $10 ELSE NULL END AS ParentResourceType,
        CASE WHEN (
            $2
            OR EXISTS (
                SELECT 1 FROM AuthorizedResources parentAccess
                WHERE parentAccess.ResourceType = $10
                  AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
            THEN platform.Name ELSE NULL END AS ParentName,
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
    FROM AutomationActionCandidates candidate
    JOIN Actions action ON action.Id = candidate.Id
    LEFT JOIN ActionRuns currentRun ON currentRun.Id = action.CurrentRunId
    LEFT JOIN LATERAL (
        SELECT runs.Status
        FROM ActionRuns runs
        WHERE runs.ActionId = action.Id
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
            CASE WHEN (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $11
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentStack.Id)))
                THEN parentStack.Name
                ELSE NULL
            END,
            CASE WHEN (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $12
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentDeployment.Id)))
                THEN parentDeployment.Name
                ELSE NULL
            END,
            policy.Source ->> '$type') AS SecondaryText,
        COALESCE(currentRun.Status, latestRun.Status, CASE WHEN policy.Enabled THEN 'Enabled' ELSE 'Disabled' END) AS Status,
        CASE
            WHEN parentStack.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $11
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentStack.Id)))
                THEN parentStack.Id
            WHEN parentDeployment.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $12
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentDeployment.Id)))
                THEN parentDeployment.Id
            ELSE NULL
        END AS ParentId,
        CASE
            WHEN parentStack.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $11
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentStack.Id)))
                THEN $11
            WHEN parentDeployment.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $12
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentDeployment.Id)))
                THEN $12
            ELSE NULL
        END AS ParentResourceType,
        CASE
            WHEN parentStack.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $11
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentStack.Id)))
                THEN parentStack.Name
            WHEN parentDeployment.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $12
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentDeployment.Id)))
                THEN parentDeployment.Name
            ELSE NULL
        END AS ParentName,
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
    LEFT JOIN Deployments parentDeployment
        ON policy.Source ->> '$type' = 'Deployment'
        AND parentDeployment.Id = NULLIF(
            COALESCE(policy.Source ->> 'DeploymentId', policy.Source ->> 'deploymentId'),
            '')::uuid

    UNION ALL

    SELECT
        project.Id,
        $18 AS ResourceType,
        project.Name,
        project.Branch AS SecondaryText,
        COALESCE(currentRun.Status, latestRun.Status, CASE WHEN project.Enabled THEN 'Enabled' ELSE 'Disabled' END) AS Status,
        CASE
            WHEN project.BuilderKind = 'Platform' AND platform.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $10
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                THEN platform.Id
            WHEN project.BuilderKind = 'BuildAgentPool' AND pool.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $19
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = pool.Id)))
                THEN pool.Id
            ELSE NULL
        END AS ParentId,
        CASE
            WHEN project.BuilderKind = 'Platform' AND platform.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $10
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                THEN $10
            WHEN project.BuilderKind = 'BuildAgentPool' AND pool.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $19
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = pool.Id)))
                THEN $19
            ELSE NULL
        END AS ParentResourceType,
        CASE
            WHEN project.BuilderKind = 'Platform' AND platform.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $10
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                THEN platform.Name
            WHEN project.BuilderKind = 'BuildAgentPool' AND pool.Id IS NOT NULL AND (
                $2
                OR EXISTS (
                    SELECT 1
                    FROM AuthorizedResources parentAccess
                    WHERE parentAccess.ResourceType = $19
                      AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = pool.Id)))
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
    LEFT JOIN BuildAgentPools pool ON pool.Id = project.BuildAgentPoolId

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

using System.Data;
using Dapper;
using Domain.Contracts.Resources.Search;
using Hosting.Common;

namespace Infrastructure.Persistence;

internal sealed class GlobalSearchRepository(
    IDbConnection db,
    Func<IDbTransaction> tx) : IGlobalSearchRepository
{
    private const string Sql = "WITH " +
        AuthorizationSql.ActorScopeCte + ", " +
        AuthorizationSql.AuthorizedResourcesCte + ", " +
        """
        PlatformCandidates AS (
            SELECT
                platform.Id,
                CASE
                    WHEN lower(platform.Name) = lower(@Query) THEN 0
                    WHEN platform.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    WHEN platform.Name ILIKE @ContainsPattern ESCAPE E'\\' THEN 2
                    WHEN platform.Address ILIKE @PrefixPattern ESCAPE E'\\' THEN 3
                    ELSE 4
                END AS MatchRank
            FROM Platforms platform
            WHERE @PlatformType = ANY(@ResourceTypes)
              AND (
                  platform.Name ILIKE @ContainsPattern ESCAPE E'\\'
                  OR platform.Address ILIKE @ContainsPattern ESCAPE E'\\')
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @PlatformType
                        AND (access.ResourceId IS NULL OR access.ResourceId = platform.Id)))
            ORDER BY MatchRank, lower(platform.Name), platform.Id
            LIMIT @LimitPerType
        ),
        StackCandidates AS (
            SELECT
                stack.Id,
                CASE
                    WHEN lower(stack.Name) = lower(@Query) THEN 0
                    WHEN stack.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    ELSE 2
                END AS MatchRank
            FROM Stacks stack
            WHERE @StackType = ANY(@ResourceTypes)
              AND stack.Name ILIKE @ContainsPattern ESCAPE E'\\'
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @StackType
                        AND (access.ResourceId IS NULL OR access.ResourceId = stack.Id)))
            ORDER BY MatchRank, lower(stack.Name), stack.Id
            LIMIT @LimitPerType
        ),
        DeploymentCandidates AS (
            SELECT
                deployment.Id,
                CASE
                    WHEN lower(deployment.Name) = lower(@Query) THEN 0
                    WHEN deployment.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    ELSE 2
                END AS MatchRank
            FROM Deployments deployment
            WHERE @DeploymentType = ANY(@ResourceTypes)
              AND deployment.Name ILIKE @ContainsPattern ESCAPE E'\\'
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @DeploymentType
                        AND (access.ResourceId IS NULL OR access.ResourceId = deployment.Id)))
            ORDER BY MatchRank, lower(deployment.Name), deployment.Id
            LIMIT @LimitPerType
        ),
        SwarmServiceCandidates AS (
            SELECT
                service.Id,
                CASE
                    WHEN lower(service.Name) = lower(@Query) THEN 0
                    WHEN service.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    ELSE 2
                END AS MatchRank
            FROM SwarmServices service
            WHERE @SwarmServiceType = ANY(@ResourceTypes)
              AND service.Name ILIKE @ContainsPattern ESCAPE E'\\'
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @SwarmServiceType
                        AND (access.ResourceId IS NULL OR access.ResourceId = service.Id)))
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources platformAccess
                      WHERE platformAccess.ResourceType = @PlatformType
                        AND (platformAccess.ResourceId IS NULL OR platformAccess.ResourceId = service.PlatformId)))
            ORDER BY MatchRank, lower(service.Name), service.Id
            LIMIT @LimitPerType
        ),
        GitRepositoryCandidates AS (
            SELECT
                repository.Id,
                CASE
                    WHEN lower(repository.Name) = lower(@Query) THEN 0
                    WHEN repository.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    WHEN repository.Name ILIKE @ContainsPattern ESCAPE E'\\' THEN 2
                    WHEN repository.Url ILIKE @PrefixPattern ESCAPE E'\\' THEN 3
                    ELSE 4
                END AS MatchRank
            FROM GitRepositories repository
            WHERE @GitRepositoryType = ANY(@ResourceTypes)
              AND (
                  repository.Name ILIKE @ContainsPattern ESCAPE E'\\'
                  OR repository.Url ILIKE @ContainsPattern ESCAPE E'\\')
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @GitRepositoryType
                        AND (access.ResourceId IS NULL OR access.ResourceId = repository.Id)))
            ORDER BY MatchRank, lower(repository.Name), repository.Id
            LIMIT @LimitPerType
        ),
        RegistryCandidates AS (
            SELECT
                registry.Id,
                CASE
                    WHEN lower(registry.Name) = lower(@Query) THEN 0
                    WHEN registry.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    WHEN registry.Name ILIKE @ContainsPattern ESCAPE E'\\' THEN 2
                    WHEN registry.RegistryHost ILIKE @PrefixPattern ESCAPE E'\\' THEN 3
                    ELSE 4
                END AS MatchRank
            FROM Registries registry
            WHERE @RegistryType = ANY(@ResourceTypes)
              AND (
                  registry.Name ILIKE @ContainsPattern ESCAPE E'\\'
                  OR registry.RegistryHost ILIKE @ContainsPattern ESCAPE E'\\')
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @RegistryType
                        AND (access.ResourceId IS NULL OR access.ResourceId = registry.Id)))
            ORDER BY MatchRank, lower(registry.Name), registry.Id
            LIMIT @LimitPerType
        ),
        AutomationActionCandidates AS (
            SELECT
                action.Id,
                CASE
                    WHEN lower(action.Name) = lower(@Query) THEN 0
                    WHEN action.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    ELSE 2
                END AS MatchRank
            FROM Actions action
            WHERE @AutomationActionType = ANY(@ResourceTypes)
              AND action.Name ILIKE @ContainsPattern ESCAPE E'\\'
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @AutomationActionType
                        AND (access.ResourceId IS NULL OR access.ResourceId = action.Id)))
            ORDER BY MatchRank, lower(action.Name), action.Id
            LIMIT @LimitPerType
        ),
        BackupRepositoryCandidates AS (
            SELECT
                repository.Id,
                CASE
                    WHEN lower(repository.Name) = lower(@Query) THEN 0
                    WHEN repository.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    WHEN repository.Name ILIKE @ContainsPattern ESCAPE E'\\' THEN 2
                    WHEN repository.Type ILIKE @PrefixPattern ESCAPE E'\\' THEN 3
                    ELSE 4
                END AS MatchRank
            FROM BackupRepositories repository
            WHERE @BackupRepositoryType = ANY(@ResourceTypes)
              AND repository.ArchivedAt IS NULL
              AND (
                  repository.Name ILIKE @ContainsPattern ESCAPE E'\\'
                  OR repository.Type ILIKE @ContainsPattern ESCAPE E'\\')
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @BackupRepositoryType
                        AND (access.ResourceId IS NULL OR access.ResourceId = repository.Id)))
            ORDER BY MatchRank, lower(repository.Name), repository.Id
            LIMIT @LimitPerType
        ),
        BackupPolicyCandidates AS (
            SELECT
                policy.Id,
                CASE
                    WHEN lower(policy.Name) = lower(@Query) THEN 0
                    WHEN policy.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    ELSE 2
                END AS MatchRank
            FROM BackupPolicies policy
            WHERE @BackupPolicyType = ANY(@ResourceTypes)
              AND policy.ArchivedAt IS NULL
              AND policy.Name ILIKE @ContainsPattern ESCAPE E'\\'
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @BackupPolicyType
                        AND (access.ResourceId IS NULL OR access.ResourceId = policy.Id)))
            ORDER BY MatchRank, lower(policy.Name), policy.Id
            LIMIT @LimitPerType
        ),
        BuildCandidates AS (
            SELECT
                project.Id,
                CASE
                    WHEN lower(project.Name) = lower(@Query) THEN 0
                    WHEN project.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    WHEN project.Name ILIKE @ContainsPattern ESCAPE E'\\' THEN 2
                    WHEN project.Branch ILIKE @PrefixPattern ESCAPE E'\\' THEN 3
                    ELSE 4
                END AS MatchRank
            FROM BuildProjects project
            WHERE @BuildType = ANY(@ResourceTypes)
              AND project.ArchivedAt IS NULL
              AND (
                  project.Name ILIKE @ContainsPattern ESCAPE E'\\'
                  OR project.Branch ILIKE @ContainsPattern ESCAPE E'\\')
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @BuildType
                        AND (access.ResourceId IS NULL OR access.ResourceId = project.Id)))
            ORDER BY MatchRank, lower(project.Name), project.Id
            LIMIT @LimitPerType
        ),
        BuildAgentPoolCandidates AS (
            SELECT
                pool.Id,
                CASE
                    WHEN lower(pool.Name) = lower(@Query) THEN 0
                    WHEN pool.Name ILIKE @PrefixPattern ESCAPE E'\\' THEN 1
                    WHEN pool.Name ILIKE @ContainsPattern ESCAPE E'\\' THEN 2
                    WHEN pool.Provider ILIKE @PrefixPattern ESCAPE E'\\' THEN 3
                    ELSE 4
                END AS MatchRank
            FROM BuildAgentPools pool
            WHERE @BuildAgentPoolType = ANY(@ResourceTypes)
              AND pool.ArchivedAt IS NULL
              AND (
                  pool.Name ILIKE @ContainsPattern ESCAPE E'\\'
                  OR pool.Provider ILIKE @ContainsPattern ESCAPE E'\\')
              AND (
                  @IsAdmin
                  OR EXISTS (
                      SELECT 1
                      FROM AuthorizedResources access
                      WHERE access.ResourceType = @BuildAgentPoolType
                        AND (access.ResourceId IS NULL OR access.ResourceId = pool.Id)))
            ORDER BY MatchRank, lower(pool.Name), pool.Id
            LIMIT @LimitPerType
        ),
        Matches AS (
            SELECT
                platform.Id,
                @PlatformType AS ResourceType,
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
                @StackType AS ResourceType,
                stack.Name,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1
                        FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
                          AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                    THEN platform.Name
                    ELSE NULL
                END AS SecondaryText,
                release.Status,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1
                        FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
                          AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                    THEN platform.Id
                    ELSE NULL
                END AS ParentId,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1
                        FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
                          AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                    THEN @PlatformType
                    ELSE NULL
                END AS ParentResourceType,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1
                        FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
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
                @DeploymentType AS ResourceType,
                deployment.Name,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1
                        FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
                          AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                    THEN platform.Name
                    ELSE NULL
                END AS SecondaryText,
                deployment.Status,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1
                        FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
                          AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                    THEN platform.Id
                    ELSE NULL
                END AS ParentId,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1
                        FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
                          AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                    THEN @PlatformType
                    ELSE NULL
                END AS ParentResourceType,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1
                        FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
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
                @SwarmServiceType AS ResourceType,
                service.Name,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1 FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
                          AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                    THEN platform.Name ELSE NULL END AS SecondaryText,
                service.Health AS Status,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1 FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
                          AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                    THEN platform.Id ELSE NULL END AS ParentId,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1 FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
                          AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                    THEN @PlatformType ELSE NULL END AS ParentResourceType,
                CASE WHEN (
                    @IsAdmin
                    OR EXISTS (
                        SELECT 1 FROM AuthorizedResources parentAccess
                        WHERE parentAccess.ResourceType = @PlatformType
                          AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                    THEN platform.Name ELSE NULL END AS ParentName,
                candidate.MatchRank
            FROM SwarmServiceCandidates candidate
            JOIN SwarmServices service ON service.Id = candidate.Id
            JOIN Platforms platform ON platform.Id = service.PlatformId

            UNION ALL

            SELECT
                repository.Id,
                @GitRepositoryType AS ResourceType,
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
                @RegistryType AS ResourceType,
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
                @AutomationActionType AS ResourceType,
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
                @BackupRepositoryType AS ResourceType,
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
                @BackupPolicyType AS ResourceType,
                policy.Name,
                COALESCE(
                    CASE WHEN (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @StackType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentStack.Id)))
                        THEN parentStack.Name
                        ELSE NULL
                    END,
                    CASE WHEN (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @DeploymentType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentDeployment.Id)))
                        THEN parentDeployment.Name
                        ELSE NULL
                    END,
                    policy.Source ->> '$type') AS SecondaryText,
                COALESCE(currentRun.Status, latestRun.Status, CASE WHEN policy.Enabled THEN 'Enabled' ELSE 'Disabled' END) AS Status,
                CASE
                    WHEN parentStack.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @StackType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentStack.Id)))
                        THEN parentStack.Id
                    WHEN parentDeployment.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @DeploymentType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentDeployment.Id)))
                        THEN parentDeployment.Id
                    ELSE NULL
                END AS ParentId,
                CASE
                    WHEN parentStack.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @StackType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentStack.Id)))
                        THEN @StackType
                    WHEN parentDeployment.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @DeploymentType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentDeployment.Id)))
                        THEN @DeploymentType
                    ELSE NULL
                END AS ParentResourceType,
                CASE
                    WHEN parentStack.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @StackType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = parentStack.Id)))
                        THEN parentStack.Name
                    WHEN parentDeployment.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @DeploymentType
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
                @BuildType AS ResourceType,
                project.Name,
                project.Branch AS SecondaryText,
                COALESCE(currentRun.Status, latestRun.Status, CASE WHEN project.Enabled THEN 'Enabled' ELSE 'Disabled' END) AS Status,
                CASE
                    WHEN project.BuilderKind = 'Platform' AND platform.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @PlatformType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                        THEN platform.Id
                    WHEN project.BuilderKind = 'BuildAgentPool' AND pool.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @BuildAgentPoolType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = pool.Id)))
                        THEN pool.Id
                    ELSE NULL
                END AS ParentId,
                CASE
                    WHEN project.BuilderKind = 'Platform' AND platform.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @PlatformType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                        THEN @PlatformType
                    WHEN project.BuilderKind = 'BuildAgentPool' AND pool.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @BuildAgentPoolType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = pool.Id)))
                        THEN @BuildAgentPoolType
                    ELSE NULL
                END AS ParentResourceType,
                CASE
                    WHEN project.BuilderKind = 'Platform' AND platform.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @PlatformType
                              AND (parentAccess.ResourceId IS NULL OR parentAccess.ResourceId = platform.Id)))
                        THEN platform.Name
                    WHEN project.BuilderKind = 'BuildAgentPool' AND pool.Id IS NOT NULL AND (
                        @IsAdmin
                        OR EXISTS (
                            SELECT 1
                            FROM AuthorizedResources parentAccess
                            WHERE parentAccess.ResourceType = @BuildAgentPoolType
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
                @BuildAgentPoolType AS ResourceType,
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
        LIMIT @TotalLimit
        """;

    public async Task<IReadOnlyList<GlobalSearchMatch>> SearchAsync(
        Guid userId,
        bool isAdmin,
        IReadOnlyCollection<ResourceType> resourceTypes,
        string query,
        int limitPerType,
        int totalLimit,
        CancellationToken cancellationToken)
    {
        if (resourceTypes.Count == 0)
        {
            return [];
        }

        var escapedQuery = EscapeLikePattern(query);
        var prefixPattern = $"{escapedQuery}%";
        var containsPattern = $"%{escapedQuery}%";

        var rows = await db.QueryAsync<GlobalSearchRow>(Sql,
            new
            {
                UserId = userId,
                IsAdmin = isAdmin,
                ResourceTypes = resourceTypes.Select(static type => (int)type).ToArray(),
                Query = query,
                PrefixPattern = prefixPattern,
                ContainsPattern = containsPattern,
                LimitPerType = limitPerType,
                TotalLimit = totalLimit,
                GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(PermissionLevel.Read),
                PlatformType = (int)ResourceType.Platform,
                StackType = (int)ResourceType.Stack,
                DeploymentType = (int)ResourceType.Deployment,
                GitRepositoryType = (int)ResourceType.GitRepository,
                RegistryType = (int)ResourceType.Registry,
                AutomationActionType = (int)ResourceType.AutomationAction,
                BackupPolicyType = (int)ResourceType.BackupPolicy,
                BackupRepositoryType = (int)ResourceType.BackupRepository,
                BuildType = (int)ResourceType.Build,
                BuildAgentPoolType = (int)ResourceType.BuildAgentPool,
                SwarmServiceType = (int)ResourceType.SwarmService
            },
            transaction: tx());
        return [.. rows
            .Select(static row => new GlobalSearchMatch(
                row.Id,
                (ResourceType)row.ResourceType,
                row.Name,
                row.SecondaryText,
                row.Status,
                row.ParentId,
                row.ParentResourceType.HasValue ? (ResourceType)row.ParentResourceType.Value : null,
                row.ParentName,
                row.MatchRank))];
    }

    private static string EscapeLikePattern(string value)
        => value
            .Replace("\\", "\\\\", StringComparison.Ordinal)
            .Replace("%", "\\%", StringComparison.Ordinal)
            .Replace("_", "\\_", StringComparison.Ordinal);

    internal sealed record GlobalSearchRow(
        Guid Id,
        int ResourceType,
        string Name,
        string? SecondaryText,
        string? Status,
        Guid? ParentId,
        int? ParentResourceType,
        string? ParentName,
        int MatchRank);
}

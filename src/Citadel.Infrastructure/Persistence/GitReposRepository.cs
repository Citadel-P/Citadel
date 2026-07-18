using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class GitReposRepository(IDbConnection db, Func<IDbTransaction> tx) : IGitReposRepository
{
    public Task<bool> ExistsAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM GitRepositories WHERE Name = @Name)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name }, transaction: tx());
    }

    public async Task<int> AddAsync(GitRepository gitRepository, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null, Guid? tagCreatedByActorId = null)
    {
        string sql = ResourceTagSql.InputTagsCte + """
            inserted_repository AS (
                INSERT INTO GitRepositories (
                    Id, Name, Description, Url, DefaultBranch, Status, SyncMode, SyncIntervalMinutes, GitAccountId, CreatedAt, CreatedByActorId, Webhook, OnClone, OnPull,
                    ControlState, ControlStartedAt, ControlTriggeredBy, RowVersion)
                SELECT
                    @Id, @Name, @Description, @Url, @DefaultBranch, @Status, @SyncMode, @SyncIntervalMinutes, @GitAccountId, @CreatedAt, @CreatedByActorId, @Webhook::jsonb, @OnClone::json, @OnPull::json,
                    @ControlState, @ControlStartedAt, @ControlTriggeredBy, @RowVersion
                WHERE NOT EXISTS (SELECT 1 FROM missing_tags)
                RETURNING Id
            ),
            """ + ResourceTagSql.InsertTagsCte("inserted_repository", "r") + "\n"
            + ResourceTagSql.InsertResultSelect("inserted_repository", "inserted_repository", "inserted_tags");

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QuerySingleAsync<ResourceInsertWithTagsResult>(sql, new
        {
            Id = gitRepository.Id,
            Name = gitRepository.Name,
            Description = gitRepository.Description,
            Url = gitRepository.Url,
            DefaultBranch = gitRepository.DefaultBranch,
            Status = EnumFormatter<GitReposStatus>.GetValue(gitRepository.Status),
            SyncMode = EnumFormatter<GitRepositorySyncMode>.GetValue(gitRepository.SyncMode),
            gitRepository.SyncIntervalMinutes,
            GitAccountId = gitRepository.GitAccountId,
            CreatedAt = gitRepository.CreatedAt,
            CreatedByActorId = gitRepository.CreatedByActorId,
            Webhook = gitRepository.Webhook is null ? null : JsonSerializer.Serialize(gitRepository.Webhook, GitJsonContext.Default.RepoWebhookConfig),
            OnClone = gitRepository.OnClone is null ? null : JsonSerializer.Serialize(gitRepository.OnClone, GitJsonContext.Default.RepoCommand),
            OnPull = gitRepository.OnPull is null ? null : JsonSerializer.Serialize(gitRepository.OnPull, GitJsonContext.Default.RepoCommand),
            ControlState = EnumFormatter<ResourceControlState>.GetValue(gitRepository.ControlState),
            ControlStartedAt = gitRepository.ControlStartedAt,
            ControlTriggeredBy = gitRepository.ControlTriggeredBy,
            RowVersion = gitRepository.RowVersion,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.GitRepository),
            TagIds = tagIdArray,
            TagCreatedByActorId = tagCreatedByActorId ?? gitRepository.CreatedByActorId
        }, transaction: tx());

        gitRepository.AssignTags(result.TagsJson.ToTagSummaries());
        return result.AffectedRows;
    }

    public async Task<GitRepository?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        string sql = $$"""
            SELECT 
                g.*,
                ei.Info AS ActivityEvent_ActivityEventInfo,
                ei.EventType AS ActivityEvent_EventType,
                ei.Status AS ActivityEvent_Status,
                ei.Id AS ActivityEvent_Id,
                ei.CreatedAt AS ActivityEvent_CreatedAt,
                {{ResourceTagSql.TagAggregate("g")}}
            FROM GitRepositories g
            LEFT JOIN LATERAL (
                SELECT e.Id, e.EventType, e.Status, e.Info, e.CreatedAt
                FROM ActivityEvents e
                WHERE e.ResourceId = g.Id 
                  AND e.ResourceType = 'GitRepository'
                  AND e.EventType NOT IN ('GitRepoUpdated', 'GitRepoRenamed')
                ORDER BY e.CreatedAt DESC
                LIMIT 1
            ) ei ON TRUE
            WHERE g.Id = @Id LIMIT 1
            
            """;
        var result = await db.QuerySingleOrDefaultAsync<GitRepositoryDto>(sql, new
        {
            Id = id,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.GitRepository)
        }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<GitRepository?> GetWithAccountAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT 
                r.*,
                a.Name AS GitAccount_Name,
                a.Domain AS GitAccount_Domain,
                a.Transport AS GitAccount_Transport,
                a.AuthType AS GitAccount_AuthType,
                a.Configuration AS GitAccount_Configuration
            FROM GitRepositories r 
            LEFT JOIN GitAccounts a
                ON r.GitAccountId = a.Id
            WHERE r.Id = @Id
            LIMIT 1
            """;
            
        var result = await db.QuerySingleOrDefaultAsync<GitRepositoryDto>(sql, new { Id = id }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<GitRepository>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitRepositories gr WHERE gr.Id = ANY(@Ids)";
        var result = await db.QueryAsync<GitRepositoryDto>(sql, new { Ids = ids.ToArray() }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<GitRepository?> GetByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitRepositories WHERE Name = @Name LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<GitRepositoryDto>(sql, new { Name = name }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM GitRepositories WHERE Name = @Name AND Id != @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id }, transaction: tx());
    }

    public async Task<IEnumerable<GitRepository>> GetAllAsync(CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null)
    {
        string sql = $$"""
            SELECT gr.*,
                {{ResourceTagSql.TagAggregate("gr")}}
            FROM GitRepositories gr
            WHERE {{ResourceTagSql.FilterPredicate("gr")}}
            ORDER BY gr.CreatedAt DESC
            """;
        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<GitRepositoryDto>(sql, new
        {
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.GitRepository),
            TagIds = tagIdArray,
            TagIdsLength = tagIdArray.Length
        }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<GitRepository>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken, IReadOnlyCollection<Guid>? tagIds = null)
    {
        string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte
            + $$"""
             SELECT gr.*,
                {{ResourceTagSql.TagAggregate("gr")}}
            FROM GitRepositories gr WHERE 
            """
            + AuthorizationSql.ResourcePredicatePrefix + "gr.Id" + AuthorizationSql.ResourcePredicateSuffix
            + " AND " + ResourceTagSql.FilterPredicate("gr")
            + " ORDER BY gr.CreatedAt DESC;";

        var tagIdArray = ResourceTagSql.NormalizeTagIds(tagIds);
        var result = await db.QueryAsync<GitRepositoryDto>(sql, new
        {
            UserId = userId,
            ResourceType = (int)resourceType,
            GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
            SpecificPermission = (int)specificPermission,
            TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.GitRepository),
            TagIds = tagIdArray,
            TagIdsLength = tagIdArray.Length
        }, transaction: tx());

        return result.ToDomain();
    }

    public Task<int> UpdateAsync(GitRepository gitRepository, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE GitRepositories
            SET Name = @Name,
                Description = @Description,
                Url = @Url,
                DefaultBranch = @DefaultBranch,
                Status = @Status,
                SyncMode = @SyncMode,
                SyncIntervalMinutes = @SyncIntervalMinutes,
                GitAccountId = @GitAccountId,
                Webhook = @Webhook::jsonb,
                OnClone = @OnClone,
                OnPull = @OnPull,
                ControlState = @ControlState,
                ControlStartedAt = @ControlStartedAt,
                ControlTriggeredBy = @ControlTriggeredBy,
                RowVersion = @RowVersion
            WHERE Id = @Id
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = gitRepository.Id,
            Name = gitRepository.Name,
            Description = gitRepository.Description,
            Url = gitRepository.Url,
            DefaultBranch = gitRepository.DefaultBranch,
            Status = EnumFormatter<GitReposStatus>.GetValue(gitRepository.Status),
            SyncMode = EnumFormatter<GitRepositorySyncMode>.GetValue(gitRepository.SyncMode),
            gitRepository.SyncIntervalMinutes,
            GitAccountId = gitRepository.GitAccountId,
            Webhook = gitRepository.Webhook is null ? null : JsonSerializer.Serialize(gitRepository.Webhook, GitJsonContext.Default.RepoWebhookConfig),
            OnClone = gitRepository.OnClone is null ? null : JsonSerializer.Serialize(gitRepository.OnClone, GitJsonContext.Default.RepoCommand),
            OnPull = gitRepository.OnPull is null ? null : JsonSerializer.Serialize(gitRepository.OnPull, GitJsonContext.Default.RepoCommand),
            ControlState = EnumFormatter<ResourceControlState>.GetValue(gitRepository.ControlState),
            ControlStartedAt = gitRepository.ControlStartedAt,
            ControlTriggeredBy = gitRepository.ControlTriggeredBy,
            RowVersion = gitRepository.RowVersion
        }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM GitRepositories
            WHERE Id = ANY(@Ids)
        """;

        return db.ExecuteAsync(
            sql,
            new { Ids = ids.ToArray() },
            transaction: tx());
    }

    public async Task<IEnumerable<GitRepository>> GetStuckRepositoriesAsync(int staleAfterSeconds = 3600, CancellationToken cancellationToken = default)
    {
        string sql = $$"""
            SELECT gr.*,
                {{ResourceTagSql.TagAggregate("gr")}}
            FROM GitRepositories gr
            WHERE gr.ControlState = @ControlState
              AND gr.ControlStartedAt IS NOT NULL
              AND gr.ControlStartedAt < @ControlStartedAt
            ORDER BY gr.ControlStartedAt ASC
            """;

        var result = await db.QueryAsync<GitRepositoryDto>(
            sql,
            new
            {
                ControlState = EnumFormatter<ResourceControlState>.GetValue(ResourceControlState.Processing),
                ControlStartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds() - staleAfterSeconds,
                TagResourceType = ResourceTagSql.GetResourceTypeValue(TaggableResourceType.GitRepository)
            },
            transaction: tx());

        return result.ToDomain();
    }

    public Task<int> UpdateProcessingAsync(
        Guid id,
        GitReposStatus status,
        ResourceControlState state,
        long? startedAt,
        long rowVersion,
        bool checkRowVersion,
        Guid? controlTriggeredBy,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE GitRepositories
            SET Status = @Status,
                ControlState = @State,
                ControlStartedAt = @StartedAt,
                ControlTriggeredBy = @ControlTriggeredBy,
                RowVersion = RowVersion + 1
            WHERE Id = @Id
              AND (@CheckRowVersion = false OR RowVersion = @RowVersion)
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                Id = id,
                Status = EnumFormatter<GitReposStatus>.GetValue(status),
                State = EnumFormatter<ResourceControlState>.GetValue(state),
                StartedAt = startedAt,
                RowVersion = rowVersion,
                CheckRowVersion = checkRowVersion,
                ControlTriggeredBy = controlTriggeredBy
            },
            transaction: tx());
    }

    public async Task<GitRepositoryRef?> GetRefAsync(Guid gitRepositoryId, string branch, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM GitRepositoryRefs
            WHERE GitRepositoryId = @GitRepositoryId
              AND Branch = @Branch
            LIMIT 1
            """;

        var result = await db.QuerySingleOrDefaultAsync<GitRepositoryRefDto>(
            sql,
            new { GitRepositoryId = gitRepositoryId, Branch = branch },
            transaction: tx());

        return result?.ToDomain();
    }

    public async Task<IEnumerable<GitRepositoryRef>> GetRefsByRepositoryIdAsync(Guid gitRepositoryId, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT *
            FROM GitRepositoryRefs
            WHERE GitRepositoryId = @GitRepositoryId
            ORDER BY Branch
            """;

        var result = await db.QueryAsync<GitRepositoryRefDto>(
            sql,
            new { GitRepositoryId = gitRepositoryId },
            transaction: tx());

        return result.ToDomain();
    }

    public Task<int> UpsertRefAsync(GitRepositoryRef gitRepositoryRef, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO GitRepositoryRefs (
                Id, GitRepositoryId, Branch, ResolvedCommitSha, Status, LastError, LastSyncedAt
            )
            VALUES (
                @Id, @GitRepositoryId, @Branch, @ResolvedCommitSha, @Status, @LastError, @LastSyncedAt
            )
            ON CONFLICT (GitRepositoryId, Branch) DO UPDATE
            SET ResolvedCommitSha = EXCLUDED.ResolvedCommitSha,
                Status = EXCLUDED.Status,
                LastError = EXCLUDED.LastError,
                LastSyncedAt = EXCLUDED.LastSyncedAt
            """;

        return db.ExecuteAsync(sql, new
        {
            gitRepositoryRef.Id,
            gitRepositoryRef.GitRepositoryId,
            gitRepositoryRef.Branch,
            gitRepositoryRef.ResolvedCommitSha,
            Status = EnumFormatter<GitReposStatus>.GetValue(gitRepositoryRef.Status),
            gitRepositoryRef.LastError,
            gitRepositoryRef.LastSyncedAt
        }, transaction: tx());
    }
}

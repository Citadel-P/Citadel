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
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(GitRepository gitRepository, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO GitRepositories (
                Id, Name, Description, Url, DefaultBranch, Status, GitAccountId, CreatedAt, CreatedByActorId, WebHookEnabled, WebHookSecret, OnClone, OnPull)
            VALUES (
                @Id, @Name, @Description, @Url, @DefaultBranch, @Status, @GitAccountId, @CreatedAt, @CreatedByActorId, @WebHookEnabled, @WebHookSecret, @OnClone::json, @OnPull::json)
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = gitRepository.Id,
            Name = gitRepository.Name,
            Description = gitRepository.Description,
            Url = gitRepository.Url,
            DefaultBranch = gitRepository.DefaultBranch,
            Status = EnumFormatter<GitReposStatus>.GetValue(gitRepository.Status),
            GitAccountId = gitRepository.GitAccountId,
            CreatedAt = gitRepository.CreatedAt,
            CreatedByActorId = gitRepository.CreatedByActorId,
            WebHookEnabled = gitRepository.WebHookEnabled,
            WebHookSecret = gitRepository.WebHookSecret,
            OnClone = gitRepository.OnClone is null ? null : JsonSerializer.Serialize(gitRepository.OnClone, GitJsonContext.Default.RepoCommand),
            OnPull = gitRepository.OnPull is null ? null : JsonSerializer.Serialize(gitRepository.OnPull, GitJsonContext.Default.RepoCommand),
            ControlState = EnumFormatter<ResourceControlState>.GetValue(gitRepository.ControlState),
            ControlStartedAt = gitRepository.ControlStartedAt,
            ControlTriggeredBy = gitRepository.ControlTriggeredBy,
            RowVersion = gitRepository.RowVersion
        }, transaction: tx());
    }

    public async Task<GitRepository?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitRepositories WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<GitRepositoryDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
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
            WHERE r.Id = @Id LIMIT 1
            """;
            
        var result = await db.QuerySingleOrDefaultAsync<GitRepositoryDto>(sql, new { Id = id, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<GitRepository>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitRepositories gr WHERE gr.Id = ANY(@Ids)";
        var result = await db.QueryAsync<GitRepositoryDto>(sql, new { Ids = ids.ToArray(), cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<GitRepository?> GetByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitRepositories WHERE Name = @Name LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<GitRepositoryDto>(sql, new { Name = name, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<bool> ExistsAsync(Guid id, string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM GitRepositories WHERE Name = @Name AND Id != @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id, cancellationToken }, transaction: tx());
    }

    public async Task<IEnumerable<GitRepository>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitRepositories";
        var result = await db.QueryAsync<GitRepositoryDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<GitRepository>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " SELECT * FROM GitRepositories gr WHERE "
            + AuthorizationSql.ResourcePredicatePrefix + "gr.Id" + AuthorizationSql.ResourcePredicateSuffix
            + " ORDER BY gr.CreatedAt DESC;";

        var result = await db.QueryAsync<GitRepositoryDto>(sql, new
        {
            UserId = userId,
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(action),
            cancellationToken
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
                GitAccountId = @GitAccountId,
                WebHookEnabled = @WebHookEnabled,
                WebHookSecret = @WebHookSecret,
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
            GitAccountId = gitRepository.GitAccountId,
            WebHookEnabled = gitRepository.WebHookEnabled,
            WebHookSecret = gitRepository.WebHookSecret,
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
}

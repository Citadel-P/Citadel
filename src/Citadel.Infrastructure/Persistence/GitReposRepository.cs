using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;
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
                @Id, @Name, @Description, @Url, @DefaultBranch, @Status, @GitAccountId, @CreatedAt, @CreatedByActorId, @WebHookEnabled, @WebHookSecret, @OnClone, @OnPull)
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = gitRepository.Id.Format(),
            Name = gitRepository.Name,
            Description = gitRepository.Description,
            Url = gitRepository.Url,
            DefaultBranch = gitRepository.DefaultBranch,
            Status = EnumFormatter<GitReposStatus>.GetValue(gitRepository.Status),
            GitAccountId = gitRepository.GitAccountId?.Format(),
            CreatedAt = gitRepository.CreatedAt.ToString(),
            CreatedByActorId = gitRepository.CreatedByActorId.Format(),
            WebHookEnabled = gitRepository.WebHookEnabled ? 1 : 0,
            WebHookSecret = gitRepository.WebHookSecret,
            OnClone = gitRepository.OnClone is null ? null : JsonSerializer.Serialize(gitRepository.OnClone, GitJsonContext.Default.RepoCommand),
            OnPull = gitRepository.OnPull is null ? null : JsonSerializer.Serialize(gitRepository.OnPull, GitJsonContext.Default.RepoCommand),
            ControlState = EnumFormatter<ResourceControlState>.GetValue(gitRepository.ControlState),
            ControlStartedAt = gitRepository.ControlStartedAt,
            ControlTriggeredBy = gitRepository.ControlTriggeredBy?.Format(),
            RowVersion = gitRepository.RowVersion
        }, transaction: tx());
    }

    public async Task<GitRepository?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitRepositories WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<GitRepositoryDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<GitRepository?> GetWithAccountAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = """
            
            SELECT 
                r.*,
                a.GitAccount_Id,
                a.GitAccount_Name,
                a.GitAccount_Domain,
                a.GitAccount_Transport,
                a.GitAccount_AuthType,
                a.GitAccount_Configuration
            FROM GitRepositories r 
            LEFT JOIN GitAccounts a
            ON r.GitAccountId = a.Id
            WHERE Id = @Id LIMIT 1
            """;
            
        var result = await db.QuerySingleOrDefaultAsync<GitRepositoryDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<GitRepository>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitRepositories gr WHERE gr.Id IN (SELECT value FROM json_each(@Ids))";
        var result = await db.QueryAsync<GitRepositoryDto>(sql, new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid), cancellationToken }, transaction: tx());
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
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id.Format(), cancellationToken }, transaction: tx());
    }

    public async Task<IEnumerable<GitRepository>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitRepositories";
        var result = await db.QueryAsync<GitRepositoryDto>(sql, transaction: tx());
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
            Id = gitRepository.Id.Format(),
            Name = gitRepository.Name,
            Description = gitRepository.Description,
            Url = gitRepository.Url,
            DefaultBranch = gitRepository.DefaultBranch,
            Status = EnumFormatter<GitReposStatus>.GetValue(gitRepository.Status),
            GitAccountId = gitRepository.GitAccountId?.Format(),
            WebHookEnabled = gitRepository.WebHookEnabled ? 1 : 0,
            WebHookSecret = gitRepository.WebHookSecret,
            OnClone = gitRepository.OnClone is null ? null : JsonSerializer.Serialize(gitRepository.OnClone, GitJsonContext.Default.RepoCommand),
            OnPull = gitRepository.OnPull is null ? null : JsonSerializer.Serialize(gitRepository.OnPull, GitJsonContext.Default.RepoCommand),
            ControlState = EnumFormatter<ResourceControlState>.GetValue(gitRepository.ControlState),
            ControlStartedAt = gitRepository.ControlStartedAt,
            ControlTriggeredBy = gitRepository.ControlTriggeredBy?.Format(),
            RowVersion = gitRepository.RowVersion
        }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM GitRepositories
            WHERE Id IN (
                SELECT value FROM json_each(@Ids)
            )
        """;

        return db.ExecuteAsync(
            sql,
            new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid) },
            transaction: tx());
    }
}

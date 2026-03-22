using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class GitRepositoryRepository(IDbConnection db, Func<IDbTransaction> tx) : IGitRepositoryRepository
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
                Id, Name, Url, DefaultBranch, GitAccountId, CreatedAt, CreatedByActorId)
            VALUES (
                @Id, @Name, @Url, @DefaultBranch, @GitAccountId, @CreatedAt, @CreatedByActorId)
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = gitRepository.Id.Format(),
            Name = gitRepository.Name,
            Url = gitRepository.Url,
            DefaultBranch = gitRepository.DefaultBranch,
            GitAccountId = gitRepository.GitAccountId?.Format(),
            CreatedAt = gitRepository.CreatedAt.ToString(),
            CreatedByActorId = gitRepository.CreatedByActorId.Format()
        }, transaction: tx());
    }

    public async Task<GitRepository?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitRepositories WHERE Id = @Id LIMIT 1";
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
                Url = @Url,
                DefaultBranch = @DefaultBranch,
                GitAccountId = @GitAccountId
            WHERE Id = @Id
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = gitRepository.Id.Format(),
            Name = gitRepository.Name,
            Url = gitRepository.Url,
            DefaultBranch = gitRepository.DefaultBranch,
            GitAccountId = gitRepository.GitAccountId?.Format()
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

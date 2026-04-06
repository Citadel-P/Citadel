using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using Infrastructure.TypeHandlers;
using System.Data;
using System.Text.Json;
using static Infrastructure.TypeHandlers.FormattingExtensions;

namespace Infrastructure.Persistence;

internal sealed class GitAccountRepository(IDbConnection db, Func<IDbTransaction> tx) : IGitAccountRepository
{
    public Task<bool> ExistsAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM GitAccounts WHERE Name = @Name)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, cancellationToken }, transaction: tx());
    }

    public Task<int> AddAsync(GitAccount gitAccount, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO GitAccounts (
                Id, Name, Domain, Transport, AuthType, CreatedAt, CreatedByActorId, Configuration)
            VALUES (
                @Id, @Name, @Domain, @Transport, @AuthType, @CreatedAt, @CreatedByActorId, @Configuration)
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = gitAccount.Id.Format(),
            Name = gitAccount.Name,
            Domain = gitAccount.Domain,
            Transport = EnumFormatter<GitTransport>.GetValue(gitAccount.Transport),
            AuthType = EnumFormatter<GitAuthType>.GetValue(gitAccount.AuthType),
            CreatedAt = gitAccount.CreatedAt.ToString(),
            CreatedByActorId = gitAccount.CreatedByActorId.Format(),
            Configuration = JsonSerializer.Serialize(gitAccount.Configuration, typeof(GitAuthConfiguration), GitJsonContext.Default)
        }, transaction: tx());
    }

    public async Task<GitAccount?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitAccounts WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<GitAccountDto>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public async Task<IEnumerable<GitAccount>?> GetAllAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitAccounts ga WHERE ga.Id IN (SELECT value FROM json_each(@Ids))";
        var result = await db.QueryAsync<GitAccountDto>(sql, new { Ids = JsonSerializer.Serialize(ids, DeploymentJsonContext.Default.IEnumerableGuid), cancellationToken }, transaction: tx());
        return result.ToDomain();
    }

    public async Task<GitAccount?> GetByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitAccounts WHERE Name = @Name LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<GitAccountDto>(sql, new { Name = name, cancellationToken }, transaction: tx());
        return result?.ToDomain();
    }

    public Task<bool> IsNameTakenAsync(Guid id, string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM GitAccounts WHERE Name = @Name AND Id != @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id.Format(), cancellationToken }, transaction: tx());
    }

    public Task<bool> ExistsAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM GitAccounts WHERE Id = @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Id = id.Format(), cancellationToken }, transaction: tx());
    }

    public async Task<IEnumerable<GitAccount>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM GitAccounts";
        var result = await db.QueryAsync<GitAccountDto>(sql, transaction: tx());
        return result.ToDomain();
    }

    public async Task<IEnumerable<GitAccount>> GetAuthorizedAsync(Guid userId, ResourceType resourceType, ResourceAction action, CancellationToken cancellationToken)
    {
        const string sql = "WITH " + AuthorizationSql.ActorScopeCte + ", " + AuthorizationSql.GlobalAccessCte + " SELECT * FROM GitAccounts ga WHERE "
            + AuthorizationSql.ResourcePredicatePrefix + "ga.Id" + AuthorizationSql.ResourcePredicateSuffix
            + " ORDER BY ga.CreatedAt DESC;";

        var result = await db.QueryAsync<GitAccountDto>(sql, new
        {
            UserId = userId.Format(),
            ResourceType = EnumFormatter<ResourceType>.GetValue(resourceType),
            Action = EnumFormatter<ResourceAction>.GetValue(action),
            cancellationToken
        }, transaction: tx());

        return result.ToDomain();
    }

    public Task<int> UpdateAsync(GitAccount gitAccount, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE GitAccounts
            SET Name = @Name,
                Domain = @Domain,
                Transport = @Transport,
                AuthType = @AuthType,
                Configuration = @Configuration
            WHERE Id = @Id
        """;

        return db.ExecuteAsync(sql, new
        {
            Id = gitAccount.Id.Format(),
            Name = gitAccount.Name,
            Domain = gitAccount.Domain,
            Transport = EnumFormatter<GitTransport>.GetValue(gitAccount.Transport),
            AuthType = EnumFormatter<GitAuthType>.GetValue(gitAccount.AuthType),
            Configuration = JsonSerializer.Serialize(gitAccount.Configuration, typeof(GitAuthConfiguration), GitJsonContext.Default)
        }, transaction: tx());
    }

    public Task<int> RemoveRangeAsync(IEnumerable<Guid> ids, CancellationToken cancellationToken)
    {
        const string sql = """
            DELETE FROM GitAccounts
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

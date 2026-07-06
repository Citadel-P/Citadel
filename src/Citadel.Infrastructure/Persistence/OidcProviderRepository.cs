using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Oidc;
using Hosting.Common;
using Infrastructure.Persistence.Dtos;
using Infrastructure.Persistence.Mappers;
using System.Data;

namespace Infrastructure.Persistence;

internal sealed class OidcProviderRepository(IDbConnection db, Func<IDbTransaction> tx) : IOidcProviderRepository
{
    public Task<int> AddAsync(OidcProvider provider, CancellationToken cancellationToken)
    {
        provider.Validate();

        const string sql = """
            INSERT INTO OidcProviders (
                Id, Name, Description, DisplayName, Issuer, ClientId, ClientSecretCiphertext, Scopes,
                Enabled, AutoProvisionUsers, AllowEmailAutoLink, RequireEmailVerified,
                AllowedEmailDomains, RequiredClaimName, RequiredClaimValues, DefaultRoleId,
                CreatedByActorId, CreatedAt, UpdatedAt)
            VALUES (
                @Id, @Name, @Description, @DisplayName, @Issuer, @ClientId, @ClientSecretCiphertext, @Scopes,
                @Enabled, @AutoProvisionUsers, @AllowEmailAutoLink, @RequireEmailVerified,
                @AllowedEmailDomains, @RequiredClaimName, @RequiredClaimValues, @DefaultRoleId,
                @CreatedByActorId, @CreatedAt, @UpdatedAt)
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                provider.Id,
                provider.Name,
                provider.Description,
                provider.DisplayName,
                provider.Issuer,
                provider.ClientId,
                provider.ClientSecretCiphertext,
                provider.Scopes,
                provider.Enabled,
                provider.AutoProvisionUsers,
                provider.AllowEmailAutoLink,
                provider.RequireEmailVerified,
                provider.AllowedEmailDomains,
                provider.RequiredClaimName,
                provider.RequiredClaimValues,
                provider.DefaultRoleId,
                provider.CreatedByActorId,
                provider.CreatedAt,
                provider.UpdatedAt
            },
            transaction: tx());
    }

    public Task<int> UpdateAsync(OidcProvider provider, CancellationToken cancellationToken)
    {
        provider.Validate();

        const string sql = """
            UPDATE OidcProviders
            SET Name = @Name,
                Description = @Description,
                DisplayName = @DisplayName,
                Issuer = @Issuer,
                ClientId = @ClientId,
                ClientSecretCiphertext = @ClientSecretCiphertext,
                Scopes = @Scopes,
                Enabled = @Enabled,
                AutoProvisionUsers = @AutoProvisionUsers,
                AllowEmailAutoLink = @AllowEmailAutoLink,
                RequireEmailVerified = @RequireEmailVerified,
                AllowedEmailDomains = @AllowedEmailDomains,
                RequiredClaimName = @RequiredClaimName,
                RequiredClaimValues = @RequiredClaimValues,
                DefaultRoleId = @DefaultRoleId,
                UpdatedAt = @UpdatedAt
            WHERE Id = @Id
        """;

        return db.ExecuteAsync(
            sql,
            new
            {
                provider.Id,
                provider.Name,
                provider.Description,
                provider.DisplayName,
                provider.Issuer,
                provider.ClientId,
                provider.ClientSecretCiphertext,
                provider.Scopes,
                provider.Enabled,
                provider.AutoProvisionUsers,
                provider.AllowEmailAutoLink,
                provider.RequireEmailVerified,
                provider.AllowedEmailDomains,
                provider.RequiredClaimName,
                provider.RequiredClaimValues,
                provider.DefaultRoleId,
                provider.UpdatedAt
            },
            transaction: tx());
    }

    public Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM OidcProviders WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = id }, transaction: tx());
    }

    public async Task<OidcProvider?> GetAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM OidcProviders WHERE Id = @Id LIMIT 1";
        var result = await db.QuerySingleOrDefaultAsync<OidcProviderDto>(
            sql,
            new { Id = id },
            transaction: tx());

        return result?.ToDomain();
    }

    public async Task<IEnumerable<OidcProvider>> GetAllAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM OidcProviders ORDER BY DisplayName ASC";
        var result = await db.QueryAsync<OidcProviderDto>(sql, transaction: tx());
        return result.Select(static item => item.ToDomain());
    }

    public async Task<IEnumerable<OidcProvider>> GetAuthorizedAsync(
        Guid userId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        const string sql = $$"""
            WITH {{AuthorizationSql.ActorScopeCte}}, {{AuthorizationSql.GlobalAccessCte}}
            SELECT op.*
            FROM OidcProviders op
            WHERE {{AuthorizationSql.ResourcePredicatePrefix}}op.Id{{AuthorizationSql.ResourcePredicateSuffix}}
            ORDER BY op.DisplayName ASC
            """;

        var result = await db.QueryAsync<OidcProviderDto>(
            sql,
            new
            {
                UserId = userId,
                ResourceType = (int)resourceType,
                GrantedPermissionMask = UserRepository.GetGrantedPermissionMask(permissionLevel),
                SpecificPermission = (int)specificPermission
            },
            transaction: tx());

        return result.Select(static item => item.ToDomain());
    }

    public async Task<IEnumerable<OidcProvider>> GetEnabledAsync(CancellationToken cancellationToken)
    {
        const string sql = "SELECT * FROM OidcProviders WHERE Enabled ORDER BY DisplayName ASC";
        var result = await db.QueryAsync<OidcProviderDto>(sql, transaction: tx());
        return result.Select(static item => item.ToDomain());
    }

    public Task<bool> ExistsByNameAsync(string name, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM OidcProviders WHERE lower(Name) = lower(@Name))";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name }, transaction: tx());
    }

    public Task<bool> ExistsByNameExceptAsync(string name, Guid id, CancellationToken cancellationToken)
    {
        const string sql = "SELECT EXISTS (SELECT 1 FROM OidcProviders WHERE lower(Name) = lower(@Name) AND Id <> @Id)";
        return db.ExecuteScalarAsync<bool>(sql, new { Name = name, Id = id }, transaction: tx());
    }
}

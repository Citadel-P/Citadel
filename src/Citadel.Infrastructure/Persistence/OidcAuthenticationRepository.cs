using System.Data;
using Dapper;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Oidc;

namespace Infrastructure.Persistence;

internal sealed class OidcLoginStateRepository(IDbConnection db, Func<IDbTransaction> tx) : IOidcLoginStateRepository
{
    public Task<int> AddAsync(OidcLoginState state, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO OidcLoginStates (
                Id, ProviderId, StateHash, Nonce, CodeVerifier, ReturnUrl, CreatedAt, ExpiresAt)
            VALUES (
                @Id, @ProviderId, @StateHash, @Nonce, @CodeVerifier, @ReturnUrl, @CreatedAt, @ExpiresAt)
        """;

        return db.ExecuteAsync(sql, state, transaction: tx());
    }

    public Task<OidcLoginState?> GetByStateHashAsync(string stateHash, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, ProviderId, StateHash, Nonce, CodeVerifier, ReturnUrl, CreatedAt, ExpiresAt
            FROM OidcLoginStates
            WHERE StateHash = @StateHash
            LIMIT 1
        """;

        return db.QuerySingleOrDefaultAsync<OidcLoginState>(
            sql,
            new { StateHash = stateHash },
            transaction: tx());
    }

    public Task<int> DeleteAsync(Guid id, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM OidcLoginStates WHERE Id = @Id";
        return db.ExecuteAsync(sql, new { Id = id }, transaction: tx());
    }

    public Task<int> DeleteExpiredAsync(DateTime now, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM OidcLoginStates WHERE ExpiresAt <= @Now";
        return db.ExecuteAsync(sql, new { Now = now }, transaction: tx());
    }
}

internal sealed class OidcExternalLoginRepository(IDbConnection db, Func<IDbTransaction> tx) : IOidcExternalLoginRepository
{
    public Task<OidcExternalLogin?> GetAsync(Guid providerId, string subject, CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT Id, ProviderId, Subject, UserId, Email, CreatedAt, UpdatedAt
            FROM OidcExternalLogins
            WHERE ProviderId = @ProviderId AND Subject = @Subject
            LIMIT 1
        """;

        return db.QuerySingleOrDefaultAsync<OidcExternalLogin>(
            sql,
            new { ProviderId = providerId, Subject = subject },
            transaction: tx());
    }

    public Task<int> AddAsync(OidcExternalLogin externalLogin, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO OidcExternalLogins (
                Id, ProviderId, Subject, UserId, Email, CreatedAt, UpdatedAt)
            VALUES (
                @Id, @ProviderId, @Subject, @UserId, @Email, @CreatedAt, @UpdatedAt)
        """;

        return db.ExecuteAsync(sql, externalLogin, transaction: tx());
    }

    public Task<int> UpdateSeenAsync(Guid id, string? email, DateTime updatedAt, CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE OidcExternalLogins
            SET Email = @Email,
                UpdatedAt = @UpdatedAt
            WHERE Id = @Id
        """;

        return db.ExecuteAsync(sql, new { Id = id, Email = email, UpdatedAt = updatedAt }, transaction: tx());
    }

    public Task<int> DeleteByProviderIdAsync(Guid providerId, CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM OidcExternalLogins WHERE ProviderId = @ProviderId";
        return db.ExecuteAsync(sql, new { ProviderId = providerId }, transaction: tx());
    }
}

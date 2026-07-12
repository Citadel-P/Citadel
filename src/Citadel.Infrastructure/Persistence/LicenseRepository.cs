using Dapper;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Licensing;
using System.Data;

namespace Infrastructure.Persistence;

internal sealed class InstanceIdentityRepository(IDbConnection db, Func<IDbTransaction> tx) : IInstanceIdentityRepository
{
    public async Task<CitadelInstanceIdentity> GetOrCreateAsync(
        Guid candidateInstanceId,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO citadelinstanceidentity (id, instanceid, createdat)
            VALUES (1, @InstanceId, @CreatedAt)
            ON CONFLICT (id) DO NOTHING;

            SELECT instanceid AS InstanceId, createdat AS CreatedAt
            FROM citadelinstanceidentity
            WHERE id = 1
            LIMIT 1;
            """;

        var dto = await db.QuerySingleAsync<CitadelInstanceIdentityDto>(
            sql,
            new { InstanceId = candidateInstanceId, CreatedAt = createdAt.UtcDateTime },
            transaction: tx());

        return dto.ToDomain();
    }

    public async Task<CitadelInstanceIdentity?> GetAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT instanceid AS InstanceId, createdat AS CreatedAt
            FROM citadelinstanceidentity
            WHERE id = 1
            LIMIT 1
            """;

        var dto = await db.QuerySingleOrDefaultAsync<CitadelInstanceIdentityDto>(sql, transaction: tx());
        return dto?.ToDomain();
    }

    public async Task<CitadelInstanceIdentity> GetOrCreateLockedAsync(
        Guid candidateInstanceId,
        DateTimeOffset createdAt,
        CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO citadelinstanceidentity (id, instanceid, createdat)
            VALUES (1, @InstanceId, @CreatedAt)
            ON CONFLICT (id) DO NOTHING;

            SELECT instanceid AS InstanceId, createdat AS CreatedAt
            FROM citadelinstanceidentity
            WHERE id = 1
            FOR UPDATE;
            """;

        var dto = await db.QuerySingleAsync<CitadelInstanceIdentityDto>(
            sql,
            new { InstanceId = candidateInstanceId, CreatedAt = createdAt.UtcDateTime },
            transaction: tx());

        return dto.ToDomain();
    }
}

internal sealed class InstalledLicenseRepository(IDbConnection db, Func<IDbTransaction> tx) : IInstalledLicenseRepository
{
    public async Task<InstalledLicense?> GetAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT rawlicense AS RawLicense, fingerprint AS Fingerprint, installedat AS InstalledAt,
                   installedbyactorid AS InstalledByActorId, lastvalidatedat AS LastValidatedAt,
                   lastvalidationstatus AS LastValidationStatus, lastvalidationerrorcode AS LastValidationErrorCode
            FROM installedlicenses
            WHERE id = 1
            LIMIT 1
            """;

        var dto = await db.QuerySingleOrDefaultAsync<InstalledLicenseDto>(sql, transaction: tx());
        return dto?.ToDomain();
    }

    public Task<int> UpsertAsync(InstalledLicense license, CancellationToken cancellationToken)
    {
        const string sql = """
            INSERT INTO installedlicenses (
                id, rawlicense, fingerprint, installedat, installedbyactorid,
                lastvalidatedat, lastvalidationstatus, lastvalidationerrorcode)
            VALUES (
                1, @RawLicense, @Fingerprint, @InstalledAt, @InstalledByActorId,
                @LastValidatedAt, @LastValidationStatus, @LastValidationErrorCode)
            ON CONFLICT (id) DO UPDATE SET
                rawlicense = EXCLUDED.rawlicense,
                fingerprint = EXCLUDED.fingerprint,
                installedat = EXCLUDED.installedat,
                installedbyactorid = EXCLUDED.installedbyactorid,
                lastvalidatedat = EXCLUDED.lastvalidatedat,
                lastvalidationstatus = EXCLUDED.lastvalidationstatus,
                lastvalidationerrorcode = EXCLUDED.lastvalidationerrorcode
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                license.RawLicense,
                license.Fingerprint,
                InstalledAt = license.InstalledAt.UtcDateTime,
                license.InstalledByActorId,
                LastValidatedAt = license.LastValidatedAt?.UtcDateTime,
                LastValidationStatus = license.LastValidationStatus?.ToString(),
                license.LastValidationErrorCode
            },
            transaction: tx());
    }

    public Task<int> DeleteAsync(CancellationToken cancellationToken)
    {
        const string sql = "DELETE FROM installedlicenses WHERE id = 1";
        return db.ExecuteAsync(sql, transaction: tx());
    }

    public Task<int> UpdateValidationStatusAsync(
        LicenseStatus status,
        DateTimeOffset validatedAt,
        string? validationErrorCode,
        CancellationToken cancellationToken)
    {
        const string sql = """
            UPDATE installedlicenses
            SET lastvalidatedat = @ValidatedAt,
                lastvalidationstatus = @Status,
                lastvalidationerrorcode = @ValidationErrorCode
            WHERE id = 1
            """;

        return db.ExecuteAsync(
            sql,
            new
            {
                ValidatedAt = validatedAt.UtcDateTime,
                Status = status.ToString(),
                ValidationErrorCode = validationErrorCode
            },
            transaction: tx());
    }
}

internal sealed class LicenseUsageRepository(IDbConnection db, Func<IDbTransaction> tx) : ILicenseUsageRepository
{
    public async Task<LicenseReadModel> GetLicenseReadModelAsync(CancellationToken cancellationToken)
    {
        const string sql = """
            SELECT
                (SELECT COUNT(*)::int FROM oidcproviders) AS OidcProviders,
                (SELECT COUNT(*)::int FROM platforms WHERE connectortype = 'EdgeAgent') AS EdgeAgentPlatforms,
                (SELECT COUNT(*)::int FROM secretproviders WHERE providertype <> 'InternalEncrypted') AS SecretProviders,
                (SELECT COUNT(*)::int FROM roles WHERE roletype = 'Custom') AS CustomRoles,
                (SELECT COUNT(*)::int FROM users u JOIN actors a ON a.id = u.actorid WHERE a.isenabled) AS ActiveUsers,
                (SELECT COUNT(*)::int FROM platforms) AS Platforms,
                il.rawlicense AS RawLicense,
                il.fingerprint AS Fingerprint,
                il.installedat AS InstalledAt,
                il.installedbyactorid AS InstalledByActorId,
                il.lastvalidatedat AS LastValidatedAt,
                il.lastvalidationstatus AS LastValidationStatus,
                il.lastvalidationerrorcode AS LastValidationErrorCode
            FROM (SELECT 1) seed
            LEFT JOIN installedlicenses il ON il.id = 1
            """;

        var dto = await db.QuerySingleAsync<LicenseReadModelDto>(sql, transaction: tx());
        return dto.ToDomain();
    }
}

internal sealed record CitadelInstanceIdentityDto(Guid InstanceId, DateTime CreatedAt)
{
    public CitadelInstanceIdentity ToDomain() => new(InstanceId, LicenseDateTimeMapper.ToOffset(CreatedAt));
}

internal sealed record InstalledLicenseDto(
    string RawLicense,
    string Fingerprint,
    DateTime InstalledAt,
    Guid? InstalledByActorId,
    DateTime? LastValidatedAt,
    string? LastValidationStatus,
    string? LastValidationErrorCode)
{
    public InstalledLicense ToDomain()
        => new(
            RawLicense,
            Fingerprint,
            LicenseDateTimeMapper.ToOffset(InstalledAt),
            InstalledByActorId,
            LicenseDateTimeMapper.ToOffset(LastValidatedAt),
            Enum.TryParse<LicenseStatus>(LastValidationStatus, out var status) ? status : null,
            LastValidationErrorCode);
}

internal sealed record LicenseReadModelDto(
    int OidcProviders,
    int EdgeAgentPlatforms,
    int SecretProviders,
    int CustomRoles,
    int ActiveUsers,
    int Platforms,
    string? RawLicense,
    string? Fingerprint,
    DateTime? InstalledAt,
    Guid? InstalledByActorId,
    DateTime? LastValidatedAt,
    string? LastValidationStatus,
    string? LastValidationErrorCode)
{
    public LicenseReadModel ToDomain()
    {
        var usage = new LicenseUsageSnapshot(
            OidcProviders,
            EdgeAgentPlatforms,
            SecretProviders,
            CustomRoles,
            ActiveUsers,
            Platforms);

        var installed = RawLicense is null || Fingerprint is null || InstalledAt is null
            ? null
            : new InstalledLicense(
                RawLicense,
                Fingerprint,
                LicenseDateTimeMapper.ToOffset(InstalledAt.Value),
                InstalledByActorId,
                LicenseDateTimeMapper.ToOffset(LastValidatedAt),
                Enum.TryParse<LicenseStatus>(LastValidationStatus, out var status) ? status : null,
                LastValidationErrorCode);

        return new LicenseReadModel(usage, installed);
    }
}

internal static class LicenseDateTimeMapper
{
    public static DateTimeOffset ToOffset(DateTime value)
    {
        var utc = value.Kind switch
        {
            DateTimeKind.Utc => value,
            DateTimeKind.Local => value.ToUniversalTime(),
            _ => DateTime.SpecifyKind(value, DateTimeKind.Utc)
        };

        return new DateTimeOffset(utc);
    }

    public static DateTimeOffset? ToOffset(DateTime? value)
        => value is null ? null : ToOffset(value.Value);
}

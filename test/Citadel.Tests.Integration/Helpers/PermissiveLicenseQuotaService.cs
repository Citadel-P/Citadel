using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Licensing;
using LightResults;

namespace Tests.Integration.Helpers;

internal sealed class PermissiveLicenseQuotaService : ILicenseQuotaService
{
    private static readonly IReadOnlyDictionary<LicenseLimit, int> Limits = new Dictionary<LicenseLimit, int>
    {
        [LicenseLimit.CustomRoles] = 1000,
        [LicenseLimit.ActiveUsers] = 1000,
        [LicenseLimit.Platforms] = 1000,
        [LicenseLimit.BackupPolicies] = 1000,
        [LicenseLimit.AutomationActions] = 1000
    };

    public ValueTask<LicenseState> GetOverviewAsync(CancellationToken cancellationToken)
        => ValueTask.FromResult(new LicenseState(
            Status: LicenseStatus.Valid,
            Edition: LicenseConstants.EditionBusiness,
            InstanceId: Guid.Empty,
            LicenseId: "integration-tests",
            ReplacedLicenseId: null,
            CustomerId: "integration-tests",
            CustomerName: "Integration Tests",
            Fingerprint: null,
            IssuedAt: null,
            NotBefore: null,
            ExpiresAt: null,
            GraceUntil: null,
            EffectiveLimits: Limits,
            Usage: new LicenseUsageSnapshot(0, 0, 0, 0, 0),
            Warnings: []));

    public ValueTask<Result> EnsureCanIncreaseAsync(
        IReadOnlyDictionary<LicenseLimit, int> increases,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
        => ValueTask.FromResult(Result.Success());
}

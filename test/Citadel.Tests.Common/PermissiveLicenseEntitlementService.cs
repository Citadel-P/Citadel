using Application.Services.Licensing;
using Domain;
using Domain.Entities.Licensing;
using LightResults;

namespace Tests.Common;

public sealed class PermissiveLicenseEntitlementService : ILicenseEntitlementService
{
    private static readonly IReadOnlySet<LicenseCapability> Capabilities =
        Enum.GetValues<LicenseCapability>().ToHashSet();

    private static readonly LicenseState State = new(
        Status: LicenseStatus.Valid,
        EffectiveEdition: LicenseConstants.EditionTeam,
        LicensedEdition: LicenseConstants.EditionTeam,
        InstanceId: Guid.Empty,
        LicenseSchema: LicenseConstants.CurrentSchema,
        LicenseId: "test-license",
        ReplacedLicenseId: null,
        CustomerId: "test-customer",
        CustomerName: "Test Customer",
        Fingerprint: null,
        IssuedAt: null,
        NotBefore: null,
        ExpiresAt: null,
        GraceUntil: null,
        EffectiveCapabilities: Capabilities,
        Warnings: []);

    public ValueTask<LicenseState> GetOverviewAsync(CancellationToken cancellationToken)
        => ValueTask.FromResult(State);

    public ValueTask<bool> IsEnabledAsync(
        LicenseCapability capability,
        CancellationToken cancellationToken)
        => ValueTask.FromResult(true);

    public ValueTask<Result> EnsureEnabledAsync(
        LicenseCapability capability,
        CancellationToken cancellationToken)
        => ValueTask.FromResult(Result.Success());
}

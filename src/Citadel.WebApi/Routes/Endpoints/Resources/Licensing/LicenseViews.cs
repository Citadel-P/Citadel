using Application.Features.Licensing;
using Domain;
using Domain.Entities.Licensing;

namespace WebApi.Routes.Endpoints.Resources.Licensing;

public sealed record InstallLicenseInput(string License);

public sealed record LicenseView(
    LicenseStatus Status,
    string EffectiveEdition,
    string? LicensedEdition,
    Guid InstanceId,
    int? LicenseSchema,
    string? LicenseId,
    string? ReplacedLicenseId,
    string? CustomerId,
    string? CustomerName,
    string? Fingerprint,
    DateTimeOffset? IssuedAt,
    DateTimeOffset? NotBefore,
    DateTimeOffset? ExpiresAt,
    DateTimeOffset? GraceUntil,
    IReadOnlyList<LicenseCapabilityView> Capabilities,
    IReadOnlyList<string> Warnings)
{
    public static LicenseView Map(LicenseState state)
        => new(
            state.Status,
            state.EffectiveEdition,
            state.LicensedEdition,
            state.InstanceId,
            state.LicenseSchema,
            state.LicenseId,
            state.ReplacedLicenseId,
            state.CustomerId,
            state.CustomerName,
            state.Fingerprint,
            state.IssuedAt,
            state.NotBefore,
            state.ExpiresAt,
            state.GraceUntil,
            MapCapabilities(state),
            state.Warnings);

    private static IReadOnlyList<LicenseCapabilityView> MapCapabilities(LicenseState state)
        => [.. Enum.GetValues<LicenseCapability>()
            .Select(capability => new LicenseCapabilityView(
                capability,
                state.EffectiveCapabilities.Contains(capability)))];
}

public sealed record LicenseEntitlementsView(
    LicenseStatus Status,
    string EffectiveEdition,
    IReadOnlyList<LicenseCapabilityView> Capabilities)
{
    public static LicenseEntitlementsView Map(LicenseState state)
        => new(
            state.Status,
            state.EffectiveEdition,
            [.. Enum.GetValues<LicenseCapability>()
                .Select(capability => new LicenseCapabilityView(
                    capability,
                    state.EffectiveCapabilities.Contains(capability)))]);
}

public sealed record LicenseCapabilityView(
    LicenseCapability Capability,
    bool Enabled);

public sealed record LicenseRequestView(
    string Product,
    Guid InstanceId,
    string CoreVersion,
    DateTimeOffset GeneratedAt)
{
    public static LicenseRequestView Map(LicenseRequest request)
        => new(
            request.Product,
            request.InstanceId,
            request.CoreVersion,
            request.GeneratedAt);
}

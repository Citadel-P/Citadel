using Application.Features.Licensing;
using Domain;
using Domain.Entities.Licensing;

namespace WebApi.Routes.Endpoints.Resources.Licensing;

public sealed record InstallLicenseInput(string License);

public sealed record LicenseView(
    LicenseStatus Status,
    string Edition,
    Guid InstanceId,
    string? LicenseId,
    string? ReplacedLicenseId,
    string? CustomerId,
    string? CustomerName,
    string? Fingerprint,
    DateTimeOffset? IssuedAt,
    DateTimeOffset? NotBefore,
    DateTimeOffset? ExpiresAt,
    DateTimeOffset? GraceUntil,
    IReadOnlyList<LicenseLimitView> Limits,
    IReadOnlyList<string> Warnings)
{
    public static LicenseView Map(LicenseState state)
        => new(
            state.Status,
            state.Edition,
            state.InstanceId,
            state.LicenseId,
            state.ReplacedLicenseId,
            state.CustomerId,
            state.CustomerName,
            state.Fingerprint,
            state.IssuedAt,
            state.NotBefore,
            state.ExpiresAt,
            state.GraceUntil,
            [.. state.EffectiveLimits
                .OrderBy(x => x.Key)
                .Select(x => new LicenseLimitView(
                    x.Key,
                    state.Usage.GetValue(x.Key),
                    x.Value,
                    state.Usage.GetValue(x.Key) > x.Value))],
            state.Warnings);
}

public sealed record LicenseLimitView(
    LicenseLimit Limit,
    int Current,
    int Maximum,
    bool OverQuota);

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

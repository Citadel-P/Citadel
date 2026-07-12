using Domain;

namespace Domain.Entities.Licensing;

public sealed record CitadelInstanceIdentity(
    Guid InstanceId,
    DateTimeOffset CreatedAt);

public sealed record InstalledLicense(
    string RawLicense,
    string Fingerprint,
    DateTimeOffset InstalledAt,
    Guid? InstalledByActorId,
    DateTimeOffset? LastValidatedAt = null,
    LicenseStatus? LastValidationStatus = null,
    string? LastValidationErrorCode = null);

public sealed record LicenseCustomer(
    string Id,
    string Name);

public sealed record LicensePayload(
    int Schema,
    string Product,
    string Issuer,
    string Audience,
    string LicenseId,
    string? ReplacedLicenseId,
    LicenseCustomer Customer,
    string Edition,
    Guid InstanceId,
    DateTimeOffset IssuedAt,
    DateTimeOffset NotBefore,
    DateTimeOffset ExpiresAt,
    DateTimeOffset? GraceUntil,
    IReadOnlyDictionary<string, int> Limits);

public sealed record LicenseProtectedHeader(
    string Alg,
    string Typ,
    string Kid);

public sealed record VerifiedLicense(
    string RawLicense,
    string Fingerprint,
    string KeyId,
    LicensePayload Payload,
    LicenseStatus Status,
    IReadOnlyDictionary<LicenseLimit, int> EffectiveLimits,
    IReadOnlyList<string> Warnings);

public sealed record LicenseVerificationResult(
    LicenseStatus Status,
    string? ErrorCode,
    string? ErrorMessage,
    VerifiedLicense? License)
{
    public bool IsAccepted =>
        Status is LicenseStatus.Valid or LicenseStatus.GracePeriod or LicenseStatus.NotYetValid;

    public static LicenseVerificationResult Failed(LicenseStatus status, string errorCode, string errorMessage)
        => new(status, errorCode, errorMessage, null);

    public static LicenseVerificationResult Succeeded(VerifiedLicense license)
        => new(license.Status, null, null, license);
}

public sealed record LicenseUsageSnapshot(
    int OidcProviders,
    int EdgeAgentPlatforms,
    int SecretProviders,
    int CustomRoles,
    int ActiveUsers,
    int Platforms)
{
    public int GetValue(LicenseLimit limit)
        => limit switch
        {
            LicenseLimit.OidcProviders => OidcProviders,
            LicenseLimit.EdgeAgentPlatforms => EdgeAgentPlatforms,
            LicenseLimit.SecretProviders => SecretProviders,
            LicenseLimit.CustomRoles => CustomRoles,
            LicenseLimit.ActiveUsers => ActiveUsers,
            LicenseLimit.Platforms => Platforms,
            _ => 0
        };
}

public sealed record LicenseReadModel(
    LicenseUsageSnapshot Usage,
    InstalledLicense? InstalledLicense);

public sealed record LicenseLimitState(
    LicenseLimit Limit,
    int Current,
    int Maximum,
    bool OverQuota);

public sealed record LicenseState(
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
    IReadOnlyDictionary<LicenseLimit, int> EffectiveLimits,
    LicenseUsageSnapshot Usage,
    IReadOnlyList<string> Warnings);

public sealed record LicenseQuotaViolation(
    LicenseLimit Limit,
    int Current,
    int Requested,
    int Maximum);

public sealed record LicenseQuotaExceeded(
    IReadOnlyList<LicenseQuotaViolation> Violations,
    LicenseStatus LicenseStatus,
    string Edition);

public static class LicenseConstants
{
    public const int CurrentSchema = 1;
    public const string Product = "citadel";
    public const string Issuer = "citadel-p";
    public const string Audience = "citadel-core";
    public const string EditionCommunity = "Community";
    public const string EditionBusiness = "Business";

    public const string JoseAlgorithm = "Ed25519";
    public const string JoseType = "citadel-license+jws";

    public const int MaxCompactLicenseBytes = 64 * 1024;
    public const int MaxKeyIdLength = 128;
    public const int MaxLicenseIdLength = 128;
    public const int MaxCustomerIdLength = 128;
    public const int MaxCustomerNameLength = 256;
    public const int MaxLimitEntries = 32;
    public const int JsonMaxDepth = 16;
}

public static class LicenseLimitKeys
{
    public const string OidcProviders = "oidc-providers";
    public const string EdgeAgentPlatforms = "edge-agent-platforms";
    public const string SecretProviders = "secret-providers";
    public const string CustomRoles = "custom-roles";
    public const string ActiveUsers = "active-users";
    public const string Platforms = "platforms";

    public static string GetKey(LicenseLimit limit)
        => limit switch
        {
            LicenseLimit.OidcProviders => OidcProviders,
            LicenseLimit.EdgeAgentPlatforms => EdgeAgentPlatforms,
            LicenseLimit.SecretProviders => SecretProviders,
            LicenseLimit.CustomRoles => CustomRoles,
            LicenseLimit.ActiveUsers => ActiveUsers,
            LicenseLimit.Platforms => Platforms,
            _ => throw new ArgumentOutOfRangeException(nameof(limit), limit, null)
        };

    public static bool TryGetLimit(string key, out LicenseLimit limit)
    {
        limit = key switch
        {
            OidcProviders => LicenseLimit.OidcProviders,
            EdgeAgentPlatforms => LicenseLimit.EdgeAgentPlatforms,
            SecretProviders => LicenseLimit.SecretProviders,
            CustomRoles => LicenseLimit.CustomRoles,
            ActiveUsers => LicenseLimit.ActiveUsers,
            Platforms => LicenseLimit.Platforms,
            _ => default
        };

        return key is OidcProviders or EdgeAgentPlatforms or SecretProviders or CustomRoles or ActiveUsers or Platforms;
    }
}

public static class CommunityLicenseLimits
{
    public static readonly IReadOnlyDictionary<LicenseLimit, int> Values = new Dictionary<LicenseLimit, int>
    {
        [LicenseLimit.OidcProviders] = 1,
        [LicenseLimit.EdgeAgentPlatforms] = 1,
        [LicenseLimit.SecretProviders] = 1,
        [LicenseLimit.CustomRoles] = 0,
        [LicenseLimit.ActiveUsers] = 10,
        [LicenseLimit.Platforms] = 5
    };
}

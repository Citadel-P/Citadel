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
    IReadOnlyDictionary<string, int>? Limits,
    IReadOnlyList<string>? Capabilities = null);

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
    IReadOnlySet<LicenseCapability> EffectiveCapabilities,
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

public sealed record LicenseState(
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
    IReadOnlySet<LicenseCapability> EffectiveCapabilities,
    IReadOnlyList<string> Warnings);

public static class LicenseConstants
{
    public const int LegacySchema = 1;
    public const int CurrentSchema = 2;
    public const string Product = "citadel";
    public const string Issuer = "citadel-p";
    public const string Audience = "citadel-core";
    public const string EditionCommunity = "Community";
    public const string EditionBusiness = "Business";
    public const string EditionTeam = "Team";
    public const string EditionEnterprise = "Enterprise";

    public const string JoseAlgorithm = "Ed25519";
    public const string JoseType = "citadel-license+jws";

    public const int MaxCompactLicenseBytes = 64 * 1024;
    public const int MaxKeyIdLength = 128;
    public const int MaxLicenseIdLength = 128;
    public const int MaxCustomerIdLength = 128;
    public const int MaxCustomerNameLength = 256;
    public const int MaxLimitEntries = 32;
    public const int MaxCapabilityEntries = 64;
    public const int MaxCapabilityKeyLength = 128;
    public const int JsonMaxDepth = 16;
}

public static class LicenseCapabilityKeys
{
    public const string CustomAccessControl = "custom-access-control";
    public const string AutomatedOperations = "automated-operations";
    public const string AdvancedAlerting = "advanced-alerting";
    public const string OperationalGuardrails = "operational-guardrails";
    public const string ElasticBuildExecution = "elastic-build-execution";

    public static string GetKey(LicenseCapability capability)
        => capability switch
        {
            LicenseCapability.CustomAccessControl => CustomAccessControl,
            LicenseCapability.AutomatedOperations => AutomatedOperations,
            LicenseCapability.AdvancedAlerting => AdvancedAlerting,
            LicenseCapability.OperationalGuardrails => OperationalGuardrails,
            LicenseCapability.ElasticBuildExecution => ElasticBuildExecution,
            _ => throw new ArgumentOutOfRangeException(nameof(capability), capability, null)
        };

    public static bool TryGetCapability(string key, out LicenseCapability capability)
    {
        capability = key switch
        {
            CustomAccessControl => LicenseCapability.CustomAccessControl,
            AutomatedOperations => LicenseCapability.AutomatedOperations,
            AdvancedAlerting => LicenseCapability.AdvancedAlerting,
            OperationalGuardrails => LicenseCapability.OperationalGuardrails,
            ElasticBuildExecution => LicenseCapability.ElasticBuildExecution,
            _ => default
        };

        return key is CustomAccessControl
            or AutomatedOperations
            or AdvancedAlerting
            or OperationalGuardrails
            or ElasticBuildExecution;
    }

    public static IReadOnlySet<LicenseCapability> All { get; } =
        Enum.GetValues<LicenseCapability>().ToHashSet();
}

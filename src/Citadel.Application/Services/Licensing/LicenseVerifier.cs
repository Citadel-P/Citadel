using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using Domain;
using Domain.Entities.Licensing;
using NSec.Cryptography;

namespace Application.Services.Licensing;

public interface ILicenseVerifier
{
    LicenseVerificationResult Verify(string rawLicense, CitadelInstanceIdentity instance, DateTimeOffset now);
}

public interface ILicensePublicKeyRegistry
{
    bool TryGetPublicKey(string keyId, out string publicKeyPem);
}

public sealed class EmbeddedLicensePublicKeyRegistry : ILicensePublicKeyRegistry
{
    private static readonly IReadOnlyDictionary<string, string> Keys = new Dictionary<string, string>
    {
        ["citadel-license-2026-01"] = """
            -----BEGIN PUBLIC KEY-----
            MCowBQYDK2VwAyEA1lCChB1fwbhdK3a844AtpzUfCeGwn+o8+Al+OOHrCGQ=
            -----END PUBLIC KEY-----
            """,
        ["citadel-license-dev-2026-01"] = """
            -----BEGIN PUBLIC KEY-----
            MCowBQYDK2VwAyEAiKohdkN3GouALrVhfXvUHN/v4vZ4c5FNXVUHGaxxzzI=
            -----END PUBLIC KEY-----
            """
    };

    public bool TryGetPublicKey(string keyId, out string publicKeyPem)
        => Keys.TryGetValue(keyId, out publicKeyPem!);
}

public sealed class LicenseVerifier(ILicensePublicKeyRegistry publicKeyRegistry) : ILicenseVerifier
{
    private const string InvalidLicense = "LICENSE_INVALID";
    private static readonly SignatureAlgorithm SignatureAlgorithm = SignatureAlgorithm.Ed25519;

    public LicenseVerificationResult Verify(string rawLicense, CitadelInstanceIdentity instance, DateTimeOffset now)
    {
        if (!LicenseInputNormalizer.TryNormalize(rawLicense, out var normalized, out var normalizeError))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, normalizeError);

        var segments = normalized.Split('.');
        if (segments.Length != 3 || segments.Any(string.IsNullOrEmpty))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, "License must be a compact three-segment JWS.");

        if (!TryDecodeJson<LicenseProtectedHeader>(
                segments[0],
                LicenseJsonContext.Default.LicenseProtectedHeader,
                out var header,
                out var headerError))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, headerError);

        var headerResult = ValidateHeader(header);
        if (headerResult is not null)
            return headerResult;

        if (!publicKeyRegistry.TryGetPublicKey(header.Kid, out var publicKeyPem))
            return LicenseVerificationResult.Failed(LicenseStatus.UnknownSigningKey, "LICENSE_UNKNOWN_SIGNING_KEY", "License signing key is not known.");

        if (!TryVerifySignature(segments[0], segments[1], segments[2], publicKeyPem))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, "LICENSE_INVALID_SIGNATURE", "License signature is invalid.");

        if (!TryDecodeJson<LicensePayload>(
                segments[1],
                LicenseJsonContext.Default.LicensePayload,
                out var payload,
                out var payloadError))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, payloadError);

        var payloadResult = ValidatePayload(payload, instance, now);
        if (payloadResult.Status is LicenseStatus.Invalid or LicenseStatus.InstanceMismatch or LicenseStatus.UnsupportedSchema)
            return payloadResult;

        var effectiveLimits = BuildEffectiveLimits(payload, out var warnings);
        var fingerprint = LicenseFingerprint.Compute(normalized);
        var verified = new VerifiedLicense(
            RawLicense: normalized,
            Fingerprint: fingerprint,
            KeyId: header.Kid,
            Payload: payload,
            Status: payloadResult.Status,
            EffectiveLimits: effectiveLimits,
            Warnings: warnings);

        return LicenseVerificationResult.Succeeded(verified);
    }

    private static LicenseVerificationResult? ValidateHeader(LicenseProtectedHeader header)
    {
        if (header is null)
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, "License header is missing.");

        if (!string.Equals(header.Alg, LicenseConstants.JoseAlgorithm, StringComparison.Ordinal))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, "LICENSE_UNSUPPORTED_ALGORITHM", $"Unsupported license algorithm '{header.Alg}'.");

        if (!string.Equals(header.Typ, LicenseConstants.JoseType, StringComparison.Ordinal))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, "LICENSE_UNSUPPORTED_TYPE", $"Unsupported license type '{header.Typ}'.");

        if (string.IsNullOrWhiteSpace(header.Kid) || header.Kid.Length > LicenseConstants.MaxKeyIdLength)
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, "License key id is invalid.");

        return null;
    }

    private static LicenseVerificationResult ValidatePayload(
        LicensePayload payload,
        CitadelInstanceIdentity instance,
        DateTimeOffset now)
    {
        if (payload.Schema != LicenseConstants.CurrentSchema)
            return LicenseVerificationResult.Failed(LicenseStatus.UnsupportedSchema, "LICENSE_UNSUPPORTED_SCHEMA", "License schema is not supported.");

        if (!string.Equals(payload.Product, LicenseConstants.Product, StringComparison.Ordinal)
            || !string.Equals(payload.Issuer, LicenseConstants.Issuer, StringComparison.Ordinal)
            || !string.Equals(payload.Audience, LicenseConstants.Audience, StringComparison.Ordinal))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, "License product identity is invalid.");

        if (payload.InstanceId != instance.InstanceId)
            return LicenseVerificationResult.Failed(LicenseStatus.InstanceMismatch, "LICENSE_INSTANCE_MISMATCH", "License is bound to another Citadel instance.");

        if (!string.Equals(payload.Edition, LicenseConstants.EditionBusiness, StringComparison.Ordinal))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, "License edition is invalid.");

        if (!IsValidIdentifier(payload.LicenseId, LicenseConstants.MaxLicenseIdLength)
            || !IsValidOptionalReplacementId(payload.ReplacedLicenseId, payload.LicenseId)
            || payload.Customer is null
            || !IsValidIdentifier(payload.Customer.Id, LicenseConstants.MaxCustomerIdLength)
            || string.IsNullOrWhiteSpace(payload.Customer.Name)
            || payload.Customer.Name.Length > LicenseConstants.MaxCustomerNameLength)
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, "License identity fields are invalid.");

        if (payload.Limits.Count > LicenseConstants.MaxLimitEntries || payload.Limits.Values.Any(x => x < 0))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, "License limits are invalid.");

        if (payload.IssuedAt > payload.NotBefore
            || payload.IssuedAt > payload.ExpiresAt
            || payload.NotBefore > payload.ExpiresAt
            || (payload.GraceUntil.HasValue && payload.GraceUntil.Value < payload.ExpiresAt))
            return LicenseVerificationResult.Failed(LicenseStatus.Invalid, InvalidLicense, "License date range is invalid.");

        return new LicenseVerificationResult(DeriveTemporalStatus(payload, now), null, null, null);
    }

    public static LicenseStatus DeriveTemporalStatus(LicensePayload payload, DateTimeOffset now)
    {
        if (now < payload.NotBefore)
            return LicenseStatus.NotYetValid;

        if (now <= payload.ExpiresAt)
            return LicenseStatus.Valid;

        if (payload.GraceUntil.HasValue && now <= payload.GraceUntil.Value)
            return LicenseStatus.GracePeriod;

        return LicenseStatus.Expired;
    }

    private static IReadOnlyDictionary<LicenseLimit, int> BuildEffectiveLimits(
        LicensePayload payload,
        out IReadOnlyList<string> warnings)
    {
        var result = new Dictionary<LicenseLimit, int>(CommunityLicenseLimits.Values);
        var warningList = new List<string>();

        foreach (var (key, value) in payload.Limits)
        {
            if (!LicenseLimitKeys.TryGetLimit(key, out var limit))
            {
                warningList.Add($"Unknown license limit '{key}' was ignored.");
                continue;
            }

            result[limit] = Math.Max(CommunityLicenseLimits.Values[limit], value);
        }

        warnings = warningList;
        return result;
    }

    private static bool TryVerifySignature(string encodedHeader, string encodedPayload, string encodedSignature, string publicKeyPem)
    {
        try
        {
            var signingInput = Encoding.ASCII.GetBytes($"{encodedHeader}.{encodedPayload}");
            var signature = Base64Url.Decode(encodedSignature);
            var publicKeyBytes = Encoding.ASCII.GetBytes(publicKeyPem);
            var publicKey = PublicKey.Import(SignatureAlgorithm, publicKeyBytes, KeyBlobFormat.PkixPublicKeyText);
            return SignatureAlgorithm.Verify(publicKey, signingInput, signature);
        }
        catch
        {
            return false;
        }
    }

    private static bool TryDecodeJson<T>(
        string segment,
        JsonTypeInfo<T> jsonTypeInfo,
        out T value,
        out string error)
    {
        value = default!;
        error = string.Empty;

        try
        {
            var json = Base64Url.Decode(segment);
            using var document = JsonDocument.Parse(
                json,
                new JsonDocumentOptions { MaxDepth = LicenseConstants.JsonMaxDepth });
            value = document.Deserialize(jsonTypeInfo)!;
            if (value is null)
            {
                error = "License JSON segment is empty.";
                return false;
            }

            return true;
        }
        catch (Exception ex) when (ex is JsonException or FormatException or ArgumentException)
        {
            error = $"License JSON segment is invalid: {ex.Message}";
            return false;
        }
    }

    private static bool IsValidIdentifier(string? value, int maxLength)
        => !string.IsNullOrWhiteSpace(value) && value.Length <= maxLength;

    private static bool IsValidOptionalReplacementId(string? value, string licenseId)
        => value is null
           || (IsValidIdentifier(value, LicenseConstants.MaxLicenseIdLength)
               && !string.Equals(value, licenseId, StringComparison.Ordinal));
}

public static class LicenseInputNormalizer
{
    public static bool TryNormalize(string? input, out string normalized, out string error)
    {
        normalized = string.Empty;
        error = string.Empty;

        if (string.IsNullOrWhiteSpace(input))
        {
            error = "License is required.";
            return false;
        }

        normalized = TrimAsciiWhitespace(input);
        if (Encoding.UTF8.GetByteCount(normalized) > LicenseConstants.MaxCompactLicenseBytes)
        {
            error = "License input is too large.";
            return false;
        }

        foreach (var ch in normalized)
        {
            if (ch is ' ' or '\t' or '\r' or '\n' or '\f')
            {
                error = "License must not contain whitespace inside the compact JWS.";
                return false;
            }
        }

        return true;
    }

    private static string TrimAsciiWhitespace(string value)
    {
        var start = 0;
        var end = value.Length - 1;

        while (start <= end && IsAsciiWhitespace(value[start]))
            start++;

        while (end >= start && IsAsciiWhitespace(value[end]))
            end--;

        return start > end ? string.Empty : value[start..(end + 1)];
    }

    private static bool IsAsciiWhitespace(char ch)
        => ch is ' ' or '\t' or '\r' or '\n' or '\f';
}

public static class LicenseFingerprint
{
    public static string Compute(string normalizedCompactLicense)
        => Convert.ToHexString(SHA256.HashData(Encoding.ASCII.GetBytes(normalizedCompactLicense))).ToLowerInvariant();
}

internal static class Base64Url
{
    public static byte[] Decode(string value)
    {
        var base64 = value.Replace('-', '+').Replace('_', '/');
        base64 = base64.PadRight(base64.Length + ((4 - base64.Length % 4) % 4), '=');
        return Convert.FromBase64String(base64);
    }
}

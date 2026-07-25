using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Application.Services.Licensing;
using Domain;
using Domain.Entities.Licensing;
using NSec.Cryptography;

namespace Tests.Unit.Application.Services.Licensing;

public sealed class LicenseVerifierTests
{
    private static readonly Guid InstanceId = Guid.Parse("11111111-1111-1111-1111-111111111111");

    [Fact]
    public void Verify_Should_Accept_Valid_Ed25519_Compact_Jws()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30));
        var license = fixture.Sign(payload);

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.True(result.IsAccepted);
        Assert.Equal(LicenseStatus.Valid, result.Status);
        Assert.NotNull(result.License);
        Assert.Equal(
            LicenseCapabilityKeys.All.Order(),
            result.License.EffectiveCapabilities.Order());
    }

    [Fact]
    public void Verify_Should_Reject_Deprecated_EdDsa_Algorithm()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30));
        var license = fixture.Sign(payload, algorithm: "EdDSA");

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
        Assert.Equal("LICENSE_UNSUPPORTED_ALGORITHM", result.ErrorCode);
    }

    [Fact]
    public void Verify_Should_Reject_Unknown_Key_Id()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var license = fixture.Sign(CreatePayload(expiresAt: now.AddDays(30)), keyId: "unknown-key");

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.UnknownSigningKey, result.Status);
    }

    [Fact]
    public void Verify_Should_Reject_Wrong_Type()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var license = fixture.Sign(CreatePayload(expiresAt: now.AddDays(30)), type: "citadel-license+jwt");

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
        Assert.Equal("LICENSE_UNSUPPORTED_TYPE", result.ErrorCode);
    }

    [Theory]
    [InlineData("jku")]
    [InlineData("x5u")]
    [InlineData("jwk")]
    [InlineData("crit")]
    public void Verify_Should_Reject_Unsupported_Protected_Header_Properties(string property)
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var license = fixture.Sign(
            CreatePayload(expiresAt: now.AddDays(30)),
            additionalHeaderFields: new Dictionary<string, object?>
            {
                [property] = property == "crit"
                    ? new[] { "future-header" }
                    : "https://issuer.example.test/key"
            });

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
        Assert.Equal("LICENSE_INVALID", result.ErrorCode);
    }

    [Fact]
    public void Verify_Should_Reject_Wrong_Product_Identity()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with { Product = "other-product" };
        var license = fixture.Sign(payload);

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
    }

    [Fact]
    public void Verify_Should_Accept_Replacement_License_Metadata()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with { ReplacedLicenseId = "lic-previous" };
        var license = fixture.Sign(payload);

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.True(result.IsAccepted);
        Assert.Equal("lic-previous", result.License!.Payload.ReplacedLicenseId);
    }

    [Fact]
    public void Verify_Should_Reject_Replacement_License_Id_That_Equals_Current_License_Id()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with { ReplacedLicenseId = "lic-test" };
        var license = fixture.Sign(payload);

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
    }

    [Fact]
    public void Verify_Should_Reject_Replacement_License_Id_That_Is_Too_Long()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with { ReplacedLicenseId = new string('x', 129) };
        var license = fixture.Sign(payload);

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
    }

    [Fact]
    public void Verify_Should_Reject_Tampered_Payload()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var license = fixture.Sign(CreatePayload(expiresAt: now.AddDays(30)));
        var parts = license.Split('.');
        var tamperedPayload = Base64UrlEncode(JsonSerializer.SerializeToUtf8Bytes(
            CreatePayload(expiresAt: now.AddDays(60)),
            LicenseFixture.JsonOptions));

        var result = fixture.Verifier.Verify($"{parts[0]}.{tamperedPayload}.{parts[2]}", Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
        Assert.Equal("LICENSE_INVALID_SIGNATURE", result.ErrorCode);
    }

    [Fact]
    public void Verify_Should_Reject_License_With_Internal_Whitespace()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var license = fixture.Sign(CreatePayload(expiresAt: now.AddDays(30)));
        var withWhitespace = license.Insert(license.IndexOf('.') + 1, "\n");

        var result = fixture.Verifier.Verify(withWhitespace, Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
    }

    [Fact]
    public void Verify_Should_Return_InstanceMismatch_For_Another_Instance()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var license = fixture.Sign(CreatePayload(expiresAt: now.AddDays(30)));
        var otherInstance = new CitadelInstanceIdentity(Guid.Parse("22222222-2222-2222-2222-222222222222"), now);

        var result = fixture.Verifier.Verify(license, otherInstance, now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.InstanceMismatch, result.Status);
    }

    [Fact]
    public void Verify_Should_Return_NotYetValid_For_Future_NotBefore()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with
        {
            IssuedAt = now.AddDays(-1),
            NotBefore = now.AddDays(1)
        };
        var license = fixture.Sign(payload);

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.True(result.IsAccepted);
        Assert.Equal(LicenseStatus.NotYetValid, result.Status);
        Assert.NotNull(result.License);
    }

    [Fact]
    public void Verify_Should_Reject_Unsupported_Schema()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with
        {
            Schema = LicenseConstants.CurrentSchema + 1
        };

        var result = fixture.Verifier.Verify(fixture.Sign(payload), Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.UnsupportedSchema, result.Status);
        Assert.Equal("LICENSE_UNSUPPORTED_SCHEMA", result.ErrorCode);
    }

    [Fact]
    public void Verify_Should_Accept_Schema2_Enterprise_Without_Granting_Missing_Capabilities()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with
        {
            Edition = LicenseConstants.EditionEnterprise,
            Capabilities = [LicenseCapabilityKeys.CustomAccessControl]
        };

        var result = fixture.Verifier.Verify(fixture.Sign(payload), Instance(), now);

        Assert.True(result.IsAccepted);
        Assert.NotNull(result.License);
        Assert.Equal(
            [LicenseCapability.CustomAccessControl],
            result.License.EffectiveCapabilities);
    }

    [Fact]
    public void Verify_Should_Reject_Signed_Malformed_Payload_Without_Throwing()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var license = fixture.SignPayload(Encoding.UTF8.GetBytes("{not-json"));

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
        Assert.Equal("LICENSE_INVALID", result.ErrorCode);
    }

    [Fact]
    public void Verify_Should_Map_Legacy_Business_License_To_All_Team_Capabilities()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with
        {
            Schema = LicenseConstants.LegacySchema,
            Edition = LicenseConstants.EditionBusiness,
            Limits = null,
            Capabilities = null
        };
        var license = fixture.Sign(payload);

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.True(result.IsAccepted);
        Assert.NotNull(result.License);
        Assert.Equal(
            LicenseCapabilityKeys.All.Order(),
            result.License.EffectiveCapabilities.Order());
        Assert.Contains(result.License.Warnings, warning => warning.Contains("Legacy Business", StringComparison.Ordinal));
    }

    [Fact]
    public void Verify_Should_Ignore_Unknown_Schema2_Capability_With_Warning()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with
        {
            Capabilities =
            [
                LicenseCapabilityKeys.AutomatedOperations,
                "future-capability"
            ]
        };

        var result = fixture.Verifier.Verify(fixture.Sign(payload), Instance(), now);

        Assert.True(result.IsAccepted);
        Assert.NotNull(result.License);
        Assert.Equal(
            [LicenseCapability.AutomatedOperations],
            result.License.EffectiveCapabilities);
        Assert.Contains(result.License.Warnings, warning => warning.Contains("future-capability", StringComparison.Ordinal));
    }

    [Fact]
    public void Verify_Should_Reject_Duplicate_Schema2_Capabilities()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with
        {
            Capabilities =
            [
                LicenseCapabilityKeys.AutomatedOperations,
                LicenseCapabilityKeys.AutomatedOperations
            ]
        };

        var result = fixture.Verifier.Verify(fixture.Sign(payload), Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
    }

    [Fact]
    public void Verify_Should_Reject_NonUtc_Timestamps()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var offset = TimeSpan.FromHours(2);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with
        {
            IssuedAt = now.AddDays(-1).ToOffset(offset),
            NotBefore = now.AddDays(-1).ToOffset(offset),
            ExpiresAt = now.AddDays(30).ToOffset(offset),
            GraceUntil = now.AddDays(44).ToOffset(offset)
        };

        var result = fixture.Verifier.Verify(fixture.Sign(payload), Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
    }

    [Fact]
    public void Verify_Should_Reject_Negative_Legacy_Limit()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30)) with
        {
            Schema = LicenseConstants.LegacySchema,
            Edition = LicenseConstants.EditionBusiness,
            Limits = new Dictionary<string, int> { ["active-users"] = -1 },
            Capabilities = null
        };

        var result = fixture.Verifier.Verify(fixture.Sign(payload), Instance(), now);

        Assert.False(result.IsAccepted);
        Assert.Equal(LicenseStatus.Invalid, result.Status);
    }

    [Fact]
    public void Verify_Should_Derive_GracePeriod_And_Expired_From_Current_Time()
    {
        var fixture = LicenseFixture.Create();
        var expiresAt = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var license = fixture.Sign(CreatePayload(expiresAt: expiresAt, graceUntil: expiresAt.AddDays(14)));

        var grace = fixture.Verifier.Verify(license, Instance(), expiresAt.AddDays(1));
        var expired = fixture.Verifier.Verify(license, Instance(), expiresAt.AddDays(15));

        Assert.True(grace.IsAccepted);
        Assert.Equal(LicenseStatus.GracePeriod, grace.Status);
        Assert.False(expired.IsAccepted);
        Assert.Equal(LicenseStatus.Expired, expired.Status);
    }

    private static CitadelInstanceIdentity Instance()
        => new(InstanceId, new DateTimeOffset(2026, 7, 1, 0, 0, 0, TimeSpan.Zero));

    private static LicensePayload CreatePayload(DateTimeOffset expiresAt, DateTimeOffset? graceUntil = null)
        => new(
            Schema: LicenseConstants.CurrentSchema,
            Product: LicenseConstants.Product,
            Issuer: LicenseConstants.Issuer,
            Audience: LicenseConstants.Audience,
            LicenseId: "lic-test",
            ReplacedLicenseId: null,
            Customer: new LicenseCustomer("customer-test", "Test Customer"),
            Edition: LicenseConstants.EditionTeam,
            InstanceId: InstanceId,
            IssuedAt: expiresAt.AddDays(-30),
            NotBefore: expiresAt.AddDays(-30),
            ExpiresAt: expiresAt,
            GraceUntil: graceUntil,
            Limits: null,
            Capabilities: LicenseCapabilityKeys.All
                .Select(LicenseCapabilityKeys.GetKey)
                .ToArray());

    private sealed class LicenseFixture
    {
        private const string KeyId = "citadel-license-test";
        public static readonly JsonSerializerOptions JsonOptions = new()
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
        };

        private readonly Key key;

        private LicenseFixture(Key key, string publicKeyPem)
        {
            this.key = key;
            Verifier = new LicenseVerifier(new TestPublicKeyRegistry(publicKeyPem));
        }

        public LicenseVerifier Verifier { get; }

        public static LicenseFixture Create()
        {
            var key = new Key(SignatureAlgorithm.Ed25519, new KeyCreationParameters
            {
                ExportPolicy = KeyExportPolicies.AllowPlaintextExport
            });
            var publicKeyPem = Encoding.ASCII.GetString(key.Export(KeyBlobFormat.PkixPublicKeyText));
            return new LicenseFixture(key, publicKeyPem);
        }

        public string Sign(
            LicensePayload payload,
            string algorithm = LicenseConstants.JoseAlgorithm,
            string type = LicenseConstants.JoseType,
            string keyId = KeyId,
            IReadOnlyDictionary<string, object?>? additionalHeaderFields = null)
        {
            return SignPayload(
                JsonSerializer.SerializeToUtf8Bytes(payload, JsonOptions),
                algorithm,
                type,
                keyId,
                additionalHeaderFields);
        }

        public string SignPayload(
            byte[] payload,
            string algorithm = LicenseConstants.JoseAlgorithm,
            string type = LicenseConstants.JoseType,
            string keyId = KeyId,
            IReadOnlyDictionary<string, object?>? additionalHeaderFields = null)
        {
            object header;
            if (additionalHeaderFields is null)
            {
                header = new LicenseProtectedHeader(algorithm, type, keyId);
            }
            else
            {
                var headerFields = additionalHeaderFields.ToDictionary(
                    pair => pair.Key,
                    pair => pair.Value,
                    StringComparer.Ordinal);
                headerFields["alg"] = algorithm;
                headerFields["typ"] = type;
                headerFields["kid"] = keyId;
                header = headerFields;
            }

            var encodedHeader = Base64UrlEncode(JsonSerializer.SerializeToUtf8Bytes(header, JsonOptions));
            var encodedPayload = Base64UrlEncode(payload);
            var signingInput = Encoding.ASCII.GetBytes($"{encodedHeader}.{encodedPayload}");
            var signature = SignatureAlgorithm.Ed25519.Sign(key, signingInput);

            return $"{encodedHeader}.{encodedPayload}.{Base64UrlEncode(signature)}";
        }
    }

    private sealed class TestPublicKeyRegistry(string publicKeyPem) : ILicensePublicKeyRegistry
    {
        private readonly string pem = publicKeyPem;

        public bool TryGetPublicKey(string keyId, out string publicKeyPem)
        {
            if (keyId == "citadel-license-test")
            {
                publicKeyPem = pem;
                return true;
            }

            publicKeyPem = string.Empty;
            return false;
        }
    }

    private static string Base64UrlEncode(byte[] value)
        => Convert.ToBase64String(value).TrimEnd('=').Replace('+', '-').Replace('/', '_');
}

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
        Assert.Equal(20, result.License.EffectiveLimits[LicenseLimit.ActiveUsers]);
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
    public void Verify_Should_Use_Community_Limits_When_Limits_Are_Missing()
    {
        var fixture = LicenseFixture.Create();
        var now = new DateTimeOffset(2026, 7, 12, 10, 0, 0, TimeSpan.Zero);
        var payload = CreatePayload(expiresAt: now.AddDays(30));
        var payloadWithoutLimits = new
        {
            payload.Schema,
            payload.Product,
            payload.Issuer,
            payload.Audience,
            payload.LicenseId,
            payload.ReplacedLicenseId,
            payload.Customer,
            payload.Edition,
            payload.InstanceId,
            payload.IssuedAt,
            payload.NotBefore,
            payload.ExpiresAt,
            payload.GraceUntil
        };
        var license = fixture.SignPayload(JsonSerializer.SerializeToUtf8Bytes(
            payloadWithoutLimits,
            LicenseFixture.JsonOptions));

        var result = fixture.Verifier.Verify(license, Instance(), now);

        Assert.True(result.IsAccepted);
        Assert.NotNull(result.License);
        Assert.Equal(
            CommunityLicenseLimits.Values[LicenseLimit.ActiveUsers],
            result.License.EffectiveLimits[LicenseLimit.ActiveUsers]);
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
            Edition: LicenseConstants.EditionBusiness,
            InstanceId: InstanceId,
            IssuedAt: expiresAt.AddDays(-30),
            NotBefore: expiresAt.AddDays(-30),
            ExpiresAt: expiresAt,
            GraceUntil: graceUntil,
            Limits: new Dictionary<string, int>
            {
                [LicenseLimitKeys.CustomRoles] = 10,
                [LicenseLimitKeys.ActiveUsers] = 20,
                [LicenseLimitKeys.Platforms] = 10,
                [LicenseLimitKeys.BackupPolicies] = 25,
                [LicenseLimitKeys.AutomationActions] = 50
            });

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
            string keyId = KeyId)
        {
            return SignPayload(
                JsonSerializer.SerializeToUtf8Bytes(payload, JsonOptions),
                algorithm,
                type,
                keyId);
        }

        public string SignPayload(
            byte[] payload,
            string algorithm = LicenseConstants.JoseAlgorithm,
            string type = LicenseConstants.JoseType,
            string keyId = KeyId)
        {
            var header = new LicenseProtectedHeader(algorithm, type, keyId);
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

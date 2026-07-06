using Application.Features.Oidc.Commands;
using Application.Services;
using Domain.Entities.Oidc;
using Hosting.Common;
using System.Security.Cryptography;

namespace Tests.Unit.Application.Services;

public sealed class OidcAuthenticationServiceTests
{
    [Fact]
    public void CreateAuthorizationRequest_ShouldIncludePkceAndOidcParameters()
    {
        var provider = CreateProvider();
        var discovery = new OidcDiscoveryResult(
            "https://issuer.test",
            "https://issuer.test/authorize",
            "https://issuer.test/token",
            "https://issuer.test/jwks");
        var service = new OidcAuthenticationService();
        const string codeVerifier = "deterministic-code-verifier";

        var result = service.CreateAuthorizationRequest(
            provider,
            discovery,
            "https://citadel.test/api/v1/authentication/oidc/00000000-0000-0000-0000-000000000001/callback",
            "state-value",
            "nonce-value",
            codeVerifier);

        var uri = new Uri(result.Url);
        var query = ParseQuery(uri.Query);

        Assert.Equal("https://issuer.test/authorize", result.Url[..result.Url.IndexOf('?', StringComparison.Ordinal)]);
        Assert.Equal("code", query["response_type"]);
        Assert.Equal("client-id", query["client_id"]);
        Assert.Equal("openid profile email", query["scope"]);
        Assert.Equal("state-value", query["state"]);
        Assert.Equal("nonce-value", query["nonce"]);
        Assert.Equal("S256", query["code_challenge_method"]);
        Assert.Equal(Base64UrlEncode(SHA256.HashData(System.Text.Encoding.ASCII.GetBytes(codeVerifier))), query["code_challenge"]);
        Assert.Equal(codeVerifier, result.CodeVerifier);
    }

    [Theory]
    [InlineData("https://citadel.test/login", "https://citadel.test/login")]
    [InlineData("http://localhost:5173/login", "http://localhost:5173/login")]
    [InlineData("https://evil.test/login", null)]
    [InlineData("/login", "/login")]
    [InlineData("//evil.test/login", null)]
    [InlineData("javascript:alert(1)", null)]
    [InlineData("login", null)]
    public void NormalizeReturnUrl_ShouldOnlyAllowTrustedAbsoluteOrRootRelativeUrls(string value, string? expected)
    {
        Assert.Equal(
            expected,
            OidcLoginUrl.NormalizeReturnUrl(
                value,
                "https://citadel.test/api/v1/authentication/oidc/00000000-0000-0000-0000-000000000001/callback",
                ["http://localhost:5173"]));
    }

    private static OidcProvider CreateProvider()
        => new(
            "github",
            null,
            "GitHub",
            "https://issuer.test",
            "client-id",
            null,
            "openid profile email",
            enabled: true,
            autoProvisionUsers: false,
            allowEmailAutoLink: true,
            requireEmailVerified: true,
            allowedEmailDomains: null,
            requiredClaimName: null,
            requiredClaimValues: null,
            defaultRoleId: null,
            createdByActorId: Constants.SystemId);

    private static string Base64UrlEncode(byte[] bytes)
        => Convert.ToBase64String(bytes)
            .TrimEnd('=')
            .Replace('+', '-')
            .Replace('/', '_');

    private static Dictionary<string, string> ParseQuery(string query)
        => query.TrimStart('?')
            .Split('&', StringSplitOptions.RemoveEmptyEntries)
            .Select(part => part.Split('=', 2))
            .ToDictionary(
                parts => Uri.UnescapeDataString(parts[0]),
                parts => parts.Length > 1 ? Uri.UnescapeDataString(parts[1]) : string.Empty);
}

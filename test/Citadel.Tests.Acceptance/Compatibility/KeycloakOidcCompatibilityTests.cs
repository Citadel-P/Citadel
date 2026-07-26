using System.Net;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Compatibility;

[Collection("AcceptancePostgres")]
public sealed class KeycloakOidcCompatibilityTests(
    AcceptancePostgresFixture postgres)
{
    [Fact]
    public async Task Keycloak_ShouldDiscoverStartLoginAndRejectDisabledProvider()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var connectionString =
            await postgres.CreateDatabaseAsync(cancellationToken);
        var candidateDirectory = Path.Combine(
            Path.GetTempPath(),
            $"citadel-keycloak-{Guid.NewGuid():N}");
        Directory.CreateDirectory(candidateDirectory);

        try
        {
            await using var keycloak =
                await KeycloakInstance.StartAsync(cancellationToken);
            await using var candidate =
                await CandidateApplicationProcess.StartAsync(
                    connectionString,
                    candidateDirectory,
                    cancellationToken);
            await candidate.AuthenticateAsAdminAsync(cancellationToken);

            var providerId = await CreateProviderAsync(
                candidate,
                keycloak.Issuer,
                cancellationToken);
            await AssertDiscoveryAsync(
                candidate,
                providerId,
                keycloak.Issuer,
                cancellationToken);
            await AssertLoginRedirectAsync(
                candidate,
                providerId,
                keycloak.Issuer,
                cancellationToken);

            await DisableProviderAsync(
                candidate,
                providerId,
                cancellationToken);
            await AssertProviderCannotStartLoginAsync(
                candidate,
                providerId,
                cancellationToken);
        }
        finally
        {
            await postgres.DropDatabaseAsync(
                connectionString,
                CancellationToken.None);
            await DeleteDirectoryAsync(candidateDirectory);
        }
    }

    private static async Task<Guid> CreateProviderAsync(
        CandidateApplicationProcess candidate,
        string issuer,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            "/api/v1/oidcProviders",
            new
            {
                name = $"acceptance-keycloak-{Guid.NewGuid():N}",
                description = "Disposable Keycloak acceptance provider",
                displayName = "Keycloak Acceptance",
                issuer,
                clientId = "citadel-e2e",
                clientSecret = "citadel-e2e-client-secret",
                scopes = "openid profile email",
                enabled = true,
                autoProvisionUsers = false,
                allowEmailAutoLink = true,
                requireEmailVerified = true,
                allowedEmailDomains = (string?)null,
                requiredClaimName = "citadel_access",
                requiredClaimValues = "allowed",
                defaultRoleId = (Guid?)null
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(
            cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"""
            Creating the Keycloak provider failed with HTTP {(int)response.StatusCode}: {body}
            {candidate.Output}
            """);

        using var json = JsonDocument.Parse(body);
        return json.RootElement.GetProperty("id").GetGuid();
    }

    private static async Task AssertDiscoveryAsync(
        CandidateApplicationProcess candidate,
        Guid providerId,
        string expectedIssuer,
        CancellationToken cancellationToken)
    {
        using var response = await candidate.Client.PostAsJsonAsync(
            $"/api/v1/oidcProviders/{providerId:D}/testDiscovery",
            new { },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(
            cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"""
            Discovering Keycloak metadata failed with HTTP {(int)response.StatusCode}: {body}
            {candidate.Output}
            """);

        using var json = JsonDocument.Parse(body);
        var discovery = json.RootElement;
        Assert.Equal(
            expectedIssuer,
            discovery.GetProperty("issuer").GetString());
        Assert.StartsWith(
            expectedIssuer,
            discovery.GetProperty("authorizationEndpoint").GetString(),
            StringComparison.Ordinal);
        Assert.StartsWith(
            expectedIssuer,
            discovery.GetProperty("tokenEndpoint").GetString(),
            StringComparison.Ordinal);
        Assert.StartsWith(
            expectedIssuer,
            discovery.GetProperty("jwksUri").GetString(),
            StringComparison.Ordinal);
    }

    private static async Task AssertLoginRedirectAsync(
        CandidateApplicationProcess candidate,
        Guid providerId,
        string expectedIssuer,
        CancellationToken cancellationToken)
    {
        using var client = new HttpClient(
            new HttpClientHandler
            {
                AllowAutoRedirect = false
            })
        {
            BaseAddress = candidate.Client.BaseAddress,
            Timeout = TimeSpan.FromSeconds(30)
        };
        using var response = await client.GetAsync(
            $"/api/v1/authentication/oidc/{providerId:D}/login?returnUrl=%2Fprofile",
            cancellationToken);
        Assert.Equal(HttpStatusCode.Redirect, response.StatusCode);

        var location = response.Headers.Location;
        Assert.NotNull(location);
        Assert.StartsWith(
            $"{expectedIssuer}/protocol/openid-connect/auth",
            location.ToString(),
            StringComparison.Ordinal);
        Assert.Contains("code_challenge=", location.Query);
        Assert.Contains("code_challenge_method=S256", location.Query);
        Assert.DoesNotContain("client_secret", location.Query);
    }

    private static async Task DisableProviderAsync(
        CandidateApplicationProcess candidate,
        Guid providerId,
        CancellationToken cancellationToken)
    {
        using var request = new HttpRequestMessage(
            HttpMethod.Patch,
            $"/api/v1/oidcProviders/{providerId:D}")
        {
            Content = new StringContent(
                """{"enabled":false}""",
                Encoding.UTF8,
                "application/merge-patch+json")
        };
        using var response = await candidate.Client.SendAsync(
            request,
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(
            cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Disabling the Keycloak provider failed with HTTP {(int)response.StatusCode}: {body}");
    }

    private static async Task AssertProviderCannotStartLoginAsync(
        CandidateApplicationProcess candidate,
        Guid providerId,
        CancellationToken cancellationToken)
    {
        using var providersResponse = await candidate.Client.GetAsync(
            "/api/v1/authentication/oidc/providers",
            cancellationToken);
        var providersBody = await providersResponse.Content.ReadAsStringAsync(
            cancellationToken);
        Assert.True(
            providersResponse.IsSuccessStatusCode,
            $"Listing enabled OIDC providers failed with HTTP {(int)providersResponse.StatusCode}: {providersBody}");
        using (var providersJson = JsonDocument.Parse(providersBody))
        {
            Assert.DoesNotContain(
                providersJson.RootElement
                    .GetProperty("providers")
                    .EnumerateArray(),
                provider =>
                    provider.GetProperty("id").GetGuid() == providerId);
        }

        using var loginResponse = await candidate.Client.GetAsync(
            $"/api/v1/authentication/oidc/{providerId:D}/login",
            cancellationToken);
        Assert.Equal(HttpStatusCode.NotFound, loginResponse.StatusCode);
    }

    private static async Task DeleteDirectoryAsync(string path)
    {
        for (var attempt = 0; attempt < 5; attempt++)
        {
            if (!Directory.Exists(path))
                return;

            try
            {
                Directory.Delete(path, recursive: true);
                return;
            }
            catch (IOException) when (attempt < 4)
            {
            }
            catch (UnauthorizedAccessException) when (attempt < 4)
            {
            }

            await Task.Delay(TimeSpan.FromMilliseconds(200));
        }
    }
}

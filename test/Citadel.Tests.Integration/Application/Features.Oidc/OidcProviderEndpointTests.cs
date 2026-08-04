using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using Application.Services;
using Domain.Contracts.Interfaces;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Oidc;

public sealed class OidcProviderEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly FakeOidcDiscoveryService discoveryService = new();

    protected override void ConfigureTestServices(IServiceCollection services)
        => services.ReplaceService<IOidcDiscoveryService>(discoveryService);

    [Fact]
    public async Task ProviderEndpoints_ShouldPersistCompleteLifecycleAndTestDiscovery()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var createResponse = await Client.PostAsJsonAsync(
            "/api/v1/oidcProviders",
            new
            {
                name = "oidc-api",
                description = "created through the API",
                displayName = "OIDC API",
                issuer = "https://issuer.test",
                clientId = "citadel-client",
                clientSecret = "initial-secret",
                scopes = "openid profile email",
                enabled = true,
                autoProvisionUsers = false,
                allowEmailAutoLink = true,
                requireEmailVerified = true,
                allowedEmailDomains = "citadel.test",
                requiredClaimName = (string?)null,
                requiredClaimValues = (string?)null,
                defaultRoleId = (Guid?)null
            },
            cancellationToken);
        createResponse.EnsureSuccessStatusCode();

        using var createDocument = JsonDocument.Parse(
            await createResponse.Content.ReadAsStreamAsync(cancellationToken));
        var providerId = createDocument.RootElement.GetProperty("id").GetGuid();
        Assert.True(createDocument.RootElement.GetProperty("hasClientSecret").GetBoolean());

        await using (var scope = Services.CreateAsyncScope())
        {
            var persisted = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
                .OidcProviders.GetAsync(providerId, cancellationToken);
            Assert.NotNull(persisted);
            Assert.NotEqual("initial-secret", persisted.ClientSecretCiphertext);
        }

        using var listResponse = await Client.GetAsync("/api/v1/oidcProviders", cancellationToken);
        listResponse.EnsureSuccessStatusCode();
        using var listDocument = JsonDocument.Parse(await listResponse.Content.ReadAsStreamAsync(cancellationToken));
        Assert.Contains(
            listDocument.RootElement.GetProperty("providers").EnumerateArray(),
            provider => provider.GetProperty("id").GetGuid() == providerId);

        using var getResponse = await Client.GetAsync($"/api/v1/oidcProviders/{providerId:D}", cancellationToken);
        getResponse.EnsureSuccessStatusCode();

        using var updateRequest = new HttpRequestMessage(HttpMethod.Patch, $"/api/v1/oidcProviders/{providerId:D}")
        {
            Content = new StringContent(
                """
                {
                  "displayName": "Updated OIDC",
                  "description": "updated through merge patch",
                  "clientSecret": "rotated-secret"
                }
                """,
                Encoding.UTF8,
                "application/merge-patch+json")
        };
        using var updateResponse = await Client.SendAsync(updateRequest, cancellationToken);
        updateResponse.EnsureSuccessStatusCode();

        using var renameResponse = await Client.PostAsJsonAsync(
            "/api/v1/oidcProviders/rename",
            new { id = providerId, name = "oidc-renamed" },
            cancellationToken);
        renameResponse.EnsureSuccessStatusCode();

        using var metadataRequest = new HttpRequestMessage(
            HttpMethod.Patch,
            $"/api/v1/oidcProviders/{providerId:D}/_metadata")
        {
            Content = new StringContent(
                """{"description":"metadata description"}""",
                Encoding.UTF8,
                "application/merge-patch+json")
        };
        using var metadataResponse = await Client.SendAsync(metadataRequest, cancellationToken);
        metadataResponse.EnsureSuccessStatusCode();

        using var persistedDiscoveryResponse = await Client.PostAsync(
            $"/api/v1/oidcProviders/{providerId:D}/testDiscovery",
            content: null,
            cancellationToken);
        persistedDiscoveryResponse.EnsureSuccessStatusCode();

        using var directDiscoveryResponse = await Client.PostAsJsonAsync(
            "/api/v1/oidcProviders/testDiscovery",
            new { providerId = (Guid?)null, issuer = "https://direct-issuer.test" },
            cancellationToken);
        directDiscoveryResponse.EnsureSuccessStatusCode();
        Assert.Contains("https://direct-issuer.test", discoveryService.RequestedIssuers);

        await using (var scope = Services.CreateAsyncScope())
        {
            var persisted = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
                .OidcProviders.GetAsync(providerId, cancellationToken);
            Assert.NotNull(persisted);
            Assert.Equal("oidc-renamed", persisted.Name);
            Assert.Equal("Updated OIDC", persisted.DisplayName);
            Assert.Equal("metadata description", persisted.Description);
            Assert.NotEqual("rotated-secret", persisted.ClientSecretCiphertext);
        }

        using var deleteResponse = await Client.DeleteAsync(
            $"/api/v1/oidcProviders/{providerId:D}",
            cancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.NoContent, deleteResponse.StatusCode);

        await using (var scope = Services.CreateAsyncScope())
        {
            var persisted = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
                .OidcProviders.GetAsync(providerId, cancellationToken);
            Assert.Null(persisted);
        }
    }

    private sealed class FakeOidcDiscoveryService : IOidcDiscoveryService
    {
        public List<string> RequestedIssuers { get; } = [];

        public Task<Result<OidcDiscoveryResult>> GetDiscoveryAsync(
            string issuer,
            CancellationToken cancellationToken)
            => TestDiscoveryAsync(issuer, cancellationToken);

        public Task<Result<OidcDiscoveryResult>> TestDiscoveryAsync(
            string issuer,
            CancellationToken cancellationToken)
        {
            RequestedIssuers.Add(issuer);
            var normalizedIssuer = issuer.TrimEnd('/');
            return Task.FromResult(Result.Success(new OidcDiscoveryResult(
                normalizedIssuer,
                $"{normalizedIssuer}/authorize",
                $"{normalizedIssuer}/token",
                $"{normalizedIssuer}/jwks")));
        }
    }
}

using System.Net;
using System.Text;
using Domain;
using Domain.Entities.ResourceBindings;
using Infrastructure.HttpClients.Serializer;
using Infrastructure.Repositories;
using Infrastructure.Vault;
using Refit;

namespace Tests.Unit.Infrastructure.Repositories;

public sealed class ExternalSecretProviderClientTests
{
    [Fact]
    public async Task TestConnectionAsync_Should_Check_Health_And_Token_Lookup()
    {
        var requests = new List<HttpRequestMessage>();
        var handler = new RecordingHandler(request =>
        {
            requests.Add(request);
            return request.RequestUri?.AbsolutePath switch
            {
                "/v1/sys/health" => new HttpResponseMessage(HttpStatusCode.OK),
                "/v1/auth/token/lookup-self" => new HttpResponseMessage(HttpStatusCode.OK),
                _ => new HttpResponseMessage(HttpStatusCode.NotFound)
            };
        });
        var client = CreateClient(handler);

        var result = await client.TestConnectionAsync(CreateProvider(), "vault-token", TestContext.Current.CancellationToken);

        Assert.True(result.Success);
        Assert.Equal("Connection successful. Vault is reachable and the token is valid.", result.Message);
        Assert.Equal(2, requests.Count);
        Assert.Equal("https://vault.local/v1/sys/health", requests[0].RequestUri?.ToString());
        Assert.Equal("https://vault.local/v1/auth/token/lookup-self", requests[1].RequestUri?.ToString());
        Assert.True(requests[1].Headers.TryGetValues("X-Vault-Token", out var tokenValues));
        Assert.Equal("vault-token", Assert.Single(tokenValues));
    }

    [Fact]
    public async Task TestConnectionAsync_Should_Return_Success_When_Token_Lookup_Is_Unsupported()
    {
        var handler = new RecordingHandler(request => request.RequestUri?.AbsolutePath switch
        {
            "/v1/sys/health" => new HttpResponseMessage(HttpStatusCode.OK),
            "/v1/auth/token/lookup-self" => new HttpResponseMessage(HttpStatusCode.NotFound),
            _ => new HttpResponseMessage(HttpStatusCode.NotFound)
        });
        var client = CreateClient(handler);

        var result = await client.TestConnectionAsync(CreateProvider(), "vault-token", TestContext.Current.CancellationToken);

        Assert.True(result.Success);
        Assert.Equal(
            "Vault is reachable. Token lookup is not supported by this provider; test a secret reference to verify token and KV access.",
            result.Message);
    }

    [Fact]
    public async Task TestConnectionAsync_Should_Return_Safe_Failure_When_Token_Is_Rejected()
    {
        var handler = new RecordingHandler(request => request.RequestUri?.AbsolutePath switch
        {
            "/v1/sys/health" => new HttpResponseMessage(HttpStatusCode.OK),
            "/v1/auth/token/lookup-self" => new HttpResponseMessage(HttpStatusCode.Forbidden)
            {
                Content = new StringContent("token vault-token rejected")
            },
            _ => new HttpResponseMessage(HttpStatusCode.NotFound)
        });
        var client = CreateClient(handler);

        var result = await client.TestConnectionAsync(CreateProvider(), "vault-token", TestContext.Current.CancellationToken);

        Assert.False(result.Success);
        Assert.Equal("Vault is reachable but the token was rejected: HTTP 403.", result.Message);
        Assert.DoesNotContain("vault-token", result.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task ResolveAsync_Should_Read_Vault_KvV2_Path_With_Version_And_Token_Header()
    {
        var handler = new RecordingHandler(_ => new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new StringContent(
                """
                {
                  "data": {
                    "data": {
                      "api_key": "super-secret"
                    }
                  }
                }
                """,
                Encoding.UTF8,
                "application/json")
        });
        var client = CreateClient(handler);
        var secret = CreateSecret(externalVersion: 3);
        var provider = CreateProvider();

        var result = await client.ResolveAsync(secret, provider, "vault-token", TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess);
        Assert.Equal("super-secret", result.Value);
        Assert.NotNull(handler.Request);
        Assert.Equal(HttpMethod.Get, handler.Request.Method);
        Assert.Equal("https://vault.local/v1/secret/data/apps/api/prod?version=3", handler.Request.RequestUri?.ToString());
        Assert.True(handler.Request.Headers.TryGetValues("X-Vault-Token", out var tokenValues));
        Assert.Equal("vault-token", Assert.Single(tokenValues));
    }

    [Fact]
    public async Task ResolveAsync_Should_Return_Missing_Key_Failure_Without_Response_Body()
    {
        var handler = new RecordingHandler(_ => new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new StringContent(
                """
                {
                  "data": {
                    "data": {
                      "other_key": "should-not-leak"
                    }
                  }
                }
                """,
                Encoding.UTF8,
                "application/json")
        });
        var client = CreateClient(handler);

        var result = await client.ResolveAsync(
            CreateSecret(),
            CreateProvider(),
            "vault-token",
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess);
        Assert.Equal("Secret API_KEY key 'api_key' was not found in provider vault.", result.ErrorMessage);
        Assert.DoesNotContain("should-not-leak", result.ErrorMessage, StringComparison.Ordinal);
    }

    [Fact]
    public async Task ResolveAsync_Should_Return_Safe_Failure_When_KvV2_Data_Payload_Is_Missing()
    {
        var handler = new RecordingHandler(_ => new HttpResponseMessage(HttpStatusCode.OK)
        {
            Content = new StringContent(
                """
                {
                  "data": null
                }
                """,
                Encoding.UTF8,
                "application/json")
        });
        var client = CreateClient(handler);

        var result = await client.ResolveAsync(
            CreateSecret(),
            CreateProvider(),
            "vault-token",
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess);
        Assert.Equal("Secret API_KEY response from provider vault did not contain a KV v2 data payload.", result.ErrorMessage);
    }

    [Fact]
    public async Task ResolveAsync_Should_Return_Safe_Http_Failure()
    {
        var handler = new RecordingHandler(_ => new HttpResponseMessage(HttpStatusCode.Forbidden)
        {
            Content = new StringContent("token vault-token rejected for super-secret")
        });
        var client = CreateClient(handler);

        var result = await client.ResolveAsync(
            CreateSecret(),
            CreateProvider(),
            "vault-token",
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess);
        Assert.Equal("Secret API_KEY could not be resolved from provider vault: HTTP 403.", result.ErrorMessage);
        Assert.DoesNotContain("vault-token", result.ErrorMessage, StringComparison.Ordinal);
        Assert.DoesNotContain("super-secret", result.ErrorMessage, StringComparison.Ordinal);
    }

    [Fact]
    public async Task ResolveAsync_Should_Return_Actionable_NotFound_Failure()
    {
        var handler = new RecordingHandler(_ => new HttpResponseMessage(HttpStatusCode.NotFound)
        {
            Content = new StringContent("missing secret super-secret")
        });
        var client = CreateClient(handler);

        var result = await client.ResolveAsync(
            CreateSecret(),
            CreateProvider(),
            "vault-token",
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess);
        Assert.Equal(
            "Secret API_KEY was not found in provider vault at KV v2 path 'secret/data/apps/api/prod'. Check that the provider mount path is only the KV engine mount, and the secret path is relative to that mount without '/data'.",
            result.ErrorMessage);
        Assert.DoesNotContain("super-secret", result.ErrorMessage, StringComparison.Ordinal);
        Assert.DoesNotContain("vault-token", result.ErrorMessage, StringComparison.Ordinal);
    }

    private static ExternalSecretProviderClient CreateClient(HttpMessageHandler handler)
    {
        var apiFactory = new TestVaultKvV2ApiFactory(handler);
        return new ExternalSecretProviderClient(apiFactory);
    }

    private static SecretDefinition CreateSecret(int? externalVersion = null)
        => new(
            "API_KEY",
            SecretProviderType.VaultCompatibleKvV2,
            Guid.CreateVersion7(),
            "/apps/api/prod/",
            "api_key",
            externalVersion);

    private static SecretProvider CreateProvider()
        => new(
            "vault",
            SecretProviderType.VaultCompatibleKvV2,
            new VaultKvV2SecretProviderConfiguration("https://vault.local/", "/secret/", "protected-token"));

    private sealed class RecordingHandler(Func<HttpRequestMessage, HttpResponseMessage> respond) : HttpMessageHandler
    {
        public HttpRequestMessage? Request { get; private set; }

        protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request, CancellationToken cancellationToken)
        {
            Request = request;
            var response = respond(request);
            response.RequestMessage = request;
            return Task.FromResult(response);
        }
    }

    private sealed class TestVaultKvV2ApiFactory(HttpMessageHandler handler) : IVaultKvV2ApiFactory
    {
        public IVaultKvV2Api Create(string providerAddress)
        {
            var httpClient = new HttpClient(handler, disposeHandler: false)
            {
                BaseAddress = new Uri(providerAddress.TrimEnd('/'))
            };

            return RestService.ForGenerated<IVaultKvV2Api>(
                httpClient,
                new RefitSettings { ContentSerializer = new STJSourceGeneratorSerializer() });
        }
    }
}

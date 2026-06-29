using System.Net;
using System.Text;
using Domain.Entities.Configuration;
using Infrastructure.Repositories;
using Microsoft.Extensions.Http;
using Moq;

namespace Tests.Unit.Infrastructure.Repositories;

public sealed class ExternalSecretProviderClientTests
{
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

    private static ExternalSecretProviderClient CreateClient(HttpMessageHandler handler)
    {
        var factory = new Mock<IHttpClientFactory>();
        factory
            .Setup(x => x.CreateClient(ExternalSecretProviderClient.HttpClientName))
            .Returns(() => new HttpClient(handler, disposeHandler: false));

        return new ExternalSecretProviderClient(factory.Object);
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
}

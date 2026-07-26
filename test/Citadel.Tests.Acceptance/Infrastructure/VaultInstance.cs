using System.Net.Http.Json;
using DotNet.Testcontainers.Builders;
using DotNet.Testcontainers.Containers;

namespace Tests.Acceptance.Infrastructure;

internal sealed class VaultInstance : IAsyncDisposable
{
    private const ushort HttpPort = 8200;
    private const string Image =
        "hashicorp/vault@sha256:3fd53308acccd9e4e83fde3e6c6cf5b15e0f7fac1ff7510780b8b23f5e4c3de7";

    public const string RootToken = "citadel-vault-acceptance-root-token";
    public const string SecretPath = "citadel/acceptance";
    public const string SecretKey = "api_key";
    public const string SecretValue =
        "citadel-vault-acceptance-value-7b8d6e9c";

    private readonly IContainer container;
    private readonly HttpClient client;

    private VaultInstance(
        IContainer container,
        HttpClient client,
        Uri baseAddress)
    {
        this.container = container;
        this.client = client;
        BaseAddress = baseAddress;
    }

    public Uri BaseAddress { get; }

    public static async Task<VaultInstance> StartAsync(
        CancellationToken cancellationToken)
    {
        var container = new ContainerBuilder(Image)
            .WithPortBinding(HttpPort, assignRandomHostPort: true)
            .WithEnvironment("VAULT_DEV_ROOT_TOKEN_ID", RootToken)
            .WithEnvironment(
                "VAULT_DEV_LISTEN_ADDRESS",
                $"0.0.0.0:{HttpPort}")
            .WithEnvironment("SKIP_SETCAP", "true")
            .WithCommand("server", "-dev")
            .WithWaitStrategy(
                Wait.ForUnixContainer().UntilHttpRequestIsSucceeded(
                    request => request
                        .ForPort(HttpPort)
                        .ForPath("/v1/sys/health")))
            .Build();

        try
        {
            await container.StartAsync(cancellationToken);
            var baseAddress = new Uri(
                $"http://127.0.0.1:{container.GetMappedPublicPort(HttpPort)}");
            var client = new HttpClient
            {
                BaseAddress = baseAddress,
                Timeout = TimeSpan.FromSeconds(30)
            };
            client.DefaultRequestHeaders.Add("X-Vault-Token", RootToken);

            await SeedSecretAsync(client, cancellationToken);
            return new VaultInstance(container, client, baseAddress);
        }
        catch
        {
            await container.DisposeAsync();
            throw;
        }
    }

    public async ValueTask DisposeAsync()
    {
        client.Dispose();
        await container.DisposeAsync();
    }

    private static async Task SeedSecretAsync(
        HttpClient client,
        CancellationToken cancellationToken)
    {
        using var response = await client.PostAsJsonAsync(
            $"/v1/secret/data/{SecretPath}",
            new
            {
                data = new Dictionary<string, string>
                {
                    [SecretKey] = SecretValue,
                    ["secondary"] = "secondary-value"
                }
            },
            cancellationToken);
        var body = await response.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            response.IsSuccessStatusCode,
            $"Seeding the Vault KV v2 fixture failed with HTTP {(int)response.StatusCode}: {body}");
    }
}

using DotNet.Testcontainers.Builders;
using DotNet.Testcontainers.Containers;

namespace Tests.Acceptance.Infrastructure;

internal sealed class KeycloakInstance : IAsyncDisposable
{
    private const ushort HttpPort = 8080;
    private const string Image =
        "quay.io/keycloak/keycloak@sha256:0f198be292568439d700cdbfb893e69a6009bb43a94a06a945b1d3d506c76b13";
    private const string Realm = "citadel-e2e";

    private readonly IContainer container;

    private KeycloakInstance(IContainer container)
    {
        this.container = container;
        BaseAddress = new Uri(
            $"http://127.0.0.1:{container.GetMappedPublicPort(HttpPort)}");
    }

    public Uri BaseAddress { get; }

    public string Issuer =>
        new Uri(BaseAddress, $"realms/{Realm}").ToString().TrimEnd('/');

    public static async Task<KeycloakInstance> StartAsync(
        CancellationToken cancellationToken)
    {
        var realmFile = new FileInfo(
            Path.Combine(
                AppContext.BaseDirectory,
                "Fixtures",
                "Keycloak",
                "citadel-e2e-realm.json"));
        Assert.True(
            realmFile.Exists,
            $"Keycloak realm fixture not found at {realmFile.FullName}.");

        var container = new ContainerBuilder(Image)
            .WithPortBinding(HttpPort, assignRandomHostPort: true)
            .WithEnvironment(
                "KC_BOOTSTRAP_ADMIN_USERNAME",
                "citadel-acceptance-admin")
            .WithEnvironment(
                "KC_BOOTSTRAP_ADMIN_PASSWORD",
                "citadel-acceptance-admin-password")
            .WithEnvironment("KC_HEALTH_ENABLED", "true")
            .WithResourceMapping(
                realmFile,
                "/opt/keycloak/data/import/")
            .WithCommand(
                "start-dev",
                "--import-realm",
                "--hostname-strict=false")
            .WithWaitStrategy(
                Wait.ForUnixContainer().UntilHttpRequestIsSucceeded(
                    request => request
                        .ForPort(HttpPort)
                        .ForPath(
                            $"/realms/{Realm}/.well-known/openid-configuration")))
            .Build();

        try
        {
            await container.StartAsync(cancellationToken);
            return new KeycloakInstance(container);
        }
        catch
        {
            await container.DisposeAsync();
            throw;
        }
    }

    public ValueTask DisposeAsync() => container.DisposeAsync();
}

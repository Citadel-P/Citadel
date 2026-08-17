using System.Reflection;
using System.Text.Json;

namespace Tests.Integration.Application.Features;

public sealed class ApplicationInfoEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task GetApplicationInfo_ReturnsWebApiProductVersion()
    {
        var response = await Client.GetAsync(
            "/api/v1/application/info",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var json = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var expectedInformationalVersion = typeof(global::WebApi.Routes.Endpoints.ApplicationInfo).Assembly
            .GetCustomAttribute<AssemblyInformationalVersionAttribute>()!
            .InformationalVersion;
        var expectedDisplayVersion = expectedInformationalVersion.Split('+', 2)[0];

        Assert.Equal("Citadel", json.RootElement.GetProperty("name").GetString());
        Assert.Equal(expectedDisplayVersion, json.RootElement.GetProperty("version").GetString());
        Assert.Equal(expectedInformationalVersion, json.RootElement.GetProperty("informationalVersion").GetString());
        Assert.Equal("SignalR", json.RootElement.GetProperty("realtimeTransport").GetString());
    }
}

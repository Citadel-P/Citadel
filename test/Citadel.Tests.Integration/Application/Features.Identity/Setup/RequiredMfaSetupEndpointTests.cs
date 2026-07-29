using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Application.Configs;
using Domain;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Setup;

public sealed class RequiredMfaSetupEndpointTests(
    PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    protected override bool SeedDefaultAdministrator => false;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.Configure<MfaOptions>(
            options => options.Policy = MfaPolicy.RequiredForAdministrators);
    }

    [Fact]
    public async Task Initialize_Should_Require_Mfa_When_Administrator_Policy_Is_Enabled()
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/setup/initialize",
            new
            {
                name = "owner",
                email = "owner@example.test",
                password = "correct-horse-battery-staple"
            },
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        await using var stream = await response.Content.ReadAsStreamAsync(
            TestContext.Current.CancellationToken);
        using var document = await JsonDocument.ParseAsync(
            stream,
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(
            "EnrollMfa",
            document.RootElement.GetProperty("nextStep").GetString());
        Assert.False(document.RootElement.TryGetProperty("accessToken", out _));
    }
}

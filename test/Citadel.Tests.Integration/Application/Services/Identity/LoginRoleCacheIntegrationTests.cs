using Application.Services.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.IdentityModel.Tokens.Jwt;
using System.Net.Http;
using System.Text;
using System.Text.Json;

namespace Tests.Integration.Application.Services.Identity;

public class LoginRoleCacheIntegrationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task Login_ShouldPopulateRoleCache_OnSuccessfulLogin()
    {
        // Arrange
        var loginJson = """
        {
          "emailOrName": "admin@citadel.local",
          "password": "admin123"
        }
        """;

        var content = new StringContent(loginJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v1/authentication/login", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        using var doc = JsonDocument.Parse(body);
        var token = doc.RootElement.GetProperty("accessToken").GetString();

        // Extract the actor id used by the server-side role cache.
        var handler = new JwtSecurityTokenHandler();
        var jwt = handler.ReadJwtToken(token!);
        var actorIdClaim = jwt.Claims.FirstOrDefault(c => c.Type == "actorId")?.Value;

        Assert.False(string.IsNullOrEmpty(actorIdClaim));
        var actorId = Guid.Parse(actorIdClaim!);

        // Assert cache
        var roleCache = Services.GetRequiredService<IRoleCache>();
        var roles = roleCache.GetRoles(actorId);
        Assert.NotNull(roles);
        Assert.Contains("admin", roles, StringComparer.OrdinalIgnoreCase);
    }
}

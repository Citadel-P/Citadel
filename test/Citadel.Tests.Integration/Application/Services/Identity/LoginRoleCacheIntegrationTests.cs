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

        // Extract user id from token
        var handler = new JwtSecurityTokenHandler();
        var jwt = handler.ReadJwtToken(token!);
        var sub = jwt.Claims.FirstOrDefault(c => c.Type == JwtRegisteredClaimNames.Sub)?.Value
                  ?? jwt.Claims.FirstOrDefault(c => c.Type == "sub")?.Value;

        Assert.False(string.IsNullOrEmpty(sub));
        var userId = Guid.Parse(sub!);

        // Assert cache
        var roleCache = Services.GetRequiredService<IRoleCache>();
        var roles = roleCache.GetRoles(userId);
        Assert.NotNull(roles);
        Assert.Contains("admin", roles, StringComparer.OrdinalIgnoreCase);
    }
}

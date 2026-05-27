using Application.Services;
using Application.Services.Identity;
using Microsoft.Extensions.DependencyInjection;
using System.Security.Claims;

namespace Tests.Integration.Application.Services.Identity;

public class RoleCacheIntegrationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public void JwtService_CreateAccessToken_ShouldSeedRoleCacheFromClaims()
    {
        var userId = Guid.CreateVersion7();
        var jwt = Services.GetRequiredService<IJwtService>();
        var roleCache = Services.GetRequiredService<IRoleCache>();

        var claims = new[]
        {
            new Claim("sub", userId.ToString()),
            new Claim("role", "admin")
        };

        var token = jwt.CreateAccessToken(claims);

        var roles = roleCache.GetRoles(userId);
        Assert.NotNull(roles);
        Assert.Contains("admin", roles, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task ActorScopeEvictor_EvictUsers_ShouldRemoveRoleCacheEntries()
    {
        var roleCache = Services.GetRequiredService<IRoleCache>();
        var evictor = Services.GetRequiredService<IActorScopeEvictor>();

        var userId = Guid.CreateVersion7();
        roleCache.SetRoles(userId, new[] { "admin" });

        // Ensure present
        Assert.NotNull(roleCache.GetRoles(userId));

        evictor.EvictUsers(new[] { userId }, CancellationToken.None);

        var rolesAfter = roleCache.GetRoles(userId);
        Assert.Null(rolesAfter);
    }
}

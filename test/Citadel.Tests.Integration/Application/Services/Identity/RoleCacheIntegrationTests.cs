using Application.Services;
using Application.Services.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Security.Claims;

namespace Tests.Integration.Application.Services.Identity;

public class RoleCacheIntegrationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid SeededAdminUserId =
        Guid.Parse("10000000-0000-0000-0000-000000000001");

    [Fact]
    public void JwtService_CreateAccessToken_ShouldSeedRoleCacheFromClaims()
    {
        var userId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var jwt = Services.GetRequiredService<IJwtService>();
        var roleCache = Services.GetRequiredService<IRoleCache>();

        var claims = new[]
        {
            new Claim("sub", userId.ToString()),
            new Claim("actorId", actorId.ToString()),
            new Claim("role", "admin")
        };

        var token = jwt.CreateAccessToken(claims);

        var roles = roleCache.GetRoles(actorId);
        Assert.NotNull(roles);
        Assert.Contains("admin", roles, StringComparer.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task ActorScopeEvictor_EvictUsers_ShouldRemoveRoleCacheEntries()
    {
        var roleCache = Services.GetRequiredService<IRoleCache>();
        var evictor = Services.GetRequiredService<IActorScopeEvictor>();

        roleCache.SetRoles(Constants.DefaultAdminId, new[] { "admin" });

        // Ensure present
        Assert.NotNull(roleCache.GetRoles(Constants.DefaultAdminId));

        await evictor.EvictUsers([SeededAdminUserId], TestContext.Current.CancellationToken);

        var rolesAfter = roleCache.GetRoles(Constants.DefaultAdminId);
        Assert.Null(rolesAfter);
    }
}

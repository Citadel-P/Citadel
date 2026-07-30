using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Domain.Entities.Registries;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net;
using System.Net.Http.Headers;

namespace Tests.Integration.Application.Features.Identity.Roles;

public class RoleDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid roleId;
    private Guid registryId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var role = Role.Create("delete-me", RoleType.Custom);
        role.SetPermissions([Permission.Create(role.Id, ResourceType.Registry, PermissionLevel.Read)]);
        await uow.Roles.AddAsync(role, TestContext.Current.CancellationToken);

        var registry = new Registry(
            "delete-role-cache-registry",
            "ghcr.io",
            RegistryStatus.Active,
            Constants.SystemId,
            DockerHubRegistry.Create("test", "token"));
        await uow.Registries.AddAsync(registry, TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
        roleId = role.Id;
        registryId = registry.Id;
    }

    [Fact]
    public async Task Delete_Role_ReturnsSuccess()
    {
        var content = $$"""
        {
            "ids": ["{{roleId}}"]
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/roles")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var roles = await uow.Roles.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.DoesNotContain(roles, x => x.Id == roleId);
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Delete_NonExistent_Role_ReturnsNotFound()
    {
        var content = $$"""
        {
            "ids": ["{{Guid.NewGuid()}}"]
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/roles")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Delete_System_Role_ReturnsConflict()
    {
        var systemRole = await CreateRoleAsync("system-delete", RoleType.System);
        var content = $$"""
        {
            "ids": ["{{systemRole}}"]
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/roles")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
    }

    [Fact]
    public async Task Delete_Role_Evicts_Affected_User_Permission_Cache()
    {
        var subject = await CreateAuthorizationSubjectAsync(directRoleId: roleId);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var warmCacheResponse = await Client.GetAsync(
            $"/api/v1/registries/{registryId}",
            TestContext.Current.CancellationToken);
        warmCacheResponse.EnsureSuccessStatusCode();

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", CreateJwtToken());
        var deleteResponse = await Client.SendAsync(
            CreateDeleteRequest(roleId),
            TestContext.Current.CancellationToken);
        deleteResponse.EnsureSuccessStatusCode();

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));
        var response = await Client.GetAsync(
            $"/api/v1/registries/{registryId}",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Forbidden, response.StatusCode);
    }

    private async Task<Guid> CreateRoleAsync(string name, RoleType roleType)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var role = Role.Create(name, roleType);
        role.SetPermissions([Permission.Create(role.Id, ResourceType.Role, PermissionLevel.Read)]);
        await uow.Roles.AddAsync(role, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return role.Id;
    }

    private static HttpRequestMessage CreateDeleteRequest(Guid id)
    {
        var content = $$"""
        {
            "ids": ["{{id}}"]
        }
        """;

        return new HttpRequestMessage(HttpMethod.Delete, "/api/v1/roles")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };
    }
}

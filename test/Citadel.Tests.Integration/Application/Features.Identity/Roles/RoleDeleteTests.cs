using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Roles;

public class RoleDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid roleId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var role = Role.Create("delete-me", RoleType.Custom);
        role.SetPermissions([Permission.Create(role.Id, ResourceType.Role, ResourceAction.View)]);
        await uow.Roles.AddAsync(role, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        roleId = role.Id;
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

    private async Task<Guid> CreateRoleAsync(string name, RoleType roleType)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var role = Role.Create(name, roleType);
        role.SetPermissions([Permission.Create(role.Id, ResourceType.Role, ResourceAction.View)]);
        await uow.Roles.AddAsync(role, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return role.Id;
    }
}

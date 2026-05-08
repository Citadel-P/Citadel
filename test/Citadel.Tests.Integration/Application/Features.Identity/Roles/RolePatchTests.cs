using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Roles;

public class RolePatchTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid roleId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var role = Role.Create("OriginalRole", RoleType.Custom);
        role.SetPermissions([Permission.Create(role.Id, ResourceType.Registry, PermissionLevel.Read)]);
        await uow.Roles.AddAsync(role, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        roleId = role.Id;
    }

    [Fact]
    public async Task Patch_Role_Permissions_Should_Apply_MergePatch()
    {
        var patchJson = """
        {
          "permissions": [
            {
              "resourceType": "Role",
              "permissionLevel": "Write",
              "specificPermissions": []
            },
            {
              "resourceType": "Registry",
              "permissionLevel": "Execute",
              "specificPermissions": []
            }
          ]
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/roles/{roleId}/permissions", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var role = await uow.Roles.GetAsync(roleId, TestContext.Current.CancellationToken);

        Assert.NotNull(role);
        Assert.Equal(2, role.Permissions.Count());
        Assert.Contains(role.Permissions, x => x.ResourceType == ResourceType.Role && x.PermissionLevel == PermissionLevel.Write);
        Assert.Contains(role.Permissions, x => x.ResourceType == ResourceType.Registry && x.PermissionLevel == PermissionLevel.Execute);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_Role_Permissions_With_Invalid_Data_Should_Return_BadRequest()
    {
        var patchJson = """
        {
          "permissions": [
            {
              "resourceType": "Role",
              "permissionLevel": "Nope",
              "specificPermissions": []
            }
          ]
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/roles/{roleId}/permissions", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Patch_NonExistent_Role_Should_Return_NotFound()
    {
        var nonExistentId = Guid.NewGuid();
        var patchJson = """
        {
          "permissions": [
            {
              "resourceType": "Role",
              "permissionLevel": "Read",
              "specificPermissions": []
            }
          ]
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/roles/{nonExistentId}/permissions", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Rename_Role_Should_Update_Name()
    {
        var renameJson = $$"""
        {
          "id": "{{roleId}}",
          "name": "RenamedRole"
        }
        """;
        var content = new StringContent(renameJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/roles/rename", content, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var role = await uow.Roles.GetAsync(roleId, TestContext.Current.CancellationToken);

        Assert.Equal("RenamedRole", role?.Name);
    }

    [Fact]
    public async Task Patch_System_Role_Permissions_Should_Return_Conflict()
    {
        var systemRoleId = await CreateRoleAsync("system-patch", RoleType.System);
        var patchJson = """
        {
          "permissions": [
            {
              "resourceType": "Role",
              "permissionLevel": "Write",
              "specificPermissions": []
            }
          ]
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/roles/{systemRoleId}/permissions", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
    }

    [Fact]
    public async Task Rename_System_Role_Should_Return_Conflict()
    {
        var systemRoleId = await CreateRoleAsync("system-rename", RoleType.System);
        var renameJson = $$"""
        {
          "id": "{{systemRoleId}}",
          "name": "RenamedSystemRole"
        }
        """;
        var content = new StringContent(renameJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/roles/rename", content, TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
    }

    private async Task<Guid> CreateRoleAsync(string name, RoleType roleType)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var role = Role.Create(name, roleType);
        role.SetPermissions([Permission.Create(role.Id, ResourceType.Registry, PermissionLevel.Read)]);
        await uow.Roles.AddAsync(role, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return role.Id;
    }
}

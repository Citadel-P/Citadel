using Application.Services.Licensing;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;
using System.Text.Json;
using Tests.Common;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Identity.Roles;

public class RoleViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.ReplaceService<ILicenseEntitlementService>(new PermissiveLicenseEntitlementService());
    }

    [Fact]
    public async Task List_Roles_Should_Return_All_Roles_For_Admin()
    {
        var firstRoleId = await CreateRoleAsync("role-admin-1", [(ResourceType.Role, PermissionLevel.Read)]);
        var secondRoleId = await CreateRoleAsync("role-admin-2", [(ResourceType.Registry, PermissionLevel.Read)]);

        var response = await Client.GetAsync("/api/v1/roles", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.GetProperty("roles");
        Assert.True(items.GetArrayLength() >= 2);
        Assert.Contains(items.EnumerateArray(), x => x.GetProperty("id").GetGuid() == firstRoleId);
        Assert.Contains(items.EnumerateArray(), x => x.GetProperty("id").GetGuid() == secondRoleId);
    }

    [Fact]
    public async Task Get_Role_Should_Return_Forbidden_For_NonAdministrator_With_Resource_Access()
    {
        var roleId = await CreateRoleAsync("role-readable", [(ResourceType.Role, PermissionLevel.Read)]);
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Role, roleId, PermissionLevel.Read)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/roles/{roleId}", TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Forbidden, response.StatusCode);
    }

    [Fact]
    public async Task List_Roles_Should_Return_Forbidden_When_User_Has_Only_Resource_Level_Access()
    {
        var visibleRoleId = await CreateRoleAsync("role-visible", [(ResourceType.Role, PermissionLevel.Read)]);
        await CreateRoleAsync("role-hidden", [(ResourceType.Registry, PermissionLevel.Read)]);

        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Role, visibleRoleId, PermissionLevel.Read)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/roles", TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.Forbidden, response.StatusCode);
    }

    [Fact]
    public async Task Get_Role_Should_Return_Forbidden_When_User_Has_No_Rights_To_View_It()
    {
        var roleId = await CreateRoleAsync("role-protected", [(ResourceType.Role, PermissionLevel.Read)]);
        var subject = await CreateAuthorizationSubjectAsync();

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/roles/{roleId}", TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Forbidden, response.StatusCode);
    }

    private async Task<Guid> CreateRoleAsync(string name, IEnumerable<(ResourceType ResourceType, PermissionLevel PermissionLevel)> permissions)
    {
        var createJson = $$"""
        {
          "name": "{{name}}",
          "permissions": [
            {{string.Join(",", permissions.Select(x => $$"""
            {
              "resourceType": "{{x.ResourceType}}",
              "permissionLevel": "{{x.PermissionLevel}}"
            }
            """))}}
          ]
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/roles",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await uow.Roles.GetAllAsync(TestContext.Current.CancellationToken))
            .Single(x => x.Name == name)
            .Id;
    }
}

using System.Text;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Roles;

public class RoleCreateTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task Create_Role_ReturnsSuccess()
    {
        var createJson = """
        {
          "name": "Role-New",
          "permissions": [
            {
              "resourceType": "Registry",
              "resourceAction": "View"
            },
            {
              "resourceType": "Role",
              "resourceAction": "Update"
            }
          ]
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/roles", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var role = (await uow.Roles.GetAllAsync(TestContext.Current.CancellationToken)).Single(x => x.Name == "Role-New");

        Assert.Equal(2, role.Permissions.Count());
        Assert.Contains(role.Permissions, x => x.ResourceType == ResourceType.Registry && x.ResourceAction == ResourceAction.View);
        Assert.Contains(role.Permissions, x => x.ResourceType == ResourceType.Role && x.ResourceAction == ResourceAction.Update);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_Role_With_Empty_Name_Returns_BadRequest()
    {
        var createJson = """
        {
          "name": "",
          "permissions": [
            {
              "resourceType": "Registry",
              "resourceAction": "View"
            }
          ]
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/roles", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Create_Role_With_Invalid_Permission_Returns_BadRequest()
    {
        var createJson = """
        {
          "name": "Role-New",
          "permissions": [
            {
              "resourceType": "Registry",
              "resourceAction": "Nope"
            }
          ]
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/roles", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Create_Role_With_Duplicate_Name_Returns_Conflict()
    {
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var role = Role.Create("RoleNew", [Permission.Create(Guid.Empty, ResourceType.Registry, ResourceAction.View)]);
            await uow.Roles.AddAsync(role, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var createJson = """
        {
          "name": "RoleNew",
          "permissions": [
            {
              "resourceType": "Role",
              "resourceAction": "View"
            }
          ]
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/roles", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
    }
}

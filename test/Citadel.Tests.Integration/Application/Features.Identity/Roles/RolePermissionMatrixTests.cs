using Application.Services.Licensing;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Tests.Common;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Identity.Roles;

public class RolePermissionMatrixTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.ReplaceService<ILicenseEntitlementService>(new PermissiveLicenseEntitlementService());
    }

    [Fact]
    public async Task Get_Permission_Matrix_Returns_Ok()
    {
        var response = await Client.GetAsync(
            "/api/v1/roles/permissions/matrix",
            TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.OK, response.StatusCode);
    }

    [Fact]
    public async Task Get_Permission_Matrix_Is_Accessible_Without_Authentication()
    {
        // Remove auth header for this request only
        Client.DefaultRequestHeaders.Authorization = null;

        var response = await Client.GetAsync(
            "/api/v1/roles/permissions/matrix",
            TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.OK, response.StatusCode);

        // Restore for any follow-up tests
        Client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", CreateJwtToken());
    }

    [Fact]
    public async Task Get_Permission_Matrix_Contains_Known_Resource_Types()
    {
        var response = await Client.GetAsync(
            "/api/v1/roles/permissions/matrix",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var root = doc.RootElement;

        // Every ResourceType in the matrix must be present
        foreach (var resourceType in new[]
        {
            nameof(ResourceType.User),
            nameof(ResourceType.Team),
            nameof(ResourceType.Role),
            nameof(ResourceType.Platform),
            nameof(ResourceType.Deployment),
            nameof(ResourceType.SwarmService),
            nameof(ResourceType.Stack),
            nameof(ResourceType.GitRepository),
            nameof(ResourceType.GitAccount),
            nameof(ResourceType.Registry),
            nameof(ResourceType.Alert),
            nameof(ResourceType.AlertChannel),
            nameof(ResourceType.Binding),
            nameof(ResourceType.Tag),
        })
        {
            Assert.True(root.TryGetProperty(resourceType, out _), $"Missing resource type: {resourceType}");
        }
    }

    [Fact]
    public async Task Get_Permission_Matrix_SwarmService_Contains_Operation_Capabilities()
    {
        var response = await Client.GetAsync(
            "/api/v1/roles/permissions/matrix",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var service = doc.RootElement.GetProperty(nameof(ResourceType.SwarmService));
        var specifics = service.GetProperty("specificPermissions");

        Assert.Equal(nameof(PermissionLevel.Execute), service.GetProperty("maximumLevel").GetString());
        Assert.Equal(nameof(PermissionLevel.Read), specifics.GetProperty(nameof(SpecificPermission.Apply)).GetString());
        Assert.Equal(nameof(PermissionLevel.Read), specifics.GetProperty(nameof(SpecificPermission.Logs)).GetString());
        Assert.Equal(nameof(PermissionLevel.Read), specifics.GetProperty(nameof(SpecificPermission.Inspect)).GetString());
    }

    [Fact]
    public async Task Get_Permission_Matrix_Deployment_Contains_Maximum_Level_And_Specific_Permissions()
    {
        var response = await Client.GetAsync(
            "/api/v1/roles/permissions/matrix",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var deployment = doc.RootElement.GetProperty(nameof(ResourceType.Deployment));
        var maximumLevel = deployment.GetProperty("maximumLevel").GetString();
        var specifics = deployment.GetProperty("specificPermissions");

        Assert.Equal(nameof(PermissionLevel.Execute), maximumLevel);
        Assert.Equal(nameof(PermissionLevel.Read), specifics.GetProperty(nameof(SpecificPermission.Apply)).GetString());
        Assert.Equal(nameof(PermissionLevel.Read), specifics.GetProperty(nameof(SpecificPermission.Logs)).GetString());
    }

    [Fact]
    public async Task Get_Permission_Matrix_Platform_Contains_Pull_Terminal_And_Logs_Minimum_Levels()
    {
        var response = await Client.GetAsync(
            "/api/v1/roles/permissions/matrix",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var platform = doc.RootElement.GetProperty(nameof(ResourceType.Platform));
        var specifics = platform.GetProperty("specificPermissions");

        Assert.Equal(nameof(PermissionLevel.Read), specifics.GetProperty(nameof(SpecificPermission.Inspect)).GetString());
        Assert.Equal(nameof(PermissionLevel.Read), specifics.GetProperty(nameof(SpecificPermission.Pull)).GetString());
        Assert.Equal(nameof(PermissionLevel.Read), specifics.GetProperty(nameof(SpecificPermission.Terminal)).GetString());
        Assert.Equal(nameof(PermissionLevel.Read), specifics.GetProperty(nameof(SpecificPermission.Logs)).GetString());
    }

    [Fact]
    public async Task Get_Permission_Matrix_User_Does_Not_Contain_Specific_Permissions()
    {
        var response = await Client.GetAsync(
            "/api/v1/roles/permissions/matrix",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var user = doc.RootElement.GetProperty(nameof(ResourceType.User));
        var specifics = user.GetProperty("specificPermissions");

        Assert.Equal(JsonValueKind.Object, specifics.ValueKind);
        Assert.False(specifics.EnumerateObject().Any());
    }

    // Matrix-based validation at role creation 

    [Fact]
    public async Task Create_Role_With_Disallowed_Permission_Returns_BadRequest()
    {
        // SpecificPermission.Logs is valid globally but is NOT allowed for ResourceType.User
        var createJson = """
        {
          "name": "Role-Invalid-Matrix",
          "permissions": [
            {
              "resourceType": "User",
              "permissionLevel": "Read",
              "specificPermissions": ["Logs"]
            }
          ]
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync(
            "/api/v1/roles",
            content,
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Create_Role_With_Disallowed_Permission_Response_Contains_Validation_Error()
    {
        var createJson = """
        {
          "name": "Role-Invalid-Matrix-2",
          "permissions": [
            {
              "resourceType": "Team",
              "permissionLevel": "Execute",
              "specificPermissions": ["Apply"]
            }
          ]
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync(
            "/api/v1/roles",
            content,
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);

        var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("Team", body, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("Apply", body, StringComparison.OrdinalIgnoreCase);
    }

    // Matrix-based validation at role permission patch 

    [Fact]
    public async Task Patch_Role_Permissions_With_Disallowed_Permission_Returns_BadRequest()
    {
        // Create a valid role first
        var createJson = """
        {
          "name": "Role-Patch-Matrix",
          "permissions": [
            {
              "resourceType": "Registry",
              "permissionLevel": "Read",
              "specificPermissions": []
            }
          ]
        }
        """;
        var createResponse = await Client.PostAsync(
            "/api/v1/roles",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await createResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        var roleId = doc.RootElement.GetProperty("id").GetGuid();

        // SpecificPermission.Logs is valid globally but not allowed for ResourceType.Role
        var patchJson = """
        {
          "permissions": [
            {
              "resourceType": "Role",
              "permissionLevel": "Read",
              "specificPermissions": ["Logs"]
            }
          ]
        }
        """;
        var response = await Client.PatchAsync(
            $"/api/v1/roles/{roleId}/permissions",
            new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json"),
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
    }
}

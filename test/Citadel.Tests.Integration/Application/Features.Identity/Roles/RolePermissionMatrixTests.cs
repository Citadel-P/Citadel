using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Hosting.Common;

namespace Tests.Integration.Application.Features.Identity.Roles;

public class RolePermissionMatrixTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
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
            nameof(ResourceType.Stack),
            nameof(ResourceType.GitRepository),
            nameof(ResourceType.GitAccount),
            nameof(ResourceType.Registry),
            nameof(ResourceType.Alert),
            nameof(ResourceType.AlertChannel),
        })
        {
            Assert.True(root.TryGetProperty(resourceType, out _), $"Missing resource type: {resourceType}");
        }
    }

    [Fact]
    public async Task Get_Permission_Matrix_Deployment_Contains_Apply_And_Log()
    {
        var response = await Client.GetAsync(
            "/api/v1/roles/permissions/matrix",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var deployment = doc.RootElement.GetProperty(nameof(ResourceType.Deployment));
        var actions = deployment.EnumerateArray().Select(x => x.GetString()).ToArray();

        Assert.Contains(nameof(ResourceAction.Apply), actions);
        Assert.Contains(nameof(ResourceAction.Log), actions);
    }

    [Fact]
    public async Task Get_Permission_Matrix_Platform_Contains_Pull_Exec_Log()
    {
        var response = await Client.GetAsync(
            "/api/v1/roles/permissions/matrix",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var platform = doc.RootElement.GetProperty(nameof(ResourceType.Platform));
        var actions = platform.EnumerateArray().Select(x => x.GetString()).ToArray();

        Assert.Contains(nameof(ResourceAction.Pull), actions);
        Assert.Contains(nameof(ResourceAction.Exec), actions);
        Assert.Contains(nameof(ResourceAction.Log), actions);
    }

    [Fact]
    public async Task Get_Permission_Matrix_User_Does_Not_Contain_Apply_Or_Log()
    {
        var response = await Client.GetAsync(
            "/api/v1/roles/permissions/matrix",
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using var doc = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var user = doc.RootElement.GetProperty(nameof(ResourceType.User));
        var actions = user.EnumerateArray().Select(x => x.GetString()).ToArray();

        Assert.DoesNotContain(nameof(ResourceAction.Apply), actions);
        Assert.DoesNotContain(nameof(ResourceAction.Log), actions);
        Assert.DoesNotContain(nameof(ResourceAction.Pull), actions);
        Assert.DoesNotContain(nameof(ResourceAction.Exec), actions);
    }

    // ── Matrix-based validation at role creation ──────────────────────────────

    [Fact]
    public async Task Create_Role_With_Disallowed_Permission_Returns_BadRequest()
    {
        // ResourceAction.Log is a valid enum value but is NOT allowed for ResourceType.User
        var createJson = """
        {
          "name": "Role-Invalid-Matrix",
          "permissions": [
            {
              "resourceType": "User",
              "resourceAction": "Log"
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
              "resourceAction": "Apply"
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

    // ── Matrix-based validation at role permission patch ──────────────────────

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
              "resourceAction": "View"
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

        // ResourceAction.Log is valid enum but not allowed for ResourceType.Role
        var patchJson = """
        {
          "permissions": [
            {
              "resourceType": "Role",
              "resourceAction": "Log"
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

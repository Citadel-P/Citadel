using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net.Http.Json;
using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.ResourceBindings;

public sealed class ResourceBindingsTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid _stackId;
    private Guid _deploymentId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var stack = Stack.Create(
            "stack-config",
            Constants.SystemId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack("services:\n  app:\n    image: nginx", StackUpdateBehavior.Notify));
        var deployment = new Deployment(
            "deployment-config",
            Constants.SystemId,
            platform.Id);

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _stackId = stack.Id;
        _deploymentId = deployment.Id;
    }

    [Fact]
    public async Task ResourceBinding_Should_Save_Overrides_And_Not_Return_Secret_Value()
    {
        var createSecretResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new { name = "API_KEY", value = "super-secret-value" },
            cancellationToken: TestContext.Current.CancellationToken);
        createSecretResponse.EnsureSuccessStatusCode();

        var secretJson = await JsonDocument.ParseAsync(
            await createSecretResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        var secretId = secretJson.RootElement.GetProperty("id").GetGuid();

        var globalResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/global",
            new { name = "SHARED_VALUE", kind = "Variable", value = "global", secretId = (Guid?)null, secretDeliveryMode = (string?)null, targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        globalResponse.EnsureSuccessStatusCode();

        var variableResponse = await Client.PostAsJsonAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            new { name = "SHARED_VALUE", kind = "Variable", value = "stack", secretId = (Guid?)null, secretDeliveryMode = (string?)null, targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        variableResponse.EnsureSuccessStatusCode();

        var secretResponse = await Client.PostAsJsonAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            new { name = "API_KEY", kind = "Secret", value = (string?)null, secretId, secretDeliveryMode = "EnvironmentVariable", targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        secretResponse.EnsureSuccessStatusCode();

        var getResponse = await Client.GetAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            TestContext.Current.CancellationToken);
        getResponse.EnsureSuccessStatusCode();

        var responseBody = await getResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("\"name\":\"SHARED_VALUE\"", responseBody);
        Assert.Contains("\"value\":\"stack\"", responseBody);
        Assert.Contains("\"name\":\"API_KEY\"", responseBody);
        Assert.DoesNotContain("super-secret-value", responseBody);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var storedSecret = await uow.SecretDefinitions.GetInternalValueAsync(secretId, TestContext.Current.CancellationToken);

        Assert.NotNull(storedSecret);
        Assert.NotEqual("super-secret-value", storedSecret.EncryptedValue);
    }

    [Fact]
    public async Task Post_GlobalResourceBinding_Should_Create_Single_Entry()
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/global",
            new { name = "POSTED_GLOBAL", kind = "Variable", value = "global-value", secretId = (Guid?)null },
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("\"name\":\"POSTED_GLOBAL\"", responseBody);
        Assert.Contains("\"value\":\"global-value\"", responseBody);
    }

    [Fact]
    public async Task Post_ResourceBinding_Should_Create_Single_Entry()
    {
        var response = await Client.PostAsJsonAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            new { name = "POSTED_STACK", kind = "Variable", value = "stack-value", secretId = (Guid?)null },
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("\"name\":\"POSTED_STACK\"", responseBody);
        Assert.Contains("\"value\":\"stack-value\"", responseBody);
    }

    [Fact]
    public async Task Patch_GlobalResourceBinding_Should_Update_Single_Entry()
    {
        var createResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/global",
            new { name = "PATCHED_GLOBAL", kind = "Variable", value = "before", secretId = (Guid?)null, secretDeliveryMode = (string?)null, targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();

        var id = await ReadEntryIdAsync(createResponse);

        var patchResponse = await Client.PatchAsJsonAsync(
            "/api/v1/resourceBindings/global",
            new { id, name = "PATCHED_GLOBAL", kind = "Variable", value = "after", secretId = (Guid?)null, secretDeliveryMode = (string?)null, targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        patchResponse.EnsureSuccessStatusCode();

        var responseBody = await patchResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("\"name\":\"PATCHED_GLOBAL\"", responseBody);
        Assert.Contains("\"value\":\"after\"", responseBody);
    }

    [Fact]
    public async Task Patch_ResourceBinding_Should_Update_Single_Entry()
    {
        var createResponse = await Client.PostAsJsonAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            new { name = "PATCHED_STACK", kind = "Variable", value = "before", secretId = (Guid?)null, secretDeliveryMode = (string?)null, targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();

        var id = await ReadEntryIdAsync(createResponse);

        var patchResponse = await Client.PatchAsJsonAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            new { id, name = "PATCHED_STACK", kind = "Variable", value = "after", secretId = (Guid?)null, secretDeliveryMode = (string?)null, targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        patchResponse.EnsureSuccessStatusCode();

        var responseBody = await patchResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("\"name\":\"PATCHED_STACK\"", responseBody);
        Assert.Contains("\"value\":\"after\"", responseBody);
    }

    [Fact]
    public async Task SecretDefinitions_Should_Return_Only_Secrets_Bound_To_Requested_Scope()
    {
        var globalSecretId = await CreateInternalSecretAsync("GLOBAL_ONLY_SECRET");
        var stackSecretId = await CreateInternalSecretAsync("STACK_ONLY_SECRET");

        var globalResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/global",
            new { name = "GLOBAL_ONLY_SECRET", kind = "Secret", value = (string?)null, secretId = globalSecretId, secretDeliveryMode = "EnvironmentVariable", targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        globalResponse.EnsureSuccessStatusCode();

        var stackResponse = await Client.PostAsJsonAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            new { name = "STACK_ONLY_SECRET", kind = "Secret", value = (string?)null, secretId = stackSecretId, secretDeliveryMode = "EnvironmentVariable", targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        stackResponse.EnsureSuccessStatusCode();

        var globalListResponse = await Client.GetAsync(
            "/api/v1/resourceBindings/secrets",
            TestContext.Current.CancellationToken);
        globalListResponse.EnsureSuccessStatusCode();
        var globalList = await globalListResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.Contains("GLOBAL_ONLY_SECRET", globalList);
        Assert.DoesNotContain("STACK_ONLY_SECRET", globalList);

        var stackListResponse = await Client.GetAsync(
            $"/api/v1/resourceBindings/secrets?scope=Stack&resourceId={_stackId}",
            TestContext.Current.CancellationToken);
        stackListResponse.EnsureSuccessStatusCode();
        var stackList = await stackListResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        Assert.Contains("STACK_ONLY_SECRET", stackList);
        Assert.DoesNotContain("GLOBAL_ONLY_SECRET", stackList);
    }

    [Fact]
    public async Task SecretDefinitions_ForConsumer_ShouldUseConsumerPermissionWithoutGrantingBindingAccess()
    {
        var secretId = await CreateInternalSecretAsync("CONSUMER_SECRET");
        var bindingResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/global",
            new
            {
                name = "CONSUMER_SECRET",
                kind = "Secret",
                value = (string?)null,
                secretId,
                secretDeliveryMode = "EnvironmentVariable",
                targetPath = (string?)null
            },
            cancellationToken: TestContext.Current.CancellationToken);
        bindingResponse.EnsureSuccessStatusCode();

        var subject = await CreateAuthorizationSubjectAsync(directRoleId: OperatorRoleId);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var globalResponse = await Client.GetAsync(
            "/api/v1/resourceBindings/secrets",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, globalResponse.StatusCode);

        foreach (var targetResourceType in new[] { ResourceType.Build, ResourceType.BackupRepository })
        {
            var response = await Client.GetAsync(
                $"/api/v1/resourceBindings/secrets?targetResourceType={targetResourceType}",
                TestContext.Current.CancellationToken);
            var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

            Assert.True(response.IsSuccessStatusCode, body);
            Assert.Contains("CONSUMER_SECRET", body);
            Assert.Contains("\"canRead\":false", body);
            Assert.Contains("\"canWrite\":false", body);
        }
    }

    [Fact]
    public async Task Operator_Should_Create_Internal_Secrets_For_Stack_And_Deployment_Bindings()
    {
        var subject = await CreateAuthorizationSubjectAsync(directRoleId: OperatorRoleId);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var globalResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new { name = "OPERATOR_GLOBAL_SECRET", value = "secret" },
            cancellationToken: TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, globalResponse.StatusCode);

        foreach (var (scope, resourceId, name) in new[]
        {
            (ResourceBindingScope.Stack, _stackId, "OPERATOR_STACK_SECRET"),
            (ResourceBindingScope.Deployment, _deploymentId, "OPERATOR_DEPLOYMENT_SECRET")
        })
        {
            var response = await Client.PostAsJsonAsync(
                $"/api/v1/resourceBindings/secrets?scope={scope}&resourceId={resourceId}",
                new { name, value = "secret" },
                cancellationToken: TestContext.Current.CancellationToken);
            var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

            Assert.True(response.IsSuccessStatusCode, body);
            Assert.Contains($"\"name\":\"{name}\"", body);
        }
    }

    [Fact]
    public async Task Delete_SecretBinding_Should_Delete_Orphaned_SecretDefinition()
    {
        var secretId = await CreateInternalSecretAsync("DELETE_ME_SECRET");
        var createResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/global",
            new { name = "DELETE_ME_SECRET", kind = "Secret", value = (string?)null, secretId, secretDeliveryMode = "EnvironmentVariable", targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();
        var entryId = await ReadEntryIdAsync(createResponse);

        var deleteResponse = await Client.DeleteAsync(
            $"/api/v1/resourceBindings/global/{entryId}",
            TestContext.Current.CancellationToken);
        deleteResponse.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deletedSecret = await uow.SecretDefinitions.GetAsync(secretId, TestContext.Current.CancellationToken);

        Assert.Null(deletedSecret);
    }

    [Fact]
    public async Task Delete_SecretBinding_Should_Keep_SecretDefinition_When_Still_Bound()
    {
        var secretId = await CreateInternalSecretAsync("SHARED_SECRET");
        var globalResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/global",
            new { name = "SHARED_SECRET", kind = "Secret", value = (string?)null, secretId, secretDeliveryMode = "EnvironmentVariable", targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        globalResponse.EnsureSuccessStatusCode();
        var globalEntryId = await ReadEntryIdAsync(globalResponse);

        var stackResponse = await Client.PostAsJsonAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            new { name = "SHARED_SECRET", kind = "Secret", value = (string?)null, secretId, secretDeliveryMode = "EnvironmentVariable", targetPath = (string?)null },
            cancellationToken: TestContext.Current.CancellationToken);
        stackResponse.EnsureSuccessStatusCode();

        var deleteResponse = await Client.DeleteAsync(
            $"/api/v1/resourceBindings/global/{globalEntryId}",
            TestContext.Current.CancellationToken);
        deleteResponse.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var keptSecret = await uow.SecretDefinitions.GetAsync(secretId, TestContext.Current.CancellationToken);

        Assert.NotNull(keptSecret);
    }

    [Fact]
    public async Task Stack_ResourceBinding_Should_Save_Mounted_File_Secret_Delivery()
    {
        var createSecretResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new { name = "FILE_SECRET", value = "super-secret-value" },
            cancellationToken: TestContext.Current.CancellationToken);
        createSecretResponse.EnsureSuccessStatusCode();

        var secretJson = await JsonDocument.ParseAsync(
            await createSecretResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        var secretId = secretJson.RootElement.GetProperty("id").GetGuid();

        var createBindingResponse = await Client.PostAsJsonAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            new { name = "FILE_SECRET", kind = "Secret", value = (string?)null, secretId, secretDeliveryMode = "MountedFile", targetPath = "/run/secrets/file_secret" },
            cancellationToken: TestContext.Current.CancellationToken);

        createBindingResponse.EnsureSuccessStatusCode();

        var getResponse = await Client.GetAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            TestContext.Current.CancellationToken);
        getResponse.EnsureSuccessStatusCode();

        var responseBody = await getResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("\"secretDeliveryMode\":\"MountedFile\"", responseBody);
        Assert.Contains("\"targetPath\":\"/run/secrets/file_secret\"", responseBody);
        Assert.DoesNotContain("super-secret-value", responseBody);
    }

    [Fact]
    public async Task Global_ResourceBinding_Should_Reject_Mounted_File_Secret_Delivery()
    {
        var createSecretResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new { name = "FILE_SECRET", value = "super-secret-value" },
            cancellationToken: TestContext.Current.CancellationToken);
        createSecretResponse.EnsureSuccessStatusCode();

        var secretJson = await JsonDocument.ParseAsync(
            await createSecretResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        var secretId = secretJson.RootElement.GetProperty("id").GetGuid();

        var createBindingResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/global",
            new { name = "FILE_SECRET", kind = "Secret", value = (string?)null, secretId, secretDeliveryMode = "MountedFile", targetPath = "/run/secrets/file_secret" },
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, createBindingResponse.StatusCode);
        var responseBody = await createBindingResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("Only environment variable secret delivery is supported", responseBody);
    }

    [Fact]
    public async Task Patch_SecretProvider_Should_Preserve_Omitted_Fields()
    {
        var providerId = await CreateVaultProviderAsync("vault-a", "http://vault:8200", "secret");

        var patchResponse = await Client.PatchAsync(
            $"/api/v1/resourceBindings/secret-providers/vault-kv2/{providerId}",
            MergePatchContent("""{ "name": "vault-b" }"""),
            TestContext.Current.CancellationToken);
        patchResponse.EnsureSuccessStatusCode();

        var listResponse = await Client.GetAsync(
            "/api/v1/resourceBindings/secret-providers",
            TestContext.Current.CancellationToken);
        listResponse.EnsureSuccessStatusCode();

        var responseBody = await listResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("\"name\":\"vault-b\"", responseBody);
        Assert.Contains("\"address\":\"http://vault:8200\"", responseBody);
        Assert.Contains("\"mountPath\":\"secret\"", responseBody);
    }

    [Fact]
    public async Task Patch_ExternalSecret_Should_Preserve_Omitted_Version()
    {
        var providerId = await CreateVaultProviderAsync("vault-versioned", "http://vault:8200", "secret");

        var createResponse = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets/external",
            new
            {
                name = "VERSIONED_SECRET",
                providerId,
                externalPath = "apps/api/prod",
                externalKey = "api_key",
                externalVersion = 7
            },
            cancellationToken: TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();

        var secretJson = await JsonDocument.ParseAsync(
            await createResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        var secretId = secretJson.RootElement.GetProperty("id").GetGuid();

        var patchResponse = await Client.PatchAsync(
            $"/api/v1/resourceBindings/secrets/external/{secretId}",
            MergePatchContent("""{ "name": "VERSIONED_SECRET_RENAMED" }"""),
            TestContext.Current.CancellationToken);
        patchResponse.EnsureSuccessStatusCode();

        var patchedJson = await JsonDocument.ParseAsync(
            await patchResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal("VERSIONED_SECRET_RENAMED", patchedJson.RootElement.GetProperty("name").GetString());
        Assert.Equal(7, patchedJson.RootElement.GetProperty("externalVersion").GetInt32());
        Assert.Equal("apps/api/prod", patchedJson.RootElement.GetProperty("externalPath").GetString());
        Assert.Equal("api_key", patchedJson.RootElement.GetProperty("externalKey").GetString());
    }

    private async Task<Guid> CreateVaultProviderAsync(string name, string address, string mountPath)
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secret-providers/vault-kv2",
            new { name, address, mountPath, token = "root" },
            cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        var json = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        return json.RootElement.GetProperty("id").GetGuid();
    }

    private async Task<Guid> CreateInternalSecretAsync(string name)
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/resourceBindings/secrets",
            new { name, value = "super-secret-value" },
            cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        var json = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        return json.RootElement.GetProperty("id").GetGuid();
    }

    private static async Task<Guid> ReadEntryIdAsync(HttpResponseMessage response)
    {
        var json = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        return json.RootElement.GetProperty("entries")[0].GetProperty("id").GetGuid();
    }

    private static StringContent MergePatchContent(string json) => new(json, Encoding.UTF8, "application/merge-patch+json");

    [Fact]
    public async Task ResourceBinding_Should_Return_Forbidden_Without_Target_Resource_Permission()
    {
        var subject = await CreateAuthorizationSubjectAsync();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Forbidden, response.StatusCode);

        var createSecretResponse = await Client.PostAsJsonAsync(
            $"/api/v1/resourceBindings/secrets?scope=Stack&resourceId={_stackId}",
            new { name = "UNAUTHORIZED_STACK_SECRET", value = "secret" },
            cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Forbidden, createSecretResponse.StatusCode);
    }

    [Fact]
    public async Task Secret_Definitions_Should_Return_Forbidden_Without_Configuration_Permission()
    {
        var subject = await CreateAuthorizationSubjectAsync();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync(
            "/api/v1/resourceBindings/secrets",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Forbidden, response.StatusCode);
    }

    [Fact]
    public async Task Scoped_Secret_Definitions_Should_Use_Target_Resource_Configuration_Permission()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Stack,
                    _stackId,
                    PermissionLevel.Read,
                    SpecificPermission.ResourceBindings)
            ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var scopedResponse = await Client.GetAsync(
            $"/api/v1/resourceBindings/secrets?scope=Stack&resourceId={_stackId}",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.OK, scopedResponse.StatusCode);

        var globalResponse = await Client.GetAsync(
            "/api/v1/resourceBindings/secrets",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Forbidden, globalResponse.StatusCode);
    }
}

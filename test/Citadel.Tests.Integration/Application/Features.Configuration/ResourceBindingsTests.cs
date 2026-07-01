using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net.Http.Json;
using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Configuration;

public sealed class ResourceBindingsTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid _stackId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var stack = Stack.Create(
            "stack-config",
            Constants.SystemId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack("services:\n  app:\n    image: nginx", StackUpdateBehavior.Notify));

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _stackId = stack.Id;
    }

    [Fact]
    public async Task ResourceBinding_Should_Save_Overrides_And_Not_Return_Secret_Value()
    {
        var createSecretResponse = await Client.PostAsJsonAsync(
            "/api/v1/bindings/secrets",
            new { name = "API_KEY", value = "super-secret-value" },
            cancellationToken: TestContext.Current.CancellationToken);
        createSecretResponse.EnsureSuccessStatusCode();

        var secretJson = await JsonDocument.ParseAsync(
            await createSecretResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        var secretId = secretJson.RootElement.GetProperty("id").GetGuid();

        var globalJson = """
        {
          "entries": [
            { "name": "SHARED_VALUE", "kind": "Variable", "value": "global", "secretId": null }
          ]
        }
        """;
        var globalResponse = await Client.PutAsync(
            "/api/v1/resourceBindings/global",
            new StringContent(globalJson, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);
        globalResponse.EnsureSuccessStatusCode();

        var resourceJson = $$"""
        {
          "entries": [
            { "name": "SHARED_VALUE", "kind": "Variable", "value": "stack", "secretId": null },
            { "name": "API_KEY", "kind": "Secret", "value": null, "secretId": "{{secretId}}", "secretDeliveryMode": "EnvironmentVariable" }
          ]
        }
        """;
        var replaceResponse = await Client.PutAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            new StringContent(resourceJson, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);
        replaceResponse.EnsureSuccessStatusCode();

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
    public async Task Stack_ResourceBinding_Should_Save_Mounted_File_Secret_Delivery()
    {
        var createSecretResponse = await Client.PostAsJsonAsync(
            "/api/v1/bindings/secrets",
            new { name = "FILE_SECRET", value = "super-secret-value" },
            cancellationToken: TestContext.Current.CancellationToken);
        createSecretResponse.EnsureSuccessStatusCode();

        var secretJson = await JsonDocument.ParseAsync(
            await createSecretResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        var secretId = secretJson.RootElement.GetProperty("id").GetGuid();

        var resourceJson = $$"""
        {
          "entries": [
            { "name": "FILE_SECRET", "kind": "Secret", "value": null, "secretId": "{{secretId}}", "secretDeliveryMode": "MountedFile", "targetPath": "/run/secrets/file_secret" }
          ]
        }
        """;

        var replaceResponse = await Client.PutAsync(
            $"/api/v1/resourceBindings/Stack/{_stackId}",
            new StringContent(resourceJson, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        replaceResponse.EnsureSuccessStatusCode();

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
            "/api/v1/bindings/secrets",
            new { name = "FILE_SECRET", value = "super-secret-value" },
            cancellationToken: TestContext.Current.CancellationToken);
        createSecretResponse.EnsureSuccessStatusCode();

        var secretJson = await JsonDocument.ParseAsync(
            await createSecretResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);
        var secretId = secretJson.RootElement.GetProperty("id").GetGuid();

        var resourceJson = $$"""
        {
          "entries": [
            { "name": "FILE_SECRET", "kind": "Secret", "value": null, "secretId": "{{secretId}}", "secretDeliveryMode": "MountedFile", "targetPath": "/run/secrets/file_secret" }
          ]
        }
        """;

        var replaceResponse = await Client.PutAsync(
            "/api/v1/resourceBindings/global",
            new StringContent(resourceJson, Encoding.UTF8, "application/json"),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, replaceResponse.StatusCode);
        var responseBody = await replaceResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Contains("Only environment variable secret delivery is supported", responseBody);
    }

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
    }

    [Fact]
    public async Task Secret_Definitions_Should_Return_Forbidden_Without_Configuration_Permission()
    {
        var subject = await CreateAuthorizationSubjectAsync();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync(
            "/api/v1/bindings/secrets",
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
            $"/api/v1/bindings/secrets?scope=Stack&resourceId={_stackId}",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.OK, scopedResponse.StatusCode);

        var globalResponse = await Client.GetAsync(
            "/api/v1/bindings/secrets",
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Forbidden, globalResponse.StatusCode);
    }
}

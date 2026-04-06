using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Registries;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using System.Text;

namespace Tests.Integration.Application.Features.Registries;

public class RegistryPatchTests : IntegrationTestBase
{
    private Guid registryId;
    private readonly Mock<IRegistryConnectorStrategy> registryConnectorMock = new();
    private readonly Mock<IRegistryConnectorResolver> registryConnectorResolverMock = new();
    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var registry = new Registry(
            name: "OriginalName",
            registryHost: "original.url",
            status: RegistryStatus.Active,
            createdByActorId: Constants.SystemId,
            configuration: new DockerHubRegistry("original-user", "pat123")
        );

        await uow.Registries.AddAsync(registry, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        registryId = registry.Id;
    }

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services
            .AddScoped(_ => registryConnectorMock.Object)
            .AddScoped(_ => registryConnectorResolverMock.Object);
    }

    [Fact]
    public async Task Patch_Registry_Should_Apply_MergePatch()
    {
        // Arrange
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
            .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfiguration>(), It.IsAny<CancellationToken>()))
            .Returns(Task.FromResult<(bool, string?)>((true, null)));

        var patchJson = """
        {
          "registryHost": "updated.url",
          "status": "Disabled",
          "configuration": {
            "$type": "DockerHub",
            "userName": "patched-user"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/registries/{registryId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_RegistryStatus_Should_Succeed()
    {
        // Arrange
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
            .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfiguration>(), It.IsAny<CancellationToken>()))
            .Returns(Task.FromResult<(bool, string?)>((true, null)));

        var patchJson = """
        {
          "status": "Disabled",
          "registryHost": "disabled.url",
          "configuration": {
            "$type": "DockerHub",
            "userName": "patched-user"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/registries/{registryId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_Registry_With_Invalid_Data_Should_Return_BadRequest()
    {
        // Arrange
        var patchJson = """
        {
          "registryHost": ""
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/registries/{registryId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_NonExistent_Registry_Should_Return_NotFound()
    {
        // Arrange
        var nonExistentId = Guid.NewGuid();
        var patchJson = """
        {
          "registryHost": "does-not-exist.url",
          "status": "Active",
          "configuration": {
            "$type": "DockerHub",
            "userName": "patched-user"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        // Act
        var response = await Client.PatchAsync($"/api/v1/registries/{nonExistentId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_Should_Return_Forbidden_If_User_Lacks_Permission()
    {
        // Arrange
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
            .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfiguration>(), It.IsAny<CancellationToken>()))
            .Returns(Task.FromResult<(bool, string?)>((true, null)));

        var patchJson = """
        {
          "registryHost": "patched.url",
          "status": "Active",
          "configuration": {
            "$type": "DockerHub",
            "userName": "patched-user"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");
        var subject = await CreateAuthorizationSubjectAsync();
        var token = CreateJwtToken(subject.UserId, subject.ActorId);
        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", token);

        // Act
        var response = await Client.PatchAsync($"/api/v1/registries/{registryId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_Registry_Should_Succeed_When_User_Has_Registry_Resource_Access()
    {
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
            .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfiguration>(), It.IsAny<CancellationToken>()))
            .Returns(Task.FromResult<(bool, string?)>((true, null)));

        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Registry, registryId, ResourceAction.Update)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var patchJson = """
        {
          "registryHost": "acl.updated.url",
          "status": "Disabled",
          "configuration": {
            "$type": "DockerHub",
            "userName": "patched-user"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/registries/{registryId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registry = await uow.Registries.GetAsync(registryId, TestContext.Current.CancellationToken);

        Assert.NotNull(registry);
        Assert.Equal("acl.updated.url", registry.RegistryHost);
        Assert.Equal(RegistryStatus.Disabled, registry.Status);
    }

    [Fact]
    public async Task Patch_Registry_Metadata_Should_Succeed_When_User_Has_Registry_Resource_Access()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Registry, registryId, ResourceAction.Update)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var patchJson = """
        {
          "description": "ACL metadata update"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/registries/{registryId}/_metadata", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registry = await uow.Registries.GetAsync(registryId, TestContext.Current.CancellationToken);

        Assert.Equal("ACL metadata update", registry?.Description);
    }

    [Fact]
    public async Task Rename_Registry_Should_Succeed_When_User_Inherits_Update_Permission_From_Team_Role()
    {
        var subject = await CreateAuthorizationSubjectAsync(teamRoleId: OperatorRoleId);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var renameJson = $$"""
        {
          "id": "{{registryId}}",
          "name": "TeamGrantedRegistry"
        }
        """;
        var content = new StringContent(renameJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/registries/rename", content, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registry = await uow.Registries.GetAsync(registryId, TestContext.Current.CancellationToken);

        Assert.Equal("TeamGrantedRegistry", registry?.Name);
    }

    [Fact]
    public async Task Patch_Registry_Metadata_Should_Update_Description()
    {
        var patchJson = """
        {
          "description": "Updated registry metadata"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/registries/{registryId}/_metadata", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registry = await uow.Registries.GetAsync(registryId, TestContext.Current.CancellationToken);

        Assert.Equal("Updated registry metadata", registry?.Description);
    }

    [Fact]
    public async Task Rename_Registry_Should_Update_Name()
    {
        var renameJson = $$"""
        {
          "id": "{{registryId}}",
          "name": "RenamedRegistry"
        }
        """;
        var content = new StringContent(renameJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/registries/rename", content, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registry = await uow.Registries.GetAsync(registryId, TestContext.Current.CancellationToken);

        Assert.Equal("RenamedRegistry", registry?.Name);
    }
}
using System.Text;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Registries;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Integration.Application.Features.Registries;

public class RegistryPatchTests : IntegrationTestBase<WebApi.Program>
{
    private Guid registryId;
    private readonly Mock<IRegistryConnectorStrategy> registryConnectorMock = new();
    private readonly Mock<IRegistryConnectorResolver> registryConnectorResolverMock = new();
    protected override async ValueTask SeedDbAsync()
    {
        using var scope = Services.CreateScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var registry = new Registry(
            name: "OriginalName",
            url: "https://original.url",
            type: RegistryType.DockerHub,
            configuration: new DockerHubRegistry("original-user", "pat123")
        );
        uow.Registries.Add(registry);

        await uow.SaveChangesAsync();

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
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryType>()))
            .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfigurationBase>(), It.IsAny<CancellationToken>()))
            .Returns(Task.FromResult<(bool, string?)>((true, null)));

        var patchJson = """
        {
          "name": "UpdatedName",
          "type": "DockerHub",
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
          "name": ""
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
          "name": "DoesNotExist"
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
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryType>()))
            .Returns(registryConnectorMock.Object);

        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfigurationBase>(), It.IsAny<CancellationToken>()))
            .Returns(Task.FromResult<(bool, string?)>((true, null)));

        var patchJson = """
        {
          "name": "UpdatedName",
          "type": "DockerHub",
          "configuration": {
            "$type": "DockerHub",
            "userName": "patched-user"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");
        var token = CreateJwtTokenAsync([]);
        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", token);

        // Act
        var response = await Client.PatchAsync($"/api/v1/registries/{registryId}", content, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
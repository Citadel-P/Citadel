using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using System.Text;
using System.Text.Json;

namespace Tests.Integration.Application.Features.Registries;

public class RegistryViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IRegistryConnectorStrategy> registryConnectorMock = new();
    private readonly Mock<IRegistryConnectorResolver> registryConnectorResolverMock = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services
            .AddScoped(_ => registryConnectorMock.Object)
            .AddScoped(_ => registryConnectorResolverMock.Object);
    }

    [Fact]
    public async Task List_Registries_Should_Return_Only_Registries_User_Is_Permitted_To_View()
    {
        registryConnectorResolverMock.Setup(x => x.Resolve(It.IsAny<RegistryConfiguration>()))
            .Returns(registryConnectorMock.Object);
        registryConnectorMock.Setup(x => x.CanConnectAsync(It.IsAny<RegistryConfiguration>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((true, null));

        var visibleRegistryId = await CreateRegistryAsync("r-visible");
        await CreateRegistryAsync("r-hidden");

        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Registry, visibleRegistryId, ResourceAction.View)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/registries", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.GetProperty("registries");
        Assert.Equal(1, items.GetArrayLength());
        Assert.Equal(visibleRegistryId, items[0].GetProperty("id").GetGuid());
        Assert.Equal("r-visible", items[0].GetProperty("name").GetString());
    }

    private async Task<Guid> CreateRegistryAsync(string name)
    {
        var createJson = $$"""
        {
          "name": "{{name}}",
          "registryHost": "ghcr.io",
          "status": "Active",
          "configuration": {
            "$type": "DockerHub",
            "userName": "dummy-user",
            "PAT": "dummy-pat123"
          }
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/registries",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await uow.Registries.GetAllAsync(TestContext.Current.CancellationToken))
            .Single(x => x.Name == name)
            .Id;
    }
}

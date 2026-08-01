using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text;

namespace Tests.Integration.Application.Features.Registries;

public class RegistryDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid registryId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var registry = new Registry("fake", "ghcr.io", RegistryStatus.Active, Constants.SystemId,
                new GitHubRegistry("citadel-p", true, "pat1"));
        await uow.Registries.AddAsync(registry, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        registryId = registry.Id;
    }

    [Fact]
    public async Task Delete_Registry_ReturnsSuccess()
    {
        // Arrange
        var content = $$"""
        {
            "ids": ["{{registryId}}"]
        }
        """;

        // Act
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/registries")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registries = await uow.Registries.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.NotEmpty(registries);
        Assert.Equal(1, registries.Count(r => r.CreatedByActorId == Constants.SystemId));
        // Response has no content
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Delete_Registries_ShouldNotPartiallyDelete_WhenBatchContainsMissingId()
    {
        var content = $$"""
        {
            "ids": ["{{registryId}}", "{{Guid.NewGuid()}}"]
        }
        """;
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/registries")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registries = await uow.Registries.GetAllAsync(TestContext.Current.CancellationToken);
        Assert.Contains(registries, registry => registry.Id == registryId);
    }

}

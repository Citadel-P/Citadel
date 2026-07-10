using System.Net.Http.Json;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Integration.Application.Features.Platforms;

public class PlatformDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IPlatformHealthMonitorJob> healthMonitorMock = new();
    private Guid platformId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.AddSingleton(_ => healthMonitorMock.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = new Platform(
            name: "P-DELETE",
            address: "https://delete.address",
            networkCount: 1,
            volumeCount: 2,
            imageCount: 3,
            cpuCount: 4,
            memTotal: 500,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: "123456",
                ContainerCount: 5,
                ContainersRunning: 2,
                ContainersPaused: 2,
                ContainersStopped: 1));

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        platformId = platform.Id;
    }

    [Fact]
    public async Task Delete_Platform_Should_Delete_Entity_And_Add_Activity()
    {
        // Arrange
        healthMonitorMock
            .Setup(x => x.UntrackPlatform("https://delete.address", It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/platforms")
        {
            Content = JsonContent.Create(new { ids = new[] { platformId } })
        };

        // Act
        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var exists = await uow.Platforms.ExistsAsync(platformId, TestContext.Current.CancellationToken);
        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            platformId,
            ActivityResourceType.Platform,
            ActivityEventType.PlatformDeleted,
            1,
            10,
            TestContext.Current.CancellationToken);

        var activitySummary = Assert.Single(activities.Items);
        var activity = await uow.ActivityEventRepository.GetByIdAsync(activitySummary.Id, TestContext.Current.CancellationToken);
        var deleted = Assert.IsType<PlatformDeleted>(activity?.Info);

        Assert.False(exists);
        Assert.Equal(platformId, deleted.Platform.Id);
        Assert.Equal("P-DELETE", deleted.Platform.Name);
        Assert.Equal("https://delete.address", deleted.Platform.Address);
        healthMonitorMock.Verify(x => x.UntrackPlatform("https://delete.address", It.IsAny<CancellationToken>()), Times.Once);
    }
}

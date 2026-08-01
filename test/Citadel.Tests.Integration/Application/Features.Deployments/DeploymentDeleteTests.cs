using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Deployments;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Moq;
using System.Collections.Immutable;
using Application.Services;
using System.Text;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Deployments;

public class DeploymentDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid deploymentId;
    private Guid platformId;
    private readonly Mock<IContainerConnector> connector = new();
    private const string DockerContainerId = "0123456789abcdef";

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IConnectorFactory<IContainerConnector>>();
        services.AddSingleton<IConnectorFactory<IContainerConnector>>(
            new FakeConnectorFactory<IContainerConnector>(connector.Object));
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        var deployment = new Deployment
        (
            name: "Test Deployment",
            description: "A deployment for testing",
            platformId: platform.Id,
            createdByActorId: Constants.SystemId,
            spec: new DeploymentSpec
            (
                UpdateBehavior: UpdateBehavior.AutoDeploy,
                Image: new ExternalImage
                (
                    RegistryId: Constants.DefaultRegistryId,
                    ImageTag: "nginx:latest"
                ),
                Ports: new List<string> { "80:80" }
            )

        );

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        deploymentId = deployment.Id;
        platformId = platform.Id;
    }

    [Fact]
    public async Task Delete_Deployment_ReturnsSuccess()
    {
        // Arrange
        var content = $"""
            ["{deploymentId}"]
            """;
            
        // Act
        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/deployments")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();

        // Check DB
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployments = await uow.Deployments.GetInfoAsync(TestContext.Current.CancellationToken);

        Assert.Empty(deployments);
        // Response has no content
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Delete_Deployment_WithContainer_DoesNotConflictWithOwnedDeploymentClaim()
    {
        Guid containerId;
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var container = new Container(
                "deployment-container",
                "sha256:image",
                platformId,
                DockerContainerId,
                ContainerStateStatus.Running,
                deploymentId: deploymentId);
            containerId = container.Id;
            await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        Services.GetRequiredService<IPlatformContainerCache>().ReplacePlatformContainers(
            platformId,
            new PlatformCacheEntry(
                platformId,
                "local://deployment-delete",
                PlatformConnectorType.Local,
                ImmutableDictionary<string, Guid>.Empty.Add(DockerContainerId, containerId)));
        connector
            .Setup(service => service.DeleteAsync(
                It.IsAny<DeleteContainerCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        var response = await Client.SendAsync(
            new HttpRequestMessage(HttpMethod.Delete, "/api/v1/deployments")
            {
                Content = new StringContent($"[\"{deploymentId}\"]", Encoding.UTF8, "application/json")
            },
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        connector.Verify(service => service.DeleteAsync(
            It.IsAny<DeleteContainerCommand>(),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    private sealed class FakeConnectorFactory<T>(T connector) : IConnectorFactory<T>
    {
        public T GetConnector(PlatformConnectorType platformConnectorType) => connector;
    }
}

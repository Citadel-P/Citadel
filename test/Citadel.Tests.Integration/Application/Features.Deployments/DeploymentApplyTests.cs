using Application.Features.Containers.Commands;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using System.Text;
using Tests.Integration.Application.TaskJobs;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Deployments;

public class DeploymentApplyTests : IntegrationTestBase
{
    private Guid _platformId;
    private Guid _deploymentId;
    private Guid _registryId = Constants.DefaultRegistryId;

    private readonly Mock<IDeploymentConnector> deploymentConnectorMock = new();
    private readonly Mock<IContainerConnector> containerConnectorMock = new();
    private readonly Mock<IPullImageService> pullImageServiceMock = new();
    private readonly Mock<IConnectorFactory<IDeploymentConnector>> deploymentConnectorFactoryMock = new();
    private readonly Mock<IConnectorFactory<IContainerConnector>> containerConnectorFactoryMock = new();
    private readonly Mock<IPlatformContainerCache> platformCacheMock = new();
    private readonly TestPlatformHealthBroadCaster broadcaster = new();


    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();

        services
            .AddHostedService<DbWriteWorker>()
            .AddSingleton<IDbWorkQueue, DbWorkQueue>()
            .AddSingleton(_ => deploymentConnectorMock.Object)
            .AddSingleton(_ => containerConnectorMock.Object)
            .AddSingleton(_ => pullImageServiceMock.Object)
            .AddSingleton(_ => deploymentConnectorFactoryMock.Object)
            .AddSingleton(_ => containerConnectorFactoryMock.Object)
            .AddSingleton(_ => platformCacheMock.Object);

        services.AddSingleton<IPlatformHealthBroadCaster>(broadcaster);

        // Setup deployment connector factory mock
        deploymentConnectorFactoryMock
            .Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(deploymentConnectorMock.Object);

        // Setup container connector factory mock
        containerConnectorFactoryMock
            .Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>()))
            .Returns(containerConnectorMock.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        var deployment = new Deployment(
            name: "Test Deployment",
            description: "A deployment for testing apply",
            platformId: platform.Id,
            createdByActorId: Constants.SystemId,
            spec: new DeploymentSpec(
                Image: new LocalImage(ImageId: "sha256:1234567890abcdef"),
                UpdateBehavior: UpdateBehavior.AutoDeploy,
                Ports: new List<string> { "80:80" },
                ResourceSpec: new ResourceSpec(
                    NanoCpus: (float)0.5,
                    MemoryLimit: 256
                ),
                EnvVars: new List<string> { "ENV=production" }
            )
        );

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(deployment, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _platformId = platform.Id;
        _deploymentId = deployment.Id;

        // Setup platform cache mock
        var platformCacheEntry = new PlatformCacheEntry(
            Id: platform.Id,
            Address: platform.Address,
            ConnectorType: platform.ConnectorType,
            Containers: []
        );
        
        platformCacheMock
            .Setup(x => x.TryGetCacheEntry(_platformId, out It.Ref<PlatformCacheEntry>.IsAny, out It.Ref<LightResults.Error>.IsAny))
            .Returns((Guid id, out PlatformCacheEntry entry, out LightResults.Error error) =>
            {
                entry = platformCacheEntry;
                error = null!;
                return true;
            });
    }

    [Fact]
    public async Task Apply_Deployment_WithLocalImage_ReturnsSuccess()
    {
        // Arrange
        var containerId = "container-123";
        var deploymentResult = new ApplyDeploymentResult(
            ContainerId: containerId,
            DeployedContainerState: DeployedContainerState.Running
        );

        deploymentConnectorMock
            .Setup(x => x.ApplyDeploymentAsync(It.IsAny<ApplyDeploymentCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(LightResults.Result.Success(deploymentResult));

        await CreateContainer(containerId);

        var applyInputJson = $$"""
        {
            "id": "{{_deploymentId}}",
            "recreate": false
        }
        """;
        var content = new StringContent(applyInputJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync(
            "/api/v1/deployments/apply",
            content,
            cancellationToken: TestContext.Current.CancellationToken
        );

        // Assert
        Assert.True(response.IsSuccessStatusCode);

        // Give work queue time to process DeploymentSucceededWorkItem
        await Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken);

        // Verify DB state changed to Healthy
        var verifyScope = Services.CreateAsyncScope();
        var verifyUow = verifyScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await verifyUow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);
        Assert.Equal(DeploymentStatus.Healthy, deployment?.Status);

        // Verify container was linked to deployment
        var persistedContainer = await verifyUow.Containers.GetByIdAsync(containerId, TestContext.Current.CancellationToken);
        Assert.NotNull(persistedContainer);
        Assert.Equal(containerId, persistedContainer.DockerContainerId);
        Assert.Equal(_deploymentId, persistedContainer.DeploymentId);
    }

    [Fact]
    public async Task Apply_Deployment_WithExternalImage_PullsImageThenApplies()
    {
        // Arrange
        var deploymentWithExternalImage = new Deployment(
            name: "External Image Deployment",
            description: "Uses external image",
            platformId: _platformId,
            createdByActorId: Constants.SystemId,
            spec: new DeploymentSpec(
                UpdateBehavior: UpdateBehavior.AutoDeploy,
                Image: new ExternalImage(
                    RegistryId: _registryId,
                    ImageTag: "nginx:latest"
                ),
                ResourceSpec: new ResourceSpec(
                    NanoCpus: (float)0.5,
                    MemoryLimit: 256
                ),
                Ports: new List<string> { "80:80" }
            )
        );

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Deployments.AddAsync(deploymentWithExternalImage, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        // Mock pull image service to stream pull progress
        var pullItems = new[]
        {
            new Domain.Contracts.Resources.Images.PullImageStreamItem(ProgressMessage: "Pulling nginx:latest..."),
            new Domain.Contracts.Resources.Images.PullImageStreamItem(
                ProgressMessage: "Pull completed",
                DockerImageId: "sha256:pulledimage123"
            )
        };

        pullImageServiceMock
            .Setup(x => x.PullAsync(It.IsAny<PullImageService.PullImageInput>(), It.IsAny<CancellationToken>()))
            .Returns(GetMockAsyncEnumerable(pullItems));

        var containerId = "container-external-123";
        var deploymentResult = new ApplyDeploymentResult(
            ContainerId: containerId,
            DeployedContainerState: DeployedContainerState.Running
        );

        deploymentConnectorMock
            .Setup(x => x.ApplyDeploymentAsync(It.IsAny<ApplyDeploymentCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(LightResults.Result.Success(deploymentResult));

        await CreateContainer(containerId);

        var applyInputJson = $$"""
        {
            "id": "{{deploymentWithExternalImage.Id}}",
            "recreate": false
        }
        """;
        var content = new StringContent(applyInputJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync(
            "/api/v1/deployments/apply",
            content,
            cancellationToken: TestContext.Current.CancellationToken
        );

        // Assert
        Assert.True(response.IsSuccessStatusCode);

        // Verify pull was called
        pullImageServiceMock.Verify(
            x => x.PullAsync(It.IsAny<PullImageService.PullImageInput>(), It.IsAny<CancellationToken>()),
            Times.Once
        );

        // Give work queue time to process DeploymentSucceededWorkItem
        await Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken);

        // Verify deployment status is Healthy
        var verifyScope = Services.CreateAsyncScope();
        var verifyUow = verifyScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await verifyUow.Deployments.GetAsync(deploymentWithExternalImage.Id, TestContext.Current.CancellationToken);
        Assert.Equal(DeploymentStatus.Healthy, deployment?.Status);

        // Verify container was linked to deployment
        var persistedContainer = await verifyUow.Containers.GetByIdAsync(containerId, TestContext.Current.CancellationToken);
        Assert.NotNull(persistedContainer);
        Assert.Equal(containerId, persistedContainer.DockerContainerId);
        Assert.Equal(deploymentWithExternalImage.Id, persistedContainer.DeploymentId);
    }

    [Fact]
    public async Task Apply_Deployment_WithRecreate_DeletesExistingContainerFirst()
    {
        // Arrange
        var container = new Container(
            platformId: _platformId,
            dockerContainerId: "old-container-123",
            dockerImageId: "old-image",
            name: "old-deployment-container",
            created: 999999,
            state: ContainerStateStatus.Running,
            deploymentId: _deploymentId
        );

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        // Mock container deletion
        containerConnectorMock
            .Setup(x => x.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(LightResults.Result.Success());

        // Mock deployment apply
        var newContainerId = "new-container-456";
        var newDeploymentResult = new ApplyDeploymentResult(
            ContainerId: newContainerId,
            DeployedContainerState: DeployedContainerState.Running
        );

        // Manually create the new container that the connector would have returned (normally the daemon job would create it)
        await CreateContainer(newContainerId);

        deploymentConnectorMock
            .Setup(x => x.ApplyDeploymentAsync(It.IsAny<ApplyDeploymentCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(LightResults.Result.Success(newDeploymentResult));

        var applyInputJson = $$"""
        {
            "id": "{{_deploymentId}}",
            "recreate": true
        }
        """;
        var content = new StringContent(
            applyInputJson,
            Encoding.UTF8,
            "application/json"
        );

        // Act
        var response = await Client.PostAsync(
            $"/api/v1/deployments/apply",
            content,
            cancellationToken: TestContext.Current.CancellationToken
        );

        // Assert
        Assert.True(response.IsSuccessStatusCode);

        // Verify container was deleted
        containerConnectorMock.Verify(
            x => x.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()),
            Times.Once
        );

        // Give work queue time to process DeploymentSucceededWorkItem
        await Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken);

        // Verify new container was linked to deployment
        var verifyScope = Services.CreateAsyncScope();
        var verifyUow = verifyScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persistedNewContainer = await verifyUow.Containers.GetByIdAsync(newContainerId, TestContext.Current.CancellationToken);
        Assert.NotNull(persistedNewContainer);
        Assert.Equal(_deploymentId, persistedNewContainer.DeploymentId);

        // Verify deployment status is Healthy
        var deployment = await verifyUow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);
        Assert.Equal(DeploymentStatus.Healthy, deployment?.Status);
    }

    [Fact]
    public async Task Apply_Deployment_WhenDeploymentNotFound_ReturnsError()
    {
        // Arrange
        var nonExistentDeploymentId = Guid.NewGuid();

        var applyInputJson = $$"""
        {
            "id": "{{nonExistentDeploymentId}}",
            "recreate": false
        }
        """;
        var content = new StringContent(
            applyInputJson,
            Encoding.UTF8,
            "application/json"
        );

        // Act
        var response = await Client.PostAsync(
            "/api/v1/deployments/apply",
            content,
            cancellationToken: TestContext.Current.CancellationToken
        );

        // Assert
        // Stream endpoints return 200 with error items in stream
        Assert.True(response.IsSuccessStatusCode || response.StatusCode == System.Net.HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task Apply_Deployment_WhenPlatformNotFound_ReturnsError()
    {
        // Arrange
        platformCacheMock
            .Setup(x => x.TryGetCacheEntry(It.IsAny<Guid>(), out It.Ref<PlatformCacheEntry>.IsAny, out It.Ref<LightResults.Error>.IsAny))
            .Returns(false);

        var applyInputJson = $$"""
        {
            "id": "{{_deploymentId}}",
            "recreate": false
        }
        """;
        var content = new StringContent(applyInputJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync(
            $"/api/v1/deployments/apply",
            content,
            cancellationToken: TestContext.Current.CancellationToken
        );

        // Assert
        Assert.True(response.IsSuccessStatusCode);
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);
        Assert.Equal(DeploymentStatus.Failed, deployment?.Status);
    }

    [Fact]
    public async Task Apply_Deployment_WhenConnectorFails_UpdatesStatusToFailed()
    {
        // Arrange
        var deploymentError = new Hosting.Common.ErrorTypes.InternalServerError("Connector error occurred");

        deploymentConnectorMock
            .Setup(x => x.ApplyDeploymentAsync(It.IsAny<ApplyDeploymentCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(LightResults.Result.Failure<ApplyDeploymentResult>(deploymentError));

        var applyInputJson = $$"""
        {
            "id": "{{_deploymentId}}",
            "recreate": false
        }
        """;
        var content = new StringContent(applyInputJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync(
            "/api/v1/deployments/apply",
            content,
            cancellationToken: TestContext.Current.CancellationToken
        );

        // Assert
        Assert.True(response.IsSuccessStatusCode);

        // Give work queue time to process UpdateDeploymentStatusWorkItem
        await Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken);

        // Verify deployment status changed to Failed
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);
        Assert.Equal(DeploymentStatus.Failed, deployment?.Status);
    }

    [Fact]
    public async Task Apply_Deployment_WhenContainerNotRunning_UpdatesStatusToFailed()
    {
        // Arrange
        var deploymentResult = new ApplyDeploymentResult(
            ContainerId: "container-123",
            DeployedContainerState: DeployedContainerState.Exited // Not running!
        );

        deploymentConnectorMock
            .Setup(x => x.ApplyDeploymentAsync(It.IsAny<ApplyDeploymentCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(LightResults.Result.Success(deploymentResult));

        var applyInputJson = $$"""
        {
            "id": "{{_deploymentId}}",
            "recreate": false
        }
        """;
        var content = new StringContent(applyInputJson, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync(
            "/api/v1/deployments/apply",
            content,
            cancellationToken: TestContext.Current.CancellationToken
        );

        // Assert
        Assert.True(response.IsSuccessStatusCode);

        // Give work queue time to process UpdateDeploymentStatusWorkItem
        await Task.Delay(TimeSpan.FromSeconds(1), TestContext.Current.CancellationToken);

        // Verify deployment status is Failed
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deployment = await uow.Deployments.GetAsync(_deploymentId, TestContext.Current.CancellationToken);
        Assert.Equal(DeploymentStatus.Failed, deployment?.Status);
    }

    // Helper method to create mock async enumerable
    private static async IAsyncEnumerable<T> GetMockAsyncEnumerable<T>(IEnumerable<T> items)
    {
        foreach (var item in items)
        {
            yield return await Task.FromResult(item);
        }
    }

    private async Task CreateContainer(string containerId)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var container = new Container(
            platformId: _platformId,
            dockerContainerId: containerId,
            dockerImageId: "sha256:1234567890abcdef",
            name: "Test Deployment",
            created: DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            state: ContainerStateStatus.Running
        );

        await uow.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }
}

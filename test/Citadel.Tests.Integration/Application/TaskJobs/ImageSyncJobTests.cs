using System.Drawing;
using Application.Configs;
using Application.Mappers;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Hosting.DockerClient;
using LightResults;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Options;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.TaskJobs;

public class ImageSyncJobTests : IntegrationTestBase
{
    private readonly Mock<IConnectorFactory<IImageConnector>> imageFactoryMock = new();
    private readonly Mock<IImageConnector> imageConnector = new();

    private readonly Mock<IOptions<JobConfiguration>> configMock = new();
    private readonly TestPlatformHealthBroadCaster broadcaster = new();
    private readonly Mock<IImageStreamManager> streamManagerMock = new();

    private Guid platformId;
    private const int batchSize = 2;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        // Remove all existing hosted services
        services.RemoveAll<IHostedService>();

        services.AddHostedService<ImageSyncJob>();

        services.AddSingleton(streamManagerMock.Object);
        services.AddSingleton(configMock.Object);
        services.AddSingleton(imageConnector.Object);
        services.AddSingleton(imageFactoryMock.Object);
        services.AddSingleton<IPlatformHealthBroadCaster>(broadcaster);

        configMock.Setup(x => x.Value).Returns(new JobConfiguration()
        {
            BatchSize = batchSize
        });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();

        await uow.Platforms.AddPlatformAsync(platform, TestContext.Current.CancellationToken);
        foreach (var image in Fakes.GetDummyImages())
        {
            await uow.Images.AddAsync(new Image(
                name: image.GetName() ?? "",
                tag: image.GetTag(),
                imageId: image.Id,
                size: image.Size,
                isInUse : image.Containers % 2 == 0,
                isUpToDate: false,
                platformId: platform.Id,
                createdAt: DateTimeOffset.FromUnixTimeSeconds(image.Created).DateTime
                ), TestContext.Current.CancellationToken);
        }

        await uow.CommitAsync();

        platformId = platform.Id;
    }

    [Fact]
    public async Task SynchronizesImages_WhenPlatformBecomesOnline()
    {
        // Arrange
        var freshImages = Fakes.GetDummyImages();
        imageFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(imageConnector.Object);

        imageConnector.Setup(x => x.ListImagesAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(freshImages));
        
        // Act
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);

        await Task.Delay(500, TestContext.Current.CancellationToken); // wait for jobs to process

        // Assert
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var dbImages = await uow.Images.GetByPlatformIdAsync(platformId, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(3, dbImages.Count());
        Assert.Equal(dbImages.Select(s => s.Name), freshImages.Select(s => s.GetName()));
        streamManagerMock.Verify(x => x.SendImagesInfo(It.IsAny<Guid>(), It.IsAny<IEnumerable<Image>>()), Times.Once);
    }

    [Fact]
    public async Task RemovesStaleImages_WhenNotInFreshList()
    {
        // Arrange: Seed DB with a stale image that will be missing from the fresh list
        string staleImageId = "012sfv545s";
        var staleImage = new Image(
            name: "stale",
            tag: "stale:01",
            imageId: staleImageId,
            size: 900000,
            isInUse: false,
            isUpToDate: false,
            platformId: platformId,
            createdAt: DateTime.UtcNow);
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Images.AddAsync(staleImage, TestContext.Current.CancellationToken);
        await uow.CommitAsync();

        // Return fresh images
        imageFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(imageConnector.Object);
        imageConnector.Setup(x => x.ListImagesAsync(It.IsAny<string>(), It.IsAny<CancellationToken>())).ReturnsAsync(Result.Success(Fakes.GetDummyImages()));

        // Act
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert: Stale image should be removed
        await using var assertScope = Services.CreateAsyncScope();
        var freshUow = assertScope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var dbImages = await freshUow.Images.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        Assert.DoesNotContain(dbImages, c => c.ImageId == staleImageId);
        Assert.Equal(3, dbImages.Count());
    }

    [Fact]
    public async Task AddsNewImages_WhenNotInDatabase()
    {
        // Arrange: DB has no images, but fresh list has one
        var imageId = "new-id";
        var newImage = new ImageResult(
            Id: imageId,
            RepoTags: [$"new-image:latest"],
            Size: 123456,
            Containers: 3,
            Created: 9999999,
            SharedSize: 999,
            VirtualSize: 123456,
            ParentId: "p-012",
            RepoDigests: [],
            Labels: new Dictionary<string, string>());
        imageFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(imageConnector.Object);
        imageConnector.Setup(x => x.ListImagesAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new List<ImageResult>() { newImage } as IReadOnlyList<ImageResult>));

        // Act
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert: New image should be added
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var dbImages = await uow.Images.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        Assert.Contains(dbImages, c => c.ImageId == imageId);
    }

    [Fact]
    public async Task UpdatesExistingImages_WhenPropertiesChange()
    {
        // Arrange: Seed DB with an image, then fresh list has same image with different properties
        var imageId = "update-id";
        var oldImage = new Image(
            name: "old",
            tag: "old:latest",
            imageId: "fake-id",
            size: 100000,
            isInUse: true,
            platformId: platformId,
            createdAt: DateTime.UtcNow,
            isUpToDate: true,
            null, null);
        await using (var uow = Services.GetRequiredService<IUnitOfWork>())
        {
            await uow.Images.AddAsync(oldImage, TestContext.Current.CancellationToken);
            await uow.CommitAsync();
        }

        var updatedDockerContainer = new ImageResult(
            Id: imageId,
            RepoTags: ["updated-image:latest"],
            Size: 200000,
            Containers: 3,
            Created: 9999999,
            SharedSize: 999,
            VirtualSize: 123456,
            ParentId: "p-012",
            RepoDigests: [],
            Labels: new Dictionary<string, string>());
        imageFactoryMock.Setup(x => x.GetConnector(It.IsAny<PlatformConnectorType>())).Returns(imageConnector.Object);
        imageConnector.Setup(x => x.ListImagesAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
             .ReturnsAsync(Result.Success(new List<ImageResult>() { updatedDockerContainer } as IReadOnlyList<ImageResult>));

        // Act
        await broadcaster.PublishAsync(new PlatformHealth(platformId, "https://original.address", PlatformConnectorType.Agent, IsOnLine: true),
            cancellationToken: TestContext.Current.CancellationToken);
        await Task.Delay(500, TestContext.Current.CancellationToken);

        // Assert: Image should be updated
        await using var scope = Services.CreateAsyncScope();
        var uow2 = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var dbImages = await uow2.Images.GetByPlatformIdAsync(platformId, TestContext.Current.CancellationToken);
        var updated = dbImages.First(c => c.ImageId == imageId);
        Assert.Equal(1, dbImages?.Count());
        Assert.Equal(imageId, updated.ImageId);
        Assert.Equal(200000, updated.Size);
    }
}


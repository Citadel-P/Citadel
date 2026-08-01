using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Registries;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class ImageRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task GetByIdAsync_ShouldReturnImageForPlatform()
    {
        var platform = Fakes.GetDummyPlatform();
        var image = new Image(
            name: "adoption-image",
            tags: ["nginx:latest"],
            dockerImageId: $"sha256:{Guid.CreateVersion7():N}",
            size: 1024,
            containers: 1,
            platformId: platform.Id,
            createdAt: DateTime.UtcNow);

        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await unitOfWork.Platforms.AddAsync(
            platform,
            TestContext.Current.CancellationToken);
        await unitOfWork.Images.AddOrUpdateAsync(
            image,
            TestContext.Current.CancellationToken);
        await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);

        var persisted = await unitOfWork.Images.GetByIdAsync(
            image.Id,
            platform.Id,
            TestContext.Current.CancellationToken);

        Assert.NotNull(persisted);
        Assert.Equal(image.Id, persisted.Id);
        Assert.Equal(image.DockerImageId, persisted.DockerImageId);
        Assert.Equal(platform.Id, persisted.PlatformId);
        Assert.Null(persisted.RegistryId);
        Assert.Null(persisted.Registry);
    }

    [Fact]
    public async Task GetByIdAsync_ShouldHydrateRegistryBackedImage()
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var image = await AddRegistryBackedImageAsync(unitOfWork);

        var persisted = await unitOfWork.Images.GetByIdAsync(
            image.Id,
            image.PlatformId,
            TestContext.Current.CancellationToken);

        AssertRegistryBackedImage(persisted, image);
    }

    [Fact]
    public async Task GetByDockerImageIdsAsync_ShouldHydrateRegistryBackedImage()
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var image = await AddRegistryBackedImageAsync(unitOfWork);

        var persisted = Assert.Single(await unitOfWork.Images.GetByIdAsync(
            [image.DockerImageId],
            image.PlatformId,
            TestContext.Current.CancellationToken));

        AssertRegistryBackedImage(persisted, image);
    }

    [Fact]
    public async Task GetStuckImagesAsync_ShouldHydrateRegistryBackedImage()
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var image = await AddRegistryBackedImageAsync(unitOfWork);
        var startedAt = DateTimeOffset.UtcNow.AddMinutes(-2).ToUnixTimeSeconds();

        await unitOfWork.Images.UpdateProcessingAsync(
            image.Id,
            ResourceControlState.Processing,
            startedAt,
            image.RowVersion,
            checkRowVersion: false,
            TestContext.Current.CancellationToken);
        await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);

        var persisted = Assert.Single(
            await unitOfWork.Images.GetStuckImagesAsync(
                timeout_s: 60,
                TestContext.Current.CancellationToken),
            candidate => candidate.Id == image.Id);

        AssertRegistryBackedImage(persisted, image);
    }

    private static async Task<Image> AddRegistryBackedImageAsync(IUnitOfWork unitOfWork)
    {
        var platform = Fakes.GetDummyPlatform();
        var image = new Image(
            name: "registry-image",
            tags: ["nginx:latest"],
            dockerImageId: $"sha256:{Guid.CreateVersion7():N}",
            size: 1024,
            containers: 1,
            platformId: platform.Id,
            createdAt: DateTime.UtcNow,
            registryId: Constants.DefaultRegistryId);

        await unitOfWork.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await unitOfWork.Images.AddOrUpdateAsync(image, TestContext.Current.CancellationToken);
        await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
        return image;
    }

    private static void AssertRegistryBackedImage(Image? persisted, Image expected)
    {
        Assert.NotNull(persisted);
        Assert.Equal(expected.Id, persisted.Id);
        Assert.Equal(Constants.DefaultRegistryId, persisted.RegistryId);
        Assert.Equal(Constants.DefaultRegistryId, persisted.Registry?.Id);
        Assert.IsType<DockerHubRegistry>(persisted.Registry?.Configuration);
    }
}

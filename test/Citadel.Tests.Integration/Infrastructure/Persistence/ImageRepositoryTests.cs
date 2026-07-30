using Domain.Contracts.Interfaces;
using Domain.Entities;
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
    }
}

using System.Collections.Immutable;
using System.Net;
using System.Net.Http.Json;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Domain.Entities.Platforms;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Moq;

namespace Tests.Integration.Application.Features.Images;

public sealed class ImageDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IImageConnector> imageConnector = new(MockBehavior.Strict);
    private Guid platformId;
    private const string DeletedImageId = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    private const string RemainingImageId = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    private const string MissingImageId = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IConnectorFactory<IImageConnector>>();
        services.AddSingleton<IConnectorFactory<IImageConnector>>(
            new FakeConnectorFactory<IImageConnector>(imageConnector.Object));
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = new Platform(
            "image-delete-platform",
            "local://image-delete",
            0,
            0,
            2,
            1,
            1024,
            null,
            null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerPlatformDescriptor("image-delete-daemon", 0, 0, 0, 0));
        platformId = platform.Id;

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Images.AddOrUpdateAsync(CreateImage(DeletedImageId, platform.Id), TestContext.Current.CancellationToken);
        await uow.Images.AddOrUpdateAsync(CreateImage(RemainingImageId, platform.Id), TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task DeleteImages_ShouldReconcilePartialDaemonFailure()
    {
        ConnectPlatformCache();

        imageConnector
            .Setup(connector => connector.DeleteImageAsync(
                It.Is<DeleteImageCommand>(command => command.Ids.Count == 2),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<DeleteImageResult>(new BadRequestError("one image could not be deleted")));
        imageConnector
            .Setup(connector => connector.ListImagesAsync(
                "local://image-delete",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<ImageResult>>(
            [
                new ImageResult(
                    RemainingImageId,
                    1024,
                    0,
                    string.Empty,
                    0,
                    0,
                    1024,
                    ["remaining:latest"],
                    [],
                    new Dictionary<string, string>())
            ]));

        var response = await Client.SendAsync(
            new HttpRequestMessage(HttpMethod.Delete, "/api/v1/images")
            {
                Content = JsonContent.Create(new
                {
                    platformId,
                    ids = new[] { DeletedImageId, RemainingImageId }
                })
            },
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var deleted = await uow.Images.GetByDockerImageIdAsync(
            DeletedImageId,
            platformId,
            TestContext.Current.CancellationToken);
        var remaining = await uow.Images.GetByDockerImageIdAsync(
            RemainingImageId,
            platformId,
            TestContext.Current.CancellationToken);

        Assert.Null(deleted);
        Assert.NotNull(remaining);
        Assert.Equal(ResourceControlState.Idle, remaining.ControlState);
    }

    [Fact]
    public async Task DeleteImages_ShouldRollbackClaims_WhenConnectorThrowsBeforeDeleting()
    {
        ConnectPlatformCache();
        imageConnector
            .Setup(connector => connector.DeleteImageAsync(
                It.IsAny<DeleteImageCommand>(),
                It.IsAny<CancellationToken>()))
            .ThrowsAsync(new HttpRequestException("daemon unavailable"));
        imageConnector
            .Setup(connector => connector.ListImagesAsync(
                "local://image-delete",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<ImageResult>>(
            [
                CreateImageResult(DeletedImageId),
                CreateImageResult(RemainingImageId)
            ]));

        var response = await DeleteAsync([DeletedImageId, RemainingImageId]);

        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
        await AssertImageControlStateAsync(DeletedImageId, ResourceControlState.Idle);
        await AssertImageControlStateAsync(RemainingImageId, ResourceControlState.Idle);
    }

    [Fact]
    public async Task DeleteImages_ShouldNotPartiallyDelete_WhenBatchContainsMissingId()
    {
        ConnectPlatformCache();

        var response = await DeleteAsync([DeletedImageId, MissingImageId]);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
        await AssertImageControlStateAsync(DeletedImageId, ResourceControlState.Idle);
        imageConnector.Verify(
            connector => connector.DeleteImageAsync(
                It.IsAny<DeleteImageCommand>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    private void ConnectPlatformCache()
    {
        Services.GetRequiredService<IPlatformContainerCache>().ReplacePlatformContainers(
            platformId,
            new PlatformCacheEntry(
                platformId,
                "local://image-delete",
                PlatformConnectorType.Local,
                ImmutableDictionary<string, Guid>.Empty));
    }

    private Task<HttpResponseMessage> DeleteAsync(string[] ids)
        => Client.SendAsync(
            new HttpRequestMessage(HttpMethod.Delete, "/api/v1/images")
            {
                Content = JsonContent.Create(new
                {
                    platformId,
                    ids
                })
            },
            TestContext.Current.CancellationToken);

    private async Task AssertImageControlStateAsync(string dockerImageId, ResourceControlState expected)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var image = await uow.Images.GetByDockerImageIdAsync(
            dockerImageId,
            platformId,
            TestContext.Current.CancellationToken);

        Assert.NotNull(image);
        Assert.Equal(expected, image.ControlState);
    }

    private static ImageResult CreateImageResult(string dockerImageId)
        => new(
            dockerImageId,
            1024,
            0,
            string.Empty,
            0,
            0,
            1024,
            ["image:latest"],
            [],
            new Dictionary<string, string>());

    private static Image CreateImage(string dockerImageId, Guid ownerPlatformId)
        => new(
            dockerImageId == DeletedImageId ? "deleted" : "remaining",
            [],
            dockerImageId,
            1024,
            0,
            ownerPlatformId,
            DateTime.UtcNow);

    private sealed class FakeConnectorFactory<T>(T connector) : IConnectorFactory<T>
    {
        public T GetConnector(PlatformConnectorType platformConnectorType) => connector;
    }
}

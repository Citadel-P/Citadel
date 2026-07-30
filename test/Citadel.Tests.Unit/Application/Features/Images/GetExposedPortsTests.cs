using System.Collections.Immutable;
using Application.Features.Images.Queries;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Features.Images;

public sealed class GetExposedPortsTests
{
    [Fact]
    public async Task Handle_ShouldResolveCitadelImageIdBeforeInspectingDockerImage()
    {
        var platform = new PlatformCacheEntry(
            Guid.CreateVersion7(),
            "local",
            PlatformConnectorType.Local,
            ImmutableDictionary<string, Guid>.Empty);
        var image = new Image(
            name: "nginx",
            tags: ["nginx:latest"],
            dockerImageId: "sha256:5a88c9c45479443d7be2eadc894b4ed0a9801bae03d97a5760ae13b5c2005942",
            size: 1024,
            containers: 1,
            platformId: platform.Id,
            createdAt: DateTime.UtcNow);

        var platformCache = new Mock<IPlatformContainerCache>();
        Error? cacheError = null;
        platformCache
            .Setup(cache => cache.TryGetCacheEntry(platform.Id, out platform, out cacheError))
            .Returns(true);

        var images = new Mock<global::Domain.Contracts.Interfaces.IImageRepository>();
        images
            .Setup(repository => repository.GetByIdAsync(
                image.Id,
                platform.Id,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(image);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Images).Returns(images.Object);

        var imageConnector = new Mock<IImageConnector>();
        imageConnector
            .Setup(connector => connector.GetExposedPortsAsync(
                It.Is<RunImageInfoCommand>(command =>
                    command.PlatformAddress == platform.Address
                    && command.ImageId == image.DockerImageId),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new ExposedPortsResult(["80/tcp"])));
        var connectorFactory = Mock.Of<IConnectorFactory<IImageConnector>>(
            factory => factory.GetConnector(platform.ConnectorType) == imageConnector.Object);

        var handler = new GetRunImageInfoHandler(
            platformCache.Object,
            unitOfWork.Object,
            connectorFactory);

        var result = await handler.Handle(
            new GetExposedPorts(platform.Id, image.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var ports, out var error), error?.Message);
        Assert.Equal(["80/tcp"], ports.Ports);
        images.Verify(
            repository => repository.GetByIdAsync(
                image.Id,
                platform.Id,
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public void Validator_ShouldValidateTheExposedPortsQuery()
    {
        var validator = new GetExposedPorts.Validator();

        var valid = validator.Validate(new GetExposedPorts(Guid.CreateVersion7(), Guid.CreateVersion7()));
        var invalid = validator.Validate(new GetExposedPorts(Guid.Empty, Guid.Empty));

        Assert.True(valid.IsValid);
        Assert.False(invalid.IsValid);
        Assert.Equal(2, invalid.Errors.Count);
    }
}

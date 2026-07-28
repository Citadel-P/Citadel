using System.Collections.Immutable;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Entities.Stacks;
using Hosting.Common;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class ManualStackDeployedImageResolverTests
{
    [Fact]
    public async Task ResolveAsync_MatchesDockerHubShorthandAndSharedServiceImage()
    {
        var stack = CreateStack();
        var checks = new[]
        {
            CreateCheck(stack, "api", "nginx:latest", "nginx"),
            CreateCheck(stack, "worker", "nginx:latest", "nginx")
        };
        var dependencies = CreateDependencies(
            stack,
            new DockerContainer(
                Name: "/manual-stack-api-1",
                Image: "nginx:latest",
                Id: "container-id",
                ImageId: "sha256:image-id",
                State: ContainerStateStatus.Running),
            new ImageResult(
                Id: "sha256:image-id",
                Size: 1,
                Containers: 1,
                ParentId: string.Empty,
                SharedSize: 0,
                Created: 1,
                VirtualSize: 1,
                RepoTags: ["docker.io/library/nginx:latest"],
                RepoDigests: ["docker.io/library/nginx@sha256:deployed"],
                Labels: null));

        var result = await dependencies.Resolver.ResolveAsync(
            stack,
            checks,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var digests, out var error), error?.Message);
        Assert.Equal("sha256:deployed", digests[ManualStackUpdateEvaluator.StateKey("api", "nginx:latest")]);
        Assert.Equal("sha256:deployed", digests[ManualStackUpdateEvaluator.StateKey("worker", "nginx:latest")]);
        dependencies.ImageConnector.Verify(
            connector => connector.ListImagesAsync(
                dependencies.Platform.Address,
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task ResolveAsync_PreservesRegistryHostPorts()
    {
        var stack = CreateStack();
        const string imageName = "registry.example.test:5000/acme/api:1.0";
        var dependencies = CreateDependencies(
            stack,
            new DockerContainer(
                Name: "/manual-stack-api-1",
                Image: imageName,
                Id: "container-id",
                ImageId: "sha256:image-id",
                State: ContainerStateStatus.Running),
            new ImageResult(
                Id: "sha256:image-id",
                Size: 1,
                Containers: 1,
                ParentId: string.Empty,
                SharedSize: 0,
                Created: 1,
                VirtualSize: 1,
                RepoTags: [imageName],
                RepoDigests: ["registry.example.test:5000/acme/api@sha256:deployed"],
                Labels: null));

        var result = await dependencies.Resolver.ResolveAsync(
            stack,
            [CreateCheck(stack, "api", imageName, "acme/api")],
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var digests, out var error), error?.Message);
        Assert.Equal("sha256:deployed", Assert.Single(digests).Value);
    }

    [Fact]
    public async Task ResolveAsync_FailsWhenTheDeployedImageHasNoRegistryDigest()
    {
        var stack = CreateStack();
        var dependencies = CreateDependencies(
            stack,
            new DockerContainer(
                Name: "/manual-stack-api-1",
                Image: "example/api:latest",
                Id: "container-id",
                ImageId: "sha256:image-id",
                State: ContainerStateStatus.Running),
            new ImageResult(
                Id: "sha256:image-id",
                Size: 1,
                Containers: 1,
                ParentId: string.Empty,
                SharedSize: 0,
                Created: 1,
                VirtualSize: 1,
                RepoTags: ["example/api:latest"],
                RepoDigests: [],
                Labels: null));

        var result = await dependencies.Resolver.ResolveAsync(
            stack,
            [CreateCheck(stack, "api", "example/api:latest", "example/api")],
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error, out _));
        Assert.Contains("Redeploy the stack with image pulling enabled", error.Message, StringComparison.Ordinal);
    }

    private static ResolverDependencies CreateDependencies(
        Stack stack,
        DockerContainer container,
        ImageResult image)
    {
        var platform = new PlatformCacheEntry(
            stack.CurrentStackRelease!.PlatformId,
            "local",
            PlatformConnectorType.Local,
            ImmutableDictionary<string, Guid>.Empty);
        var platformCache = new Mock<IPlatformContainerCache>();
        Error? cacheError = null;
        platformCache
            .Setup(cache => cache.TryGetCacheEntry(platform.Id, out platform, out cacheError))
            .Returns(true);

        var containerConnector = new Mock<IContainerConnector>();
        IReadOnlyDictionary<string, DockerContainer> containers =
            new Dictionary<string, DockerContainer> { [container.Id] = container };
        containerConnector
            .Setup(connector => connector.ListContainersAsync(
                It.IsAny<ContainerFilterCommand>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(containers));

        var imageConnector = new Mock<IImageConnector>();
        imageConnector
            .Setup(connector => connector.ListImagesAsync(
                platform.Address,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<ImageResult>>([image]));

        var containerFactory = Mock.Of<IConnectorFactory<IContainerConnector>>(
            factory => factory.GetConnector(platform.ConnectorType) == containerConnector.Object);
        var imageFactory = Mock.Of<IConnectorFactory<IImageConnector>>(
            factory => factory.GetConnector(platform.ConnectorType) == imageConnector.Object);

        return new ResolverDependencies(
            new ManualStackDeployedImageResolver(
                platformCache.Object,
                containerFactory,
                imageFactory),
            platform,
            imageConnector);
    }

    private static Stack CreateStack()
    {
        var stack = Stack.Create(
            "manual-stack",
            Guid.CreateVersion7(),
            StackSource.WebEditor,
            Guid.CreateVersion7(),
            new ManualStack(
                """
                services:
                  api:
                    image: example/api:latest
                """,
                StackUpdateBehavior.Disabled,
                ProjectName: "manual-stack",
                RegistryId: Guid.CreateVersion7()));
        stack.PartialUpdate(StackReleaseStatus.Healthy);
        return stack;
    }

    private static ManualStackImageCheck CreateCheck(
        Stack stack,
        string serviceName,
        string imageName,
        string repository)
        => new(
            stack,
            serviceName,
            imageName,
            new ImageKey(
                stack.CurrentStackRelease!.Spec.RegistryId!.Value,
                repository,
                "latest"));

    private sealed record ResolverDependencies(
        ManualStackDeployedImageResolver Resolver,
        PlatformCacheEntry Platform,
        Mock<IImageConnector> ImageConnector);
}

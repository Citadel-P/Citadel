using Application.Features.Containers.Queries;
using Application.Features.Stacks.Queries;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Features.Stacks;

public sealed class SwarmStackImportDraftFactoryTests
{
    private static readonly IAdoptionFingerprintService FingerprintService = TestAdoptionFingerprint.Create();

    [Fact]
    public void Create_ShouldDescribeTheWholeDockerStackWithoutMutationWarnings()
    {
        var context = CreateContext(Service("service-1", "sample_web", 2, 2));

        var draft = SwarmStackImportDraftFactory.Create(context, "sample", FingerprintService);

        Assert.Equal("sample", draft.Source.ProjectName);
        Assert.Equal(["service-1"], draft.Source.ContainerIds);
        Assert.Equal("web", Assert.Single(draft.Source.Services).Name);
        Assert.Equal("Imported from Docker Swarm Stack sample.", draft.Draft.Description);
        Assert.Empty(draft.Issues);
    }

    [Fact]
    public void CreateValidation_ShouldRejectSourceWithoutAnyRuntimeService()
    {
        var context = CreateContext(Service("service-1", "sample_web", 1, 1));
        var source = new ComposeProjectSourceAnalysis(
            new global::Domain.Entities.Stacks.ManualStack("services:\n  worker:\n    image: busybox", StackUpdateBehavior.Disabled),
            new Dictionary<string, StackComposeService>
            {
                ["worker"] = new("worker", "busybox", null)
            },
            "source-digest",
            ["services:\n  worker:\n    image: busybox"]);

        var validation = SwarmStackImportDraftFactory.CreateValidation(context, source, FingerprintService);

        Assert.Contains(validation.Issues, issue =>
            issue.Code == "NO_MATCHING_SERVICES"
            && issue.Severity == AdoptionIssueSeverity.Blocker);
    }

    [Fact]
    public async Task LoadContext_ShouldRejectMixedCitadelOwnership()
    {
        var platform = CreatePlatform();
        var external = Service("service-1", "sample_web", 1, 1);
        var owned = Service(
            "service-2",
            "sample_worker",
            1,
            1,
            new Dictionary<string, string>
            {
                ["com.docker.stack.namespace"] = "sample",
                [CitadelLabels.Managed] = "true",
                [CitadelLabels.StackId] = Guid.CreateVersion7().ToString("D")
            });
        var platformRepository = new Mock<IPlatformRepository>();
        platformRepository.Setup(value => value.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var projectionRepository = new Mock<ISwarmProjectionRepository>();
        projectionRepository.Setup(value => value.GetServicesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([
                SwarmServiceProjection.FromObservation(platform.Id, external, DateTimeOffset.UtcNow),
                SwarmServiceProjection.FromObservation(platform.Id, owned, DateTimeOffset.UtcNow)
            ]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platformRepository.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(projectionRepository.Object);
        var connector = CreateConnector([external, owned]);
        var connectorFactory = new Mock<IConnectorFactory<ISwarmConnector>>();
        connectorFactory.Setup(value => value.GetConnector(platform.ConnectorType)).Returns(connector.Object);

        var result = await SwarmStackImportDraftFactory.LoadContextAsync(
            platform.Id,
            "sample",
            unitOfWork.Object,
            connectorFactory.Object,
            Mock.Of<IPermissionService>(),
            CreateAdminContext(),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("already managed", error.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task LoadContext_ShouldAllowServicesWhoseCitadelStackOwnerNoLongerExists()
    {
        var platform = CreatePlatform();
        var orphanedOwnerId = Guid.CreateVersion7();
        var service = Service(
            "service-1",
            "sample_web",
            1,
            1,
            new Dictionary<string, string>
            {
                ["com.docker.stack.namespace"] = "sample",
                [CitadelLabels.Managed] = "true",
                [CitadelLabels.StackId] = orphanedOwnerId.ToString("D")
            });
        var projection = SwarmServiceProjection.FromObservation(platform.Id, service, DateTimeOffset.UtcNow) with
        {
            Ownership = SwarmServiceOwnership.DockerStackExternal,
            StackId = null,
            OwnershipDiagnostic = "The Citadel Stack owner no longer exists. This Docker Stack can be imported."
        };
        var unitOfWork = CreateUnitOfWork(platform, [projection], orphanedOwnerId, ownerExists: false);
        var connector = CreateConnector([service]);
        var connectorFactory = new Mock<IConnectorFactory<ISwarmConnector>>();
        connectorFactory.Setup(value => value.GetConnector(platform.ConnectorType)).Returns(connector.Object);

        var result = await SwarmStackImportDraftFactory.LoadContextAsync(
            platform.Id,
            "sample",
            unitOfWork.Object,
            connectorFactory.Object,
            Mock.Of<IPermissionService>(),
            CreateAdminContext(),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
    }

    [Fact]
    public async Task LoadContext_ShouldRejectOrphanProjectionWhenClaimedOwnerNowExists()
    {
        var platform = CreatePlatform();
        var ownerId = Guid.CreateVersion7();
        var service = Service(
            "service-1",
            "sample_web",
            1,
            1,
            new Dictionary<string, string>
            {
                ["com.docker.stack.namespace"] = "sample",
                [CitadelLabels.Managed] = "true",
                [CitadelLabels.StackId] = ownerId.ToString("D")
            });
        var projection = SwarmServiceProjection.FromObservation(platform.Id, service, DateTimeOffset.UtcNow) with
        {
            Ownership = SwarmServiceOwnership.DockerStackExternal,
            StackId = null
        };
        var unitOfWork = CreateUnitOfWork(platform, [projection], ownerId, ownerExists: true);
        var connector = CreateConnector([service]);
        var connectorFactory = new Mock<IConnectorFactory<ISwarmConnector>>();
        connectorFactory.Setup(value => value.GetConnector(platform.ConnectorType)).Returns(connector.Object);

        var result = await SwarmStackImportDraftFactory.LoadContextAsync(
            platform.Id,
            "sample",
            unitOfWork.Object,
            connectorFactory.Object,
            Mock.Of<IPermissionService>(),
            CreateAdminContext(),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("existing Citadel Stack", error.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ComputeRuntimeFingerprint_ShouldChangeWhenAServiceVersionChanges()
    {
        var first = CreateContext(Service("service-1", "sample_web", 1, 1));
        var second = first with { Services = [first.Services[0] with { VersionIndex = 2 }] };

        Assert.NotEqual(
            SwarmStackImportDraftFactory.ComputeRuntimeFingerprint(first, FingerprintService),
            SwarmStackImportDraftFactory.ComputeRuntimeFingerprint(second, FingerprintService));
    }

    private static SwarmStackImportContext CreateContext(params SwarmServiceResult[] services)
        => new(CreatePlatform(), "sample", services, [], [], []);

    private static Platform CreatePlatform() => new(
        "Swarm",
        "http://localhost.docker",
        0,
        0,
        0,
        1,
        1024,
        "1.0.0",
        null,
        PlatformStatus.Online,
        PlatformConnectorType.Local,
        new DockerSwarmPlatformDescriptor(
            "node-1", "10.0.0.1", "Active", true, 1, 1, "docker", 0, 0, 0, 0, "cluster"));

    private static SwarmServiceResult Service(
        string id,
        string name,
        int running,
        int desired,
        IReadOnlyDictionary<string, string>? labels = null)
        => new(
            id,
            1,
            name,
            "Replicated",
            "nginx:latest",
            running,
            desired,
            "Completed",
            null,
            [],
            [],
            [],
            [],
            labels ?? new Dictionary<string, string> { ["com.docker.stack.namespace"] = "sample" },
            DateTimeOffset.UtcNow,
            DateTimeOffset.UtcNow,
            RuntimeHash: $"hash-{id}");

    private static Mock<ISwarmConnector> CreateConnector(IReadOnlyList<SwarmServiceResult> services)
    {
        var connector = new Mock<ISwarmConnector>();
        connector.Setup(value => value.ListServicesAsync(It.IsAny<ListSwarmServicesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmServiceResult>>(services));
        connector.Setup(value => value.ListNetworksAsync(It.IsAny<ListSwarmNetworksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmNetworkResult>>([]));
        connector.Setup(value => value.ListSecretsAsync(It.IsAny<ListSwarmSecretsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmSecretResult>>([]));
        connector.Setup(value => value.ListConfigsAsync(It.IsAny<ListSwarmConfigsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmConfigResult>>([]));
        return connector;
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(
        Platform platform,
        IReadOnlyList<SwarmServiceProjection> projections,
        Guid claimedOwnerId,
        bool ownerExists)
    {
        var platforms = new Mock<IPlatformRepository>();
        platforms.Setup(value => value.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        var swarm = new Mock<ISwarmProjectionRepository>();
        swarm.Setup(value => value.GetServicesAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(projections);
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(value => value.ExistsAsync(claimedOwnerId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(ownerExists);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.SetupGet(value => value.Swarm).Returns(swarm.Object);
        unitOfWork.SetupGet(value => value.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(value => value.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static IUserContextAccessor CreateAdminContext()
    {
        var current = Mock.Of<IUserContext>(value =>
            value.UserId == Guid.CreateVersion7()
            && value.ActorId == Guid.CreateVersion7()
            && value.IsAdmin
            && value.IsAuthenticated);
        return Mock.Of<IUserContextAccessor>(value => value.Current == current);
    }
}

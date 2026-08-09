using Application.Features.Containers.Queries;
using Application.Features.Stacks.Queries;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Moq;
using System.Text.Json;

namespace Tests.Unit.Application.Features.Stacks;

public sealed class ComposeProjectImportDraftFactoryTests
{
    private static readonly IAdoptionFingerprintService FingerprintService = TestAdoptionFingerprint.Create();

    [Fact]
    public async Task LoadContext_ShouldRejectSwarmPlatformWithoutManagerControlBeforeCallingConnector()
    {
        var context = CreateContext(("api", false, "nginx:1.27"));
        context.Platform.PartialUpdate(
            descriptor: new DockerSwarmPlatformDescriptor(
                NodeID: "node-1",
                NodeAddr: "10.0.0.1",
                LocalNodeState: "Active",
                ControlAvailable: false,
                Nodes: 1,
                Managers: 1,
                DaemonId: "docker",
                ContainerCount: 0,
                ContainersRunning: 0,
                ContainersPaused: 0,
                ContainersStopped: 0,
                ClusterId: "cluster-1"));
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(repository => repository.GetByIdAsync(context.Platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(context.Platform);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(value => value.Platforms).Returns(platforms.Object);
        var connectors = new Mock<IConnectorFactory<IContainerConnector>>();

        var result = await ComposeProjectImportDraftFactory.LoadContextAsync(
            context.Platform.Id,
            context.ProjectName,
            unitOfWork.Object,
            connectors.Object,
            Mock.Of<IContainerAuthorizationService>(),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("connected Swarm manager", error.Message, StringComparison.Ordinal);
        connectors.Verify(
            factory => factory.GetConnector(It.IsAny<PlatformConnectorType>()),
            Times.Never);
    }

    [Fact]
    public void StackOwnership_ShouldRequireManagedLabelAndValidStackId()
    {
        var stackId = Guid.CreateVersion7();
        var valid = new Dictionary<string, string>
        {
            [CitadelLabels.Managed] = "true",
            [CitadelLabels.StackId] = stackId.ToString("D")
        };
        var malformed = new Dictionary<string, string>
        {
            [CitadelLabels.Managed] = "true",
            [CitadelLabels.StackId] = "not-a-guid"
        };

        Assert.True(StackContainerOwnership.TryGetStackId(valid, out var parsed));
        Assert.Equal(stackId, parsed);
        Assert.False(StackContainerOwnership.TryGetStackId(malformed, out _));
        Assert.False(StackContainerOwnership.TryGetStackId(
            new Dictionary<string, string> { [CitadelLabels.StackId] = stackId.ToString("D") },
            out _));
    }

    [Fact]
    public void Create_ShouldExcludeOneOffContainersAndReportThem()
    {
        var context = CreateContext(
            ("api", false, "nginx:1.27"),
            ("job", true, "busybox:1.36"));

        var draft = ComposeProjectImportDraftFactory.Create(context, "sample", FingerprintService);

        var service = Assert.Single(draft.Source.Services);
        Assert.Equal("api", service.Name);
        Assert.Single(draft.Source.ContainerIds);
        Assert.Contains(draft.Issues, issue =>
            issue.Code == "ONE_OFF_CONTAINER_EXCLUDED"
            && issue.Severity == AdoptionIssueSeverity.Warning);
    }

    [Fact]
    public void Create_ShouldDescribeDockerComposeOrigin()
    {
        var context = CreateContext(("api", false, "nginx:1.27"));

        var draft = ComposeProjectImportDraftFactory.Create(context, "sample", FingerprintService);

        Assert.Equal("Imported from Docker Compose project sample.", draft.Draft.Description);
    }

    [Fact]
    public async Task AnalyzeSource_ShouldPreserveReviewedUpdateBehaviorAndForceRuntimeSafety()
    {
        var context = CreateContext(("api", false, "nginx:1.27"));
        var requested = new ManualStack(
            """
            services:
              api:
                image: nginx:1.28
              worker:
                image: busybox:1.36
            """,
            StackUpdateBehavior.StackAutoDeploy,
            ProjectName: "different-project",
            DestroyBeforeDeploy: true);

        var result = await ComposeProjectImportDraftFactory.AnalyzeSourceAsync(
            context,
            "sample",
            StackSource.WebEditor,
            requested,
            null!,
            null!,
            null!,
            null!,
            CancellationToken.None);

        Assert.True(result.IsSuccess(out var analysis));
        var safeSpec = Assert.IsType<ManualStack>(analysis.SafeSpec);
        Assert.Equal("sample", safeSpec.ProjectName);
        Assert.Equal(StackUpdateBehavior.StackAutoDeploy, safeSpec.UpdateBehavior);
        Assert.False(safeSpec.DestroyBeforeDeploy);

        var validation = ComposeProjectImportDraftFactory.CreateValidation(context, analysis, FingerprintService);
        Assert.DoesNotContain(validation.Issues, issue => issue.Severity == AdoptionIssueSeverity.Blocker);
        Assert.Contains(validation.Issues, issue => issue.Code == "SERVICE_IMAGE_DIFFERS");
        Assert.Contains(validation.Issues, issue => issue.Code == "SOURCE_SERVICE_NOT_RUNNING");
    }

    [Fact]
    public void NormalizeImportSpec_ShouldPreserveReviewedGitAutomationSettings()
    {
        var webhook = new StackWebhookConfig(
            Enabled: true,
            Secret: "secret",
            BranchFilter: "main");
        var requested = new GitStack(
            Guid.CreateVersion7(),
            "main",
            null,
            StackUpdateBehavior.StackAutoDeploy,
            ProjectName: "different-project",
            Webhook: webhook,
            DestroyBeforeDeploy: true);

        var normalized = Assert.IsType<GitStack>(
            ComposeProjectImportDraftFactory.NormalizeImportSpec("sample", requested));

        Assert.Equal("sample", normalized.ProjectName);
        Assert.Equal(StackUpdateBehavior.StackAutoDeploy, normalized.UpdateBehavior);
        Assert.Equal(webhook, normalized.Webhook);
        Assert.False(normalized.DestroyBeforeDeploy);
    }

    [Fact]
    public async Task Validation_ShouldBlockSourceWithoutMatchingRuntimeService()
    {
        var context = CreateContext(("api", false, "nginx:1.27"));
        var sourceResult = await ComposeProjectImportDraftFactory.AnalyzeSourceAsync(
            context,
            "sample",
            StackSource.WebEditor,
            new ManualStack(
                """
                services:
                  worker:
                    image: busybox:1.36
                """,
                StackUpdateBehavior.Disabled),
            null!,
            null!,
            null!,
            null!,
            CancellationToken.None);

        Assert.True(sourceResult.IsSuccess(out var source));
        var validation = ComposeProjectImportDraftFactory.CreateValidation(context, source, FingerprintService);

        Assert.Contains(validation.Issues, issue =>
            issue.Code == "NO_MATCHING_SERVICES"
            && issue.Severity == AdoptionIssueSeverity.Blocker);
    }

    [Fact]
    public async Task Validation_ShouldCompareTheConfiguredImageInsteadOfTheComposeContentDigest()
    {
        var context = CreateContext(("api", false, "nginx:1.27"));
        var container = Assert.Single(context.Containers);
        var labels = new Dictionary<string, string>(container.Inspection.Config!.Labels)
        {
            ["com.docker.compose.image"] = "sha256:content-digest"
        };
        var inspection = container.Inspection with
        {
            Config = container.Inspection.Config with { Labels = labels }
        };
        context = new ComposeProjectImportContext(
            context.Platform,
            context.ProjectName,
            [container with { Inspection = inspection }]);
        var source = await Analyze(context, "nginx:1.27");

        var validation = ComposeProjectImportDraftFactory.CreateValidation(context, source, FingerprintService);

        Assert.DoesNotContain(validation.Issues, issue => issue.Code == "SERVICE_IMAGE_DIFFERS");
    }

    [Fact]
    public async Task PreviewFingerprint_ShouldChangeWithAuthoritativeSource()
    {
        var context = CreateContext(("api", false, "nginx:1.27"));
        var first = await Analyze(context, "nginx:1.27");
        var second = await Analyze(context, "nginx:1.28");

        var firstFingerprint = ComposeProjectImportDraftFactory
            .CreateValidation(context, first, FingerprintService)
            .PreviewFingerprint;
        var secondFingerprint = ComposeProjectImportDraftFactory
            .CreateValidation(context, second, FingerprintService)
            .PreviewFingerprint;

        Assert.NotEqual(firstFingerprint, secondFingerprint);
        Assert.True(FingerprintService.Matches(firstFingerprint, firstFingerprint));
        Assert.False(FingerprintService.Matches(firstFingerprint, secondFingerprint));
    }

    [Fact]
    public async Task PreviewFingerprint_ShouldBindReviewedSourceSettings()
    {
        var context = CreateContext(("api", false, "nginx:1.27"));
        var compose = """
                      services:
                        api:
                          image: nginx:1.27
                      """;
        var first = await Analyze(context, new ManualStack(
            compose,
            StackUpdateBehavior.Disabled,
            EnvFilePath: ".env"));
        var second = await Analyze(context, new ManualStack(
            compose,
            StackUpdateBehavior.Disabled,
            EnvFilePath: "production.env"));

        var firstFingerprint = ComposeProjectImportDraftFactory
            .CreateValidation(context, first, FingerprintService)
            .PreviewFingerprint;
        var secondFingerprint = ComposeProjectImportDraftFactory
            .CreateValidation(context, second, FingerprintService)
            .PreviewFingerprint;

        Assert.NotEqual(firstFingerprint, secondFingerprint);
    }

    [Fact]
    public async Task Validation_ShouldOfferSensitiveRuntimeValuesAsStackBindingsWithoutExposingThem()
    {
        const string secretValue = "stripe-secret-value";
        var context = CreateContextWithEnvironment(
            ("api", "nginx:1.27", ["STRIPE_API_KEY=" + secretValue]),
            ("agent", "busybox:1.36", ["TOKEN=agent-token-value"]));
        var source = await Analyze(
            context,
            new ManualStack(
                """
                services:
                  api:
                    image: nginx:1.27
                    environment:
                      STRIPE_API_KEY: ${stripe_api_key_1}
                  agent:
                    image: busybox:1.36
                    environment:
                      - TOKEN=${BESZEL_AGENT_TOKEN}
                """,
                StackUpdateBehavior.Disabled));

        var validation = ComposeProjectImportDraftFactory.CreateValidation(context, source, FingerprintService);
        var sensitiveBindings = ComposeProjectImportDraftFactory.GetSensitiveBindingAnalysis(context, source);

        Assert.True(validation.CanImportSensitiveEnvironmentValues);
        Assert.Equal(["BESZEL_AGENT_TOKEN", "stripe_api_key_1"], validation.ImportableSensitiveEnvironmentNames);
        Assert.Equal(
            ["BESZEL_AGENT_TOKEN", "stripe_api_key_1"],
            sensitiveBindings.Bindings.Select(static binding => binding.Name));
        Assert.DoesNotContain(secretValue, JsonSerializer.Serialize(validation), StringComparison.Ordinal);
    }

    [Fact]
    public async Task Validation_ShouldRejectAutomaticImportWhenReplicasDisagreeOnSensitiveValue()
    {
        var context = CreateContextWithEnvironment(
            ("api", "nginx:1.27", ["TOKEN=first"]),
            ("api", "nginx:1.27", ["TOKEN=second"]));
        var source = await Analyze(
            context,
            new ManualStack(
                """
                services:
                  api:
                    image: nginx:1.27
                    environment:
                      TOKEN: ${API_TOKEN}
                """,
                StackUpdateBehavior.Disabled));

        var validation = ComposeProjectImportDraftFactory.CreateValidation(context, source, FingerprintService);

        Assert.False(validation.CanImportSensitiveEnvironmentValues);
        Assert.Empty(validation.ImportableSensitiveEnvironmentNames);
        Assert.Contains(validation.Issues, issue => issue.Code == "SENSITIVE_BINDING_VALUE_CONFLICT");
    }

    private static async Task<ComposeProjectSourceAnalysis> Analyze(
        ComposeProjectImportContext context,
        string image)
        => await Analyze(
            context,
            new ManualStack(
                $"""
                 services:
                   api:
                     image: {image}
                 """,
                StackUpdateBehavior.Disabled));

    private static async Task<ComposeProjectSourceAnalysis> Analyze(
        ComposeProjectImportContext context,
        ManualStack spec)
    {
        var result = await ComposeProjectImportDraftFactory.AnalyzeSourceAsync(
            context,
            "sample",
            StackSource.WebEditor,
            spec,
            null!,
            null!,
            null!,
            null!,
            CancellationToken.None);
        Assert.True(result.IsSuccess(out var analysis));
        return analysis;
    }

    private static ComposeProjectImportContext CreateContext(
        params (string Service, bool OneOff, string Image)[] services)
    {
        var platform = new Platform(
            "Local",
            "unix:///var/run/docker.sock",
            0,
            0,
            1,
            1,
            1024,
            "1.0.0",
            null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerPlatformDescriptor("docker", 0, 0, 0, 0));
        var containers = services.Select((service, index) =>
        {
            var dockerId = $"container-{index}";
            var container = new Container(
                $"{service.Service}-{index}",
                $"sha256:{index}",
                platform.Id,
                dockerId,
                ContainerStateStatus.Running,
                dockerStack: "sample");
            var labels = new Dictionary<string, string>
            {
                [ComposeLabels.Project] = "sample",
                [ComposeLabels.Service] = service.Service,
                ["com.docker.compose.image"] = service.Image
            };
            if (service.OneOff)
                labels["com.docker.compose.oneoff"] = "True";

            return new ComposeProjectContainerContext(
                container,
                Inspection(dockerId, service.Image, labels),
                service.Service,
                service.OneOff);
        }).ToArray();

        return new ComposeProjectImportContext(platform, "sample", containers);
    }

    private static ComposeProjectImportContext CreateContextWithEnvironment(
        params (string Service, string Image, string[] Environment)[] services)
    {
        var context = CreateContext(services.Select(service => (service.Service, false, service.Image)).ToArray());
        var containers = context.Containers.Select((container, index) => container with
        {
            Inspection = Inspection(
                container.Container.DockerContainerId,
                services[index].Image,
                container.Inspection.Config!.Labels,
                services[index].Environment)
        }).ToArray();
        return new ComposeProjectImportContext(context.Platform, context.ProjectName, containers);
    }

    private static ContainerInspectionInfo Inspection(
        string id,
        string image,
        IReadOnlyDictionary<string, string> labels,
        IReadOnlyList<string>? environment = null)
        => new(
            Id: id,
            Created: "2026-07-29T00:00:00Z",
            Path: null,
            Args: [],
            State: null,
            Image: image,
            ResolvConfPath: null,
            HostnamePath: null,
            HostsPath: null,
            LogPath: null,
            Name: $"/{id}",
            RestartCount: null,
            Driver: null,
            Platform: null,
            MountLabel: null,
            ProcessLabel: null,
            AppArmorProfile: null,
            ExecIDs: [],
            HostConfig: null,
            GraphDriver: null,
            SizeRw: null,
            SizeRootFs: null,
            Mounts: [],
            Config: new ContainerConfiguration(
                Hostname: null,
                Domainname: null,
                User: null,
                AttachStdin: null,
                AttachStdout: null,
                AttachStderr: null,
                ExposedPorts: null,
                Tty: null,
                OpenStdin: null,
                StdinOnce: null,
                Env: environment ?? [],
                Cmd: [],
                Image: image,
                Volumes: null,
                WorkingDir: null,
                Entrypoint: [],
                NetworkDisabled: null,
                MacAddress: null,
                OnBuild: [],
                Labels: labels),
            NetworkSettings: null);
}

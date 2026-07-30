using Application.Features.Containers.Queries;
using Application.Features.Stacks.Queries;
using Application.Services;
using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;

namespace Tests.Unit.Application.Features.Stacks;

public sealed class ComposeProjectImportDraftFactoryTests
{
    private static readonly IAdoptionFingerprintService FingerprintService = TestAdoptionFingerprint.Create();

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
    public async Task AnalyzeSource_ShouldForceSafeSettingsAndCompareServices()
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
        Assert.Equal(StackUpdateBehavior.Disabled, safeSpec.UpdateBehavior);
        Assert.False(safeSpec.DestroyBeforeDeploy);

        var validation = ComposeProjectImportDraftFactory.CreateValidation(context, analysis, FingerprintService);
        Assert.DoesNotContain(validation.Issues, issue => issue.Severity == AdoptionIssueSeverity.Blocker);
        Assert.Contains(validation.Issues, issue => issue.Code == "SERVICE_IMAGE_DIFFERS");
        Assert.Contains(validation.Issues, issue => issue.Code == "SOURCE_SERVICE_NOT_RUNNING");
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

    private static ContainerInspectionInfo Inspection(
        string id,
        string image,
        IReadOnlyDictionary<string, string> labels)
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
                Env: [],
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

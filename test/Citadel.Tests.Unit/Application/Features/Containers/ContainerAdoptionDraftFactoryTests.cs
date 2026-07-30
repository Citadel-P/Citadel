using Application.Features.Containers.Queries;
using Application.Services;
using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;

namespace Tests.Unit.Application.Features.Containers;

public sealed class ContainerAdoptionDraftFactoryTests
{
    private static readonly IAdoptionFingerprintService FingerprintService = TestAdoptionFingerprint.Create();

    [Fact]
    public void Create_ShouldRedactSensitiveEnvironmentAndRejectOwnershipLabels()
    {
        var context = CreateContext(
            environment: ["API_TOKEN=do-not-return", "APP_MODE=production"],
            labels: new Dictionary<string, string>
            {
                ["example.label"] = "preserved",
                [$"{CitadelLabels.Prefix}managed"] = "true"
            });

        var draft = ContainerAdoptionDraftFactory.Create(context, "existing-app", FingerprintService);

        Assert.Contains("API_TOKEN=${API_TOKEN}", draft.Draft.Spec.EnvironmentVariables!);
        Assert.DoesNotContain(draft.Draft.Spec.EnvironmentVariables!, value => value.Contains("do-not-return"));
        Assert.Contains("APP_MODE=production", draft.Draft.Spec.EnvironmentVariables!);
        Assert.Equal("preserved", draft.Draft.Spec.Labels!["example.label"]);
        Assert.DoesNotContain(draft.Draft.Spec.Labels!.Keys, key => key.StartsWith(CitadelLabels.Prefix));
        Assert.Contains(draft.Issues, issue =>
            issue.Code == "SENSITIVE_ENVIRONMENT_VALUE_REQUIRED"
            && issue.Severity == AdoptionIssueSeverity.Warning);
        Assert.Contains(draft.Issues, issue =>
            issue.Code == "CITADEL_OWNERSHIP_LABEL"
            && issue.Severity == AdoptionIssueSeverity.Blocker);
        Assert.True(draft.CanImportSensitiveEnvironmentValues);
    }

    [Fact]
    public void Create_ShouldNotOfferSensitiveImportWhenTheContainerHasNoValue()
    {
        var context = CreateContext(environment: ["API_TOKEN="]);

        var draft = ContainerAdoptionDraftFactory.Create(context, "existing-app", FingerprintService);

        Assert.False(draft.CanImportSensitiveEnvironmentValues);
    }

    [Fact]
    public void Create_ShouldBlockComposeMemberFromStandaloneAdoption()
    {
        var context = CreateContext(
            labels: new Dictionary<string, string>
            {
                [ComposeLabels.Project] = "sample",
                [ComposeLabels.Service] = "api"
            });

        var draft = ContainerAdoptionDraftFactory.Create(context, "sample-api", FingerprintService);

        Assert.Contains(draft.Issues, issue =>
            issue.Code == "COMPOSE_PROJECT_CONTAINER"
            && issue.Severity == AdoptionIssueSeverity.Blocker);
    }

    [Fact]
    public void Create_ShouldAllowSelectingReplacementWhenSourceImageWasPruned()
    {
        var context = CreateContext(
            entrypoint: ["/docker-entrypoint.sh"],
            includeImage: false);

        var draft = ContainerAdoptionDraftFactory.Create(context, "existing-app", FingerprintService);

        var image = Assert.IsType<LocalImage>(draft.Draft.Spec.Image);
        Assert.Empty(image.ImageId);
        Assert.Contains(draft.Issues, issue =>
            issue.Code == "SOURCE_IMAGE_UNAVAILABLE"
            && issue.Severity == AdoptionIssueSeverity.Warning
            && issue.FieldPath == "spec.image.imageId");
        Assert.DoesNotContain(draft.Issues, issue => issue.Code == "IMAGE_DEFAULTS_UNAVAILABLE");
    }

    [Fact]
    public void Fingerprint_ShouldBeDeterministicAndIncludeRuntimeState()
    {
        var context = CreateContext();
        var first = ContainerAdoptionDraftFactory.ComputeFingerprint(context, FingerprintService);
        var second = ContainerAdoptionDraftFactory.ComputeFingerprint(context, FingerprintService);

        context.Container.State = ContainerStateStatus.Exited;
        var changed = ContainerAdoptionDraftFactory.ComputeFingerprint(context, FingerprintService);

        Assert.Equal(first, second);
        Assert.NotEqual(first, changed);
    }

    [Fact]
    public void Fingerprint_ShouldBeBoundToServerKey()
    {
        var context = CreateContext(environment: ["API_TOKEN=guessable-secret"]);

        var first = ContainerAdoptionDraftFactory.ComputeFingerprint(
            context,
            TestAdoptionFingerprint.Create(0x11));
        var second = ContainerAdoptionDraftFactory.ComputeFingerprint(
            context,
            TestAdoptionFingerprint.Create(0x22));

        Assert.NotEqual(first, second);
    }

    [Fact]
    public void Create_ShouldAllowInheritedImageProcessDefaultsAndBlockOverrides()
    {
        var inherited = CreateContext(
            entrypoint: ["/docker-entrypoint.sh"],
            imageEntrypoint: ["/docker-entrypoint.sh"]);
        var overridden = CreateContext(
            entrypoint: ["/custom-entrypoint.sh"],
            imageEntrypoint: ["/docker-entrypoint.sh"]);

        var inheritedDraft = ContainerAdoptionDraftFactory.Create(
            inherited,
            "inherited",
            FingerprintService);
        var overriddenDraft = ContainerAdoptionDraftFactory.Create(
            overridden,
            "overridden",
            FingerprintService);

        Assert.DoesNotContain(inheritedDraft.Issues, issue => issue.Code == "ENTRYPOINT_NOT_PRESERVED");
        Assert.Contains(overriddenDraft.Issues, issue =>
            issue.Code == "ENTRYPOINT_NOT_PRESERVED"
            && issue.Severity == AdoptionIssueSeverity.Blocker);
    }

    [Fact]
    public void Create_ShouldBlockUnsupportedMountPropagation()
    {
        var context = CreateContext(
            mounts:
            [
                new MountPointInfo(
                    Type: "bind",
                    Name: null,
                    Source: "/srv/data",
                    Destination: "/data",
                    Driver: null,
                    Mode: null,
                    RW: true,
                    Propagation: "rshared")
            ]);

        var draft = ContainerAdoptionDraftFactory.Create(context, "mounted", FingerprintService);

        Assert.Contains(draft.Issues, issue =>
            issue.Code == "MOUNT_PROPAGATION_NOT_PRESERVED"
            && issue.Severity == AdoptionIssueSeverity.Blocker);
    }

    [Fact]
    public void Create_ShouldPreserveNginxStopSignal()
    {
        var context = CreateContext(stopSignal: "SIGQUIT");

        var draft = ContainerAdoptionDraftFactory.Create(context, "nginx", FingerprintService);

        Assert.DoesNotContain(draft.Issues, issue => issue.Code == "STOP_SIGNAL_NOT_SUPPORTED");
        Assert.Equal(StopSignal.SIGQUIT, draft.Draft.Spec.LifeCycleSpec?.StopSignal);
    }

    [Theory]
    [InlineData("unless-stopped", ContainerRestartPolicy.UnlessStopped)]
    [InlineData("UnlessStopped", ContainerRestartPolicy.UnlessStopped)]
    [InlineData("on-failure", ContainerRestartPolicy.OnFailure)]
    [InlineData("OnFailure", ContainerRestartPolicy.OnFailure)]
    [InlineData("always", ContainerRestartPolicy.Always)]
    [InlineData("no", ContainerRestartPolicy.No)]
    [InlineData("Empty", ContainerRestartPolicy.No)]
    public void RestartPolicy_ShouldMapDockerAndGeneratedEnumNames(
        string value,
        ContainerRestartPolicy expected)
    {
        Assert.Equal(expected, ContainerAdoptionDraftFactory.MapRestartPolicy(value));
    }

    [Fact]
    public void RestartPolicy_ShouldRejectUnsupportedNames()
        => Assert.Null(ContainerAdoptionDraftFactory.MapRestartPolicy("custom"));

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("private")]
    [InlineData("Private")]
    [InlineData("host")]
    [InlineData("Host")]
    public void CgroupNamespaceMode_ShouldAllowDockerDaemonModes(string? mode)
    {
        Assert.True(ContainerAdoptionDraftFactory.IsSupportedCgroupNamespaceMode(mode));
    }

    [Theory]
    [InlineData(null, true)]
    [InlineData("", true)]
    [InlineData("json-file", true)]
    [InlineData("JsonFile", true)]
    [InlineData("local", true)]
    [InlineData("journald", false)]
    public void LoggingConfiguration_ShouldRecognizeRepresentedDrivers(string? driver, bool expected)
    {
        var configuration = new LogConfiguration(driver, new Dictionary<string, string>());

        Assert.Equal(expected, ContainerAdoptionDraftFactory.IsLoggingConfigurationRepresented(configuration));
    }

    [Fact]
    public void LoggingConfiguration_ShouldRejectCustomOptions()
    {
        var configuration = new LogConfiguration(
            "json-file",
            new Dictionary<string, string> { ["max-size"] = "10m" });

        Assert.False(ContainerAdoptionDraftFactory.IsLoggingConfigurationRepresented(configuration));
    }

    [Fact]
    public void SensitiveEnvironment_ShouldAcceptAccessibleBareBinding()
    {
        var available = new HashSet<string>(["API_TOKEN"], StringComparer.Ordinal);

        Assert.True(ContainerAdoptionDraftFactory.HasResolvedSensitiveEnvironment(
            ["API_TOKEN"],
            "API_TOKEN",
            available));
        Assert.False(ContainerAdoptionDraftFactory.HasResolvedSensitiveEnvironment(
            ["API_TOKEN"],
            "API_TOKEN",
            new HashSet<string>(StringComparer.Ordinal)));
    }

    [Fact]
    public void SensitiveEnvironmentImport_ShouldKeepValuesServerSideAndUseBindingReferences()
    {
        var context = CreateContext(
            environment:
            [
                "RUSTFS_ACCESS_KEY=citadel",
                "RUSTFS_SECRET_KEY=secret=value",
                "RUSTFS_REGION=local"
            ]);

        var values = ContainerAdoptionDraftFactory.GetImportableSensitiveEnvironmentValues(context.Inspection);
        var environment = ContainerAdoptionDraftFactory.UseSensitiveEnvironmentBindingReferences(
            context.Inspection.Config?.Env,
            ContainerAdoptionDraftFactory.GetSensitiveEnvironmentNames(context.Inspection));
        var draft = ContainerAdoptionDraftFactory.Create(context, "rustfs", FingerprintService);

        Assert.Equal("citadel", values["RUSTFS_ACCESS_KEY"]);
        Assert.Equal("secret=value", values["RUSTFS_SECRET_KEY"]);
        Assert.True(draft.CanImportSensitiveEnvironmentValues);
        Assert.Contains("RUSTFS_ACCESS_KEY=${RUSTFS_ACCESS_KEY}", draft.Draft.Spec.EnvironmentVariables!);
        Assert.Contains("RUSTFS_SECRET_KEY=${RUSTFS_SECRET_KEY}", draft.Draft.Spec.EnvironmentVariables!);
        Assert.DoesNotContain("RUSTFS_ACCESS_KEY=", draft.Draft.Spec.EnvironmentVariables!);
        Assert.DoesNotContain("RUSTFS_SECRET_KEY=", draft.Draft.Spec.EnvironmentVariables!);
        Assert.DoesNotContain(values.Values, value => value == ContainerInspectionRedactor.RedactedValue);
        Assert.Contains("RUSTFS_ACCESS_KEY=${RUSTFS_ACCESS_KEY}", environment);
        Assert.Contains("RUSTFS_SECRET_KEY=${RUSTFS_SECRET_KEY}", environment);
        Assert.Contains("RUSTFS_REGION=local", environment);
        Assert.DoesNotContain(environment, entry => entry.Contains("citadel", StringComparison.Ordinal));
        Assert.DoesNotContain(environment, entry => entry.Contains("secret=value", StringComparison.Ordinal));
    }

    [Fact]
    public void ImportedSecretName_ShouldBeValidBoundedAndDeploymentSpecific()
    {
        var firstDeploymentId = Guid.Parse("019faff7-7f5b-7e8d-9cd6-4bd5efb3cc80");
        var secondDeploymentId = Guid.Parse("019faff7-7f5b-7e8d-9cd6-4bd5efb3cc81");

        var first = ContainerAdoptionDraftFactory.BuildImportedSecretName(
            firstDeploymentId,
            new string('a', 100),
            "RUSTFS_SECRET_KEY");
        var second = ContainerAdoptionDraftFactory.BuildImportedSecretName(
            secondDeploymentId,
            new string('a', 100),
            "RUSTFS_SECRET_KEY");

        Assert.Matches("^[A-Z_][A-Z0-9_]*$", first);
        Assert.True(first.Length <= 128);
        Assert.NotEqual(first, second);
    }

    [Theory]
    [InlineData("RUSTFS_SECRET_KEY", true)]
    [InlineData("_API_TOKEN_2", true)]
    [InlineData("2FA_SECRET", false)]
    [InlineData("API-SECRET", false)]
    [InlineData("", false)]
    public void EnvironmentBindingName_ShouldMatchCitadelBindingRules(string name, bool expected)
        => Assert.Equal(expected, ContainerAdoptionDraftFactory.IsValidEnvironmentBindingName(name));

    [Theory]
    [InlineData("/My App", "My-App")]
    [InlineData("@", "app")]
    [InlineData("a", "a-app")]
    public void NormalizeName_ShouldReturnValidIdentifier(string value, string expected)
        => Assert.Equal(expected, ContainerAdoptionDraftFactory.NormalizeName(value));

    private static ContainerAdoptionContext CreateContext(
        IReadOnlyList<string>? environment = null,
        IReadOnlyDictionary<string, string>? labels = null,
        IReadOnlyList<string>? entrypoint = null,
        IReadOnlyList<string>? imageEntrypoint = null,
        IReadOnlyList<MountPointInfo>? mounts = null,
        string? stopSignal = null,
        bool includeImage = true)
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
        var image = new Image(
            "nginx:latest",
            ["nginx:latest"],
            "sha256:image",
            100,
            1,
            platform.Id,
            DateTime.UtcNow);
        var container = new Container(
            "existing-app",
            image.DockerImageId,
            platform.Id,
            "container-id",
            ContainerStateStatus.Running,
            imageId: includeImage ? image.Id : null);

        var inspection = Inspection(
            environment ?? [],
            labels ?? new Dictionary<string, string>(),
            entrypoint ?? [],
            mounts ?? [],
            stopSignal);
        var imageInspection = entrypoint is { Count: > 0 }
            ? new InspectImageResult(
                Id: image.DockerImageId,
                Size: 100,
                Os: "linux",
                Created: "2026-07-29T00:00:00Z",
                Architecture: "amd64",
                Env: [],
                Cmd: [],
                RepoTags: image.Tags,
                Volumes: [],
                ExposedPorts: [],
                Layers: [],
                Labels: new Dictionary<string, string>(),
                Containers: [],
                EntryPoint: imageEntrypoint ?? [])
            : null;

        return new ContainerAdoptionContext(
            container,
            platform,
            inspection,
            includeImage ? image : null,
            includeImage ? imageInspection : null);
    }

    private static ContainerInspectionInfo Inspection(
        IReadOnlyList<string> environment,
        IReadOnlyDictionary<string, string> labels,
        IReadOnlyList<string> entrypoint,
        IReadOnlyList<MountPointInfo> mounts,
        string? stopSignal)
        => new(
            Id: "container-id",
            Created: "2026-07-29T00:00:00Z",
            Path: null,
            Args: [],
            State: null,
            Image: "sha256:image",
            ResolvConfPath: null,
            HostnamePath: null,
            HostsPath: null,
            LogPath: null,
            Name: "/existing-app",
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
            Mounts: mounts,
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
                Env: environment,
                Cmd: [],
                Image: "nginx:latest",
                Volumes: null,
                WorkingDir: null,
                Entrypoint: entrypoint,
                NetworkDisabled: null,
                MacAddress: null,
                OnBuild: [],
                Labels: labels,
                StopSignal: stopSignal),
            NetworkSettings: null);
}

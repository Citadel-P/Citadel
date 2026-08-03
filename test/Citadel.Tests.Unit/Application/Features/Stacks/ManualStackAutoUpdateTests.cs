using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Unit.Application.Features.Stacks;

public class ManualStackAutoUpdateTests
{
    [Fact]
    public void ParseServices_extracts_image_and_injected_service_hash()
    {
        var stackId = Guid.CreateVersion7();
        var releaseId = Guid.CreateVersion7();
        const string compose = """
        services:
          api:
            image: nginx:1.27
          worker:
            image: busybox
        """;

        var services = StackComposeParser.ParseServices(stackId, releaseId, compose);

        Assert.Equal(2, services.Count);
        Assert.Equal("nginx:1.27", services["api"].Image);
        Assert.False(string.IsNullOrWhiteSpace(services["api"].ExpectedConfigHash));
        Assert.Equal("busybox", services["worker"].Image);
    }

    [Fact]
    public void ParseVolumes_extracts_declared_named_external_and_service_references()
    {
        const string compose = """
        services:
          api:
            image: nginx
            volumes:
              - app-data:/var/lib/app
              - ./config:/etc/app:ro
              - /host/logs:/logs
              - cache:/cache
              - /tmp
          worker:
            image: busybox
            volumes:
              - type: volume
                source: external-data
                target: /external
              - type: bind
                source: ./worker
                target: /worker
              - type: tmpfs
                target: /tmp
        volumes:
          app-data:
          external-data:
            external: true
          cache:
            external:
              name: shared-cache
        """;

        var result = StackComposeParser.ParseVolumes(compose);

        Assert.Collection(
            result.DeclaredVolumes.OrderBy(x => x.Name),
            volume =>
            {
                Assert.Equal("app-data", volume.Name);
                Assert.False(volume.IsExternal);
            },
            volume =>
            {
                Assert.Equal("cache", volume.Name);
                Assert.True(volume.IsExternal);
            },
            volume =>
            {
                Assert.Equal("external-data", volume.Name);
                Assert.True(volume.IsExternal);
            });

        Assert.Equal(["app-data", "cache", "external-data"], result.ServiceVolumeReferences.OrderBy(x => x).ToArray());
        Assert.True(result.HasAnonymousVolumes);
    }

    [Fact]
    public void ParseVolumes_ignores_non_volume_sources()
    {
        const string compose = """
        services:
          api:
            image: nginx
            volumes:
              - ./config:/etc/app:ro
              - /var/run/docker.sock:/var/run/docker.sock
              - type: bind
                source: ./data
                target: /data
              - type: tmpfs
                target: /tmp
        """;

        var result = StackComposeParser.ParseVolumes(compose);

        Assert.Empty(result.DeclaredVolumes);
        Assert.Empty(result.ServiceVolumeReferences);
        Assert.False(result.HasAnonymousVolumes);
    }

    [Fact]
    public void ParseVolumeReferences_extracts_named_volumes_and_ignores_bind_mounts()
    {
        var result = StackComposeParser.ParseVolumeReferences(
            [
                "postgres-data:/var/lib/postgresql/data",
                "./config:/etc/app:ro",
                "/host/logs:/logs",
                "cache:/cache",
                "/tmp"
            ]);

        Assert.Empty(result.DeclaredVolumes);
        Assert.Equal(["cache", "postgres-data"], result.ServiceVolumeReferences.OrderBy(x => x).ToArray());
        Assert.True(result.HasAnonymousVolumes);
    }

    [Fact]
    public async Task LoadManualStackChecks_includes_only_deployed_manual_stacks_with_update_policy_and_registry()
    {
        var registryId = Guid.CreateVersion7();
        var eligible = CreateStack(
            "eligible",
            StackUpdateBehavior.Notify,
            registryId,
            StackReleaseStatus.Healthy,
            """
            services:
              api:
                image: nginx:1.27
              pinned:
                image: redis@sha256:abc
            """);
        var disabled = CreateStack("disabled", StackUpdateBehavior.Disabled, registryId, StackReleaseStatus.Healthy);
        var stopped = CreateStack("stopped", StackUpdateBehavior.Notify, registryId, StackReleaseStatus.Stopped);
        var missingRegistry = CreateStack("missing-registry", StackUpdateBehavior.Notify, null, StackReleaseStatus.Healthy);
        var swarm = CreateStack(
            "swarm",
            StackUpdateBehavior.Notify,
            registryId,
            StackReleaseStatus.Healthy,
            platformType: PlatformType.DockerSwarm);

        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAllAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync([eligible, disabled, stopped, missingRegistry, swarm]);

        var uow = new Mock<IUnitOfWork>();
        uow.Setup(x => x.Stacks).Returns(stacks.Object);

        var services = new ServiceCollection()
            .AddScoped(_ => uow.Object)
            .BuildServiceProvider();

        var scheduler = new ImageScanScheduler(services.GetRequiredService<IServiceScopeFactory>());

        var checks = await scheduler.LoadManualStackChecksAsync(CancellationToken.None);

        var check = Assert.Single(checks);
        Assert.Equal(eligible.Id, check.Stack.Id);
        Assert.Equal("api", check.ServiceName);
        Assert.Equal("nginx:1.27", check.ImageName);
    }

    [Fact]
    public void AlertRule_metadata_accepts_service_scoped_stack_update_events()
    {
        var updates = new[]
        {
            new StackImageUpdateItem("api", "nginx:1.27", "sha256:old", "sha256:new")
        };

        Assert.True(AlertTypeMetadata.IsValidInfo(
            AlertType.StackServiceAutoUpdated,
            new StackServiceAutoUpdatedAlertInfo("demo", updates)));
        Assert.True(AlertTypeMetadata.IsValidInfo(
            AlertType.StackServiceAutoDeployFailed,
            new StackServiceAutoDeployFailedAlertInfo("demo", ["api"], "failed")));
    }

    private static Stack CreateStack(
        string name,
        StackUpdateBehavior updateBehavior,
        Guid? registryId,
        StackReleaseStatus status,
        string compose = "services:\n  api:\n    image: nginx:latest\n",
        PlatformType platformType = PlatformType.Docker)
    {
        var platform = CreatePlatform(platformType);
        var stack = Stack.Create(
            name,
            Guid.CreateVersion7(),
            StackSource.WebEditor,
            platform.Id,
            new ManualStack(
                ComposeFile: compose,
                UpdateBehavior: updateBehavior,
                RegistryId: registryId),
            platform: platform);

        stack.PartialUpdate(status);
        return stack;
    }

    private static Platform CreatePlatform(PlatformType platformType) => new(
        name: platformType.ToString(),
        address: "https://platform.test",
        networkCount: 0,
        volumeCount: 0,
        imageCount: 0,
        cpuCount: 1,
        memTotal: 1024,
        serverVersion: null,
        agentVersion: null,
        status: PlatformStatus.Online,
        connectorType: PlatformConnectorType.Agent,
        platformDescriptor: platformType == PlatformType.DockerSwarm
            ? new DockerSwarmPlatformDescriptor(
                "node", "10.0.0.1", "Active", true, 1, 1,
                "daemon", 0, 0, 0, 0)
            : new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));
}

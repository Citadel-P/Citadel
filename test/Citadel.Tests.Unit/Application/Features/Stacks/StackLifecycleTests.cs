using System.Collections.Immutable;
using Application.Features.Stacks.Commands;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.MergePatch;
using LightResults;
using Moq;
using Actor = Domain.Entities.Identity.Actor;

namespace Tests.Unit.Application.Features.Stacks;

public class StackLifecycleTests
{
    [Fact]
    public async Task RenameStack_pins_existing_compose_project_name_before_renaming()
    {
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "beszel",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: Guid.CreateVersion7(),
            spec: new ManualStack(
                ComposeFile: "services:\n  beszel:\n    image: henrygd/beszel\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));
        stack.PartialUpdate(StackReleaseStatus.Healthy);

        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.ExistsAsync(stack.Id, "homelab-monitoring", It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var userContext = new Mock<IUserContextAccessor>();
        userContext
            .Setup(x => x.Current)
            .Returns(Mock.Of<IUserContext>(x => x.ActorId == actorId));

        var handler = new RenameStackHandler(unitOfWork.Object, userContext.Object);

        var result = await handler.Handle(new RenameStack(stack.Id, "homelab-monitoring"), CancellationToken.None);

        Assert.True(result.IsSuccess());
        Assert.Equal("homelab-monitoring", stack.Name);
        var manualStack = Assert.IsType<ManualStack>(stack.CurrentStackRelease?.Spec);
        Assert.Equal("beszel", manualStack.ProjectName);
        Assert.Equal("beszel", StackProjectNameResolver.Resolve(stack));
        stacks.Verify(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()), Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task PatchStack_Should_Preserve_Current_Healthy_Release_Snapshot_Before_Definition_Update()
    {
        var actorId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var originalSpec = new ManualStack(
            ComposeFile: "services:\n  app:\n    image: nginx:1\n",
            UpdateBehavior: StackUpdateBehavior.Disabled);
        var stack = Stack.Create(
            name: "web",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: originalSpec);
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        var currentRelease = stack.CurrentStackRelease!;

        StackRelease? snapshot = null;
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([currentRelease]);
        stacks
            .Setup(x => x.AddReleaseAsync(It.IsAny<StackRelease>(), It.IsAny<CancellationToken>()))
            .Callback<StackRelease, CancellationToken>((release, _) => snapshot = release)
            .ReturnsAsync(1);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(x => x.GetByIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Platform.FromPersistence(
                id: platformId,
                name: "local",
                address: "http://docker.local",
                networkCount: 0,
                volumeCount: 0,
                imageCount: 0,
                cpuCount: 0,
                memTotal: 0,
                status: PlatformStatus.Online,
                connectorType: PlatformConnectorType.Local,
                platformDescriptor: null!));
        platforms
            .Setup(x => x.GetPlatformsWithLatestStatByIdsAsync(
                It.Is<IReadOnlyCollection<Guid>>(ids => ids.SequenceEqual(new[] { platformId })),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Platforms).Returns(platforms.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var userContext = new Mock<IUserContextAccessor>();
        userContext
            .Setup(x => x.Current)
            .Returns(Mock.Of<IUserContext>(x => x.ActorId == actorId));

        var patch = JsonMergePatchDocument<StackPatchModel>.FromJson($$"""
            {
              "platformId": "{{platformId}}",
              "spec": {
                "$type": "WebEditor",
                "composeFile": "services:\n  app:\n    image: nginx:2\n",
                "updateBehavior": "Disabled"
              }
            }
            """);
        var platformHub = new Mock<IPlatformStreamManager>();
        var handler = new PatchStackHandler(unitOfWork.Object, platformHub.Object, userContext.Object);

        var result = await handler.Handle(new PatchStack(stack.Id, patch), CancellationToken.None);

        Assert.True(result.IsSuccess());
        Assert.NotNull(snapshot);
        Assert.NotEqual(currentRelease.Id, snapshot!.Id);
        Assert.Equal(currentRelease.Version, snapshot.Version);
        Assert.Equal(StackReleaseStatus.Healthy, snapshot.Status);
        Assert.Equal(originalSpec.ComposeFile, Assert.IsType<ManualStack>(snapshot.Spec).ComposeFile);
        Assert.Equal("services:\n  app:\n    image: nginx:2\n", Assert.IsType<ManualStack>(stack.CurrentStackRelease!.Spec).ComposeFile);
        Assert.Equal(currentRelease.Id, stack.CurrentStackReleaseId);
        stacks.Verify(x => x.AddReleaseAsync(It.IsAny<StackRelease>(), It.IsAny<CancellationToken>()), Times.Once);
        stacks.Verify(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()), Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DeleteStacks_deletes_owned_runtime_containers_before_removing_stack()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "renamed-beszel",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  beszel:\n    image: henrygd/beszel\n",
                UpdateBehavior: StackUpdateBehavior.Disabled,
                ProjectName: "beszel"));
        stack.PartialUpdate(StackReleaseStatus.Healthy);

        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAllAsync(
                It.Is<IEnumerable<Guid>>(ids => ids.SequenceEqual(new[] { stack.Id })),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([stack]);
        stacks
            .Setup(x => x.RemoveRangeAsync(
                It.Is<IEnumerable<Guid>>(ids => ids.SequenceEqual(new[] { stack.Id })),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(x => x.GetPlatformsWithLatestStatByIdsAsync(
                It.Is<IReadOnlyCollection<Guid>>(ids => ids.SequenceEqual(new[] { platformId })),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        unitOfWork.Setup(x => x.Platforms).Returns(platforms.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var platform = new PlatformCacheEntry(
            platformId,
            "http://docker.local",
            PlatformConnectorType.Local,
            ImmutableDictionary<string, Guid>.Empty);
        var platformCache = new TestPlatformContainerCache(platform);

        var ownedContainer = new DockerContainer(
            Name: "/beszel-beszel-1",
            Image: "henrygd/beszel",
            Id: "owned-container",
            ImageId: "sha256:owned",
            State: ContainerStateStatus.Running,
            Stack: "beszel");
        var unmanagedContainer = new DockerContainer(
            Name: "/beszel-other-1",
            Image: "busybox",
            Id: "unmanaged-container",
            ImageId: "sha256:other",
            State: ContainerStateStatus.Running,
            Stack: "beszel");

        var connector = new Mock<IContainerConnector>();
        connector
            .Setup(x => x.ListContainersAsync(
                It.Is<ContainerFilterCommand>(command =>
                    command.PlatformAddress == "http://docker.local" &&
                    command.All == true &&
                    command.Filters!["label"].ContainsKey($"{ComposeLabels.Project}=beszel")),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                new Dictionary<string, DockerContainer>
                {
                    [ownedContainer.Id] = ownedContainer,
                    [unmanagedContainer.Id] = unmanagedContainer
                }));
        connector
            .Setup(x => x.InspectAsync(
                It.Is<InspectContainerCommand>(command => command.ContainerId == ownedContainer.Id),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(InspectionWithLabels(new Dictionary<string, string>
            {
                [CitadelLabels.Managed] = "true",
                [CitadelLabels.StackId] = stack.Id.ToString("D")
            })));
        connector
            .Setup(x => x.InspectAsync(
                It.Is<InspectContainerCommand>(command => command.ContainerId == unmanagedContainer.Id),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(InspectionWithLabels(new Dictionary<string, string>
            {
                [CitadelLabels.Managed] = "true",
                [CitadelLabels.StackId] = Guid.CreateVersion7().ToString("D")
            })));
        connector
            .Setup(x => x.DeleteAsync(
                It.Is<DeleteContainerCommand>(command =>
                    command.PlatformAddress == "http://docker.local" &&
                    command.Force == true &&
                    command.Volume == false &&
                    command.ContainerIds.SequenceEqual(new[] { ownedContainer.Id })),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());

        var connectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        connectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(connector.Object);
        var stackHub = new Mock<IStackStreamManager>();
        var platformHub = new Mock<IPlatformStreamManager>();
        using var stackStorage = new TestStackStoragePathProvider();
        var gitStoragePath = Path.Combine(stackStorage.StacksRoot, stack.Id.ToString("D"));
        var currentNameStoragePath = Path.Combine(stackStorage.StacksRoot, "renamed-beszel");
        var manualStoragePath = Path.Combine(stackStorage.StacksRoot, "beszel");
        Directory.CreateDirectory(gitStoragePath);
        Directory.CreateDirectory(currentNameStoragePath);
        Directory.CreateDirectory(manualStoragePath);
        await File.WriteAllTextAsync(
            Path.Combine(manualStoragePath, "secret-file"),
            "secret",
            TestContext.Current.CancellationToken);

        var handler = new DeleteStacksHandler(
            unitOfWork.Object,
            platformCache,
            connectorFactory.Object,
            stackHub.Object,
            platformHub.Object,
            stackStorage);

        var result = await handler.Handle(new DeleteStacks([stack.Id]), CancellationToken.None);

        Assert.True(result.IsSuccess());
        Assert.False(Directory.Exists(gitStoragePath));
        Assert.False(Directory.Exists(currentNameStoragePath));
        Assert.False(Directory.Exists(manualStoragePath));
        connector.Verify(x => x.DeleteAsync(It.IsAny<DeleteContainerCommand>(), It.IsAny<CancellationToken>()), Times.Once);
        stacks.Verify(x => x.RemoveRangeAsync(
            It.Is<IEnumerable<Guid>>(ids => ids.SequenceEqual(new[] { stack.Id })),
            It.IsAny<CancellationToken>()), Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        stackHub.Verify(x => x.SendStackInfo(stack, "delete"), Times.Once);
    }

    private static ContainerInspectionInfo InspectionWithLabels(IReadOnlyDictionary<string, string> labels)
        => new(
            Id: "container-id",
            Created: string.Empty,
            Path: null,
            Args: [],
            State: null,
            Image: null,
            ResolvConfPath: null,
            HostnamePath: null,
            HostsPath: null,
            LogPath: null,
            Name: null,
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
                Image: null,
                Volumes: null,
                WorkingDir: null,
                Entrypoint: [],
                NetworkDisabled: null,
                MacAddress: null,
                OnBuild: [],
                Labels: labels),
            NetworkSettings: null);

    private sealed class TestStackStoragePathProvider : IStackStoragePathProvider, IDisposable
    {
        public string StacksRoot { get; } = Path.Combine(Path.GetTempPath(), $"citadel-stack-storage-{Guid.NewGuid():N}");

        public TestStackStoragePathProvider()
        {
            Directory.CreateDirectory(StacksRoot);
        }

        public void Dispose()
        {
            if (Directory.Exists(StacksRoot))
            {
                Directory.Delete(StacksRoot, recursive: true);
            }
        }
    }

    private sealed class TestPlatformContainerCache(PlatformCacheEntry platform) : IPlatformContainerCache
    {
        public void ReplacePlatformContainers(Guid platformId, PlatformCacheEntry cacheEntry)
        {
        }

        public bool TryAddContainer(Guid platformId, string containerId, Guid dbId) => false;

        public bool TryRemoveContainer(Guid platformId, string containerId) => false;

        public bool EvictPlatform(Guid platformId) => false;

        public bool TryGetContainers(Guid platformId, out IReadOnlyDictionary<string, Guid> containers)
        {
            containers = platform.Id == platformId
                ? platform.Containers
                : ImmutableDictionary<string, Guid>.Empty;
            return platform.Id == platformId;
        }

        public bool TryGetCacheEntry(Guid platformId, out PlatformCacheEntry cacheEntry, out Error error)
        {
            if (platform.Id == platformId)
            {
                cacheEntry = platform;
                error = null!;
                return true;
            }

            cacheEntry = null!;
            error = new Error("not found");
            return false;
        }

        public bool TryGetCacheEntries(out IEnumerable<PlatformCacheEntry> cacheEntries, out Error error)
        {
            cacheEntries = [platform];
            error = null!;
            return true;
        }

        public bool TryGetPlatformWithContainer(string containerId, out PlatformCacheEntry cacheEntry)
        {
            cacheEntry = null!;
            return false;
        }

        public bool TryGetPlatformsWithContainers(string[] containersId, out List<PlatformCacheEntry> cacheEntries)
        {
            cacheEntries = [];
            return false;
        }
    }
}

using Application.Features.Stacks.Commands;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.ResourceBindings;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common.Abstraction;
using Moq;

namespace Tests.Unit.Application.Features.Stacks;

public class RollbackStackTests
{
    [Fact]
    public async Task RollbackStack_Should_Prepare_Manual_Release_From_Selected_Snapshot_And_Invoke_Apply()
    {
        var actorId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var buildProjectId = Guid.CreateVersion7();
        var historicalBuildRunId = Guid.CreateVersion7();
        var currentBuildRunId = Guid.CreateVersion7();
        var historicalBinding = new StackBuildImageBinding(
            ServiceName: "app",
            BuildProjectId: buildProjectId,
            ResolvedImageReference: "registry.example.test/app:historical",
            ResolvedDigest: "sha256:historical",
            ResolvedBuildRunId: historicalBuildRunId);
        var oldSpec = new ManualStack(
            ComposeFile: "services:\n  app:\n    image: nginx:1\n",
            UpdateBehavior: StackUpdateBehavior.Disabled,
            BuildImageBindings: [historicalBinding]);
        var newSpec = oldSpec with
        {
            ComposeFile = "services:\n  app:\n    image: nginx:2\n",
            BuildImageBindings =
            [
                historicalBinding with
                {
                    ResolvedImageReference = "registry.example.test/app:current",
                    ResolvedDigest = "sha256:current",
                    ResolvedBuildRunId = currentBuildRunId
                }
            ]
        };
        var stack = Stack.Create(
            name: "manual-stack",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: oldSpec);
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        var oldRelease = stack.CurrentStackRelease!.CreateSnapshot();

        Assert.True(stack.PrepareReleaseForApply(actorId));
        stack.UpdateCurrentStackReleaseDefinition(platformId, newSpec);
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);

        Stack? updatedStack = null;
        var stackRepository = new Mock<IStackRepository>();
        stackRepository
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stackRepository
            .Setup(x => x.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([stack.CurrentStackRelease!, oldRelease]);
        stackRepository
            .Setup(x => x.UpdateAsync(It.IsAny<Stack>(), It.IsAny<CancellationToken>()))
            .Callback<Stack, CancellationToken>((item, _) => updatedStack = item)
            .ReturnsAsync(1);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stackRepository.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var applyStackService = new Mock<IApplyStackService>();
        applyStackService
            .Setup(x => x.ApplyAsync(
                stack.Id,
                actorId,
                null,
                false,
                false,
                true,
                StackApplyOperation.Rollback,
                It.Is<StackSnapshot?>(snapshot => snapshot != null),
                It.IsAny<CancellationToken>()))
            .Returns(SuccessfulApplyStream());

        var userContext = new Mock<IUserContextAccessor>();
        userContext.Setup(x => x.Current).Returns(new TestUserContext(actorId));

        var handler = new RollbackStackHandler(unitOfWork.Object, applyStackService.Object, userContext.Object);

        var items = new List<StackStreamItem>();
        await foreach (var item in handler.Handle(new RollbackStack(stack.Id, oldRelease.Id), TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.NotNull(updatedStack);
        Assert.NotEqual(oldRelease.Id, updatedStack!.CurrentStackReleaseId);
        var rollbackSpec = Assert.IsType<ManualStack>(updatedStack.CurrentStackRelease!.Spec);
        Assert.Equal(oldSpec.ComposeFile, rollbackSpec.ComposeFile);
        Assert.NotEqual(newSpec.ComposeFile, rollbackSpec.ComposeFile);
        var rollbackBinding = Assert.Single(rollbackSpec.BuildImageBindings!);
        Assert.Equal(historicalBinding.ResolvedImageReference, rollbackBinding.ResolvedImageReference);
        Assert.Equal(historicalBinding.ResolvedDigest, rollbackBinding.ResolvedDigest);
        Assert.Equal(historicalBuildRunId, rollbackBinding.ResolvedBuildRunId);
        Assert.NotEqual(currentBuildRunId, rollbackBinding.ResolvedBuildRunId);
        Assert.Contains(
            items,
            item => item.ProgressMessage?.Contains("Rollback release prepared", StringComparison.Ordinal) == true);
        applyStackService.Verify(x => x.ApplyAsync(
            stack.Id,
            actorId,
            null,
            false,
            false,
            true,
            StackApplyOperation.Rollback,
            It.Is<StackSnapshot?>(snapshot => snapshot != null),
            It.IsAny<CancellationToken>()), Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task RollbackStack_Should_Prepare_Git_Release_From_Historical_Source_And_Invoke_Apply()
    {
        var actorId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var repositoryId = Guid.CreateVersion7();
        var oldReleaseSpec = new GitStack(
            GitRepoId: repositoryId,
            Branch: "main",
            CommitSha: null,
            UpdateBehavior: StackUpdateBehavior.Notify,
            ComposePaths: ["stacks/app/compose.yml"],
            WorkingDirectory: "stacks/app",
            ComposeEnvFilesFromRepo: ["stacks/app/.env"],
            WatchPaths: ["stacks/app/**"]);

        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: actorId,
            StackSource: StackSource.Git,
            platformId: platformId,
            spec: oldReleaseSpec);
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);

        var oldRelease = stack.CurrentStackRelease!;
        oldRelease.UpdateSource(new StackReleaseSource(
            SourceType: StackSource.Git,
            GitRepositoryId: repositoryId,
            GitRepositoryName: "homelab",
            Branch: "main",
            RequestedCommitSha: null,
            ResolvedCommitSha: "abc123",
            ComposePaths: ["stacks/app/compose.yml", "stacks/app/compose.prod.yml"],
            EnvFilePaths: ["stacks/app/.env"],
            WorkingDirectory: "stacks/app",
            WatchPaths: ["stacks/app/**", "shared/network.yml"],
            ComposeEnvFilesFromRepo: ["stacks/app/.env"]));

        Assert.True(stack.PrepareReleaseForApply(actorId));
        stack.UpdateCurrentStackReleaseDefinition(
            platformId,
            oldReleaseSpec with
            {
                ComposePaths = ["stacks/app-v2/compose.yml"],
                WorkingDirectory = "stacks/app-v2"
            });
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);

        Stack? updatedStack = null;
        var stackRepository = new Mock<IStackRepository>();
        stackRepository
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stackRepository
            .Setup(x => x.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([stack.CurrentStackRelease!, oldRelease]);
        stackRepository
            .Setup(x => x.UpdateAsync(It.IsAny<Stack>(), It.IsAny<CancellationToken>()))
            .Callback<Stack, CancellationToken>((item, _) => updatedStack = item)
            .ReturnsAsync(1);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stackRepository.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var applyStackService = new Mock<IApplyStackService>();
        applyStackService
            .Setup(x => x.ApplyAsync(
                stack.Id,
                actorId,
                null,
                false,
                false,
                true,
                StackApplyOperation.Rollback,
                It.Is<StackSnapshot?>(snapshot => snapshot != null),
                It.IsAny<CancellationToken>()))
            .Returns(SuccessfulApplyStream());

        var userContext = new Mock<IUserContextAccessor>();
        userContext.Setup(x => x.Current).Returns(new TestUserContext(actorId));

        var handler = new RollbackStackHandler(unitOfWork.Object, applyStackService.Object, userContext.Object);

        var items = new List<StackStreamItem>();
        await foreach (var item in handler.Handle(new RollbackStack(stack.Id, oldRelease.Id), TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.NotNull(updatedStack);
        Assert.NotEqual(oldRelease.Id, updatedStack!.CurrentStackReleaseId);
        Assert.NotEqual(stack.CurrentStackReleaseId, oldRelease.Id);
        var rollbackSpec = Assert.IsType<GitStack>(updatedStack.CurrentStackRelease!.Spec);
        Assert.Equal("abc123", rollbackSpec.CommitSha);
        Assert.Equal(["stacks/app/compose.yml", "stacks/app/compose.prod.yml"], rollbackSpec.ComposePaths);
        Assert.Equal("stacks/app", rollbackSpec.WorkingDirectory);
        Assert.Equal(["stacks/app/.env"], rollbackSpec.ComposeEnvFilesFromRepo);
        Assert.Equal(["stacks/app/**", "shared/network.yml"], rollbackSpec.WatchPaths);
        Assert.Contains(
            items,
            item => item.ProgressMessage?.Contains("Rollback release prepared", StringComparison.Ordinal) == true);
        applyStackService.Verify(x => x.ApplyAsync(
            stack.Id,
            actorId,
            null,
            false,
            false,
            true,
            StackApplyOperation.Rollback,
            It.Is<StackSnapshot?>(snapshot => snapshot != null),
            It.IsAny<CancellationToken>()), Times.Once);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task RollbackStack_SwarmReleaseWithMountedSecret_ShouldFailClosedWithoutApplying()
    {
        var actorId = Guid.CreateVersion7();
        var platform = new Platform(
            "swarm",
            "http://swarm.local",
            0, 0, 0, 4, 1024, "28.0.0", "1.0.0",
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerSwarmPlatformDescriptor(
                "node-1", "10.0.0.1", "Active", true, 1, 1,
                "daemon-1", 0, 0, 0, 0, "cluster-1"),
            clusterId: "cluster-1");
        var stack = Stack.Create(
            "secret-stack",
            actorId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack(
                "services:\n  app:\n    image: nginx\n",
                StackUpdateBehavior.Disabled),
            platform: platform);
        stack.CurrentStackRelease!.UpdateResourceBindings(
        [
            new ResourceBindingSnapshot(
                "API_KEY",
                ResourceBindingKind.Secret,
                ResourceBindingScope.Stack,
                "********",
                Guid.CreateVersion7(),
                SecretDeliveryMode.MountedFile,
                "/run/secrets/api-key")
        ]);
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        var historical = stack.CurrentStackRelease.CreateSnapshot();
        Assert.True(stack.PrepareReleaseForApply(actorId));
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);

        var stacks = new Mock<IStackRepository>();
        stacks.Setup(repository => repository.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks.Setup(repository => repository.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([stack.CurrentStackRelease!, historical]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(value => value.Stacks).Returns(stacks.Object);
        var platforms = new Mock<IPlatformRepository>();
        platforms.Setup(repository => repository.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(platform);
        unitOfWork.Setup(value => value.Platforms).Returns(platforms.Object);
        var apply = new Mock<IApplyStackService>();
        var userContext = new Mock<IUserContextAccessor>();
        userContext.Setup(value => value.Current).Returns(new TestUserContext(actorId));
        var handler = new RollbackStackHandler(unitOfWork.Object, apply.Object, userContext.Object);

        var items = new List<StackStreamItem>();
        await foreach (var item in handler.Handle(
                           new RollbackStack(stack.Id, historical.Id),
                           TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.Contains(items, item => item.Message?.Contains("exact rollback", StringComparison.OrdinalIgnoreCase) == true);
        apply.VerifyNoOtherCalls();
        unitOfWork.Verify(value => value.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task RollbackStack_SwarmReleaseWithRetainedResources_ShouldCopyIdentitiesAndApply()
    {
        var actorId = Guid.CreateVersion7();
        var platform = new Platform(
            "swarm",
            "http://swarm.local",
            0, 0, 0, 4, 1024, "28.0.0", "1.0.0",
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerSwarmPlatformDescriptor(
                "node-1", "10.0.0.1", "Active", true, 1, 1,
                "daemon-1", 0, 0, 0, 0, "cluster-1"),
            clusterId: "cluster-1");
        var stack = Stack.Create(
            "secret-stack",
            actorId,
            StackSource.WebEditor,
            platform.Id,
            new ManualStack("services:\n  api:\n    image: nginx", StackUpdateBehavior.Disabled));
        stack.CurrentStackRelease!.UpdateResourceBindings(
        [
            new ResourceBindingSnapshot(
                "API_KEY",
                ResourceBindingKind.Secret,
                ResourceBindingScope.Stack,
                "********",
                Guid.CreateVersion7(),
                SecretDeliveryMode.MountedFile,
                "/run/secrets/api-key")
        ]);
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        var historical = stack.CurrentStackRelease.CreateSnapshot();
        var retained = new StackReleaseSwarmResource(
            historical.Id,
            platform.Id,
            StackReleaseSwarmResourceKind.Secret,
            "secret-id",
            "secret-stack_citadel-api-key-v1",
            "citadel-api-key-v1",
            [new StackReleaseSwarmResourceMount("api", "api-key")]);
        var retainedConfig = new StackReleaseSwarmResource(
            historical.Id,
            platform.Id,
            StackReleaseSwarmResourceKind.Config,
            "config-id",
            "secret-stack_settings-v1",
            "settings-v1",
            [new StackReleaseSwarmResourceMount("api", "/etc/demo/settings.yml")]);
        Assert.True(stack.PrepareReleaseForApply(actorId));
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);

        IReadOnlyCollection<StackReleaseSwarmResource>? copied = null;
        var stacks = new Mock<IStackRepository>();
        stacks.Setup(repository => repository.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        stacks.Setup(repository => repository.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([stack.CurrentStackRelease!, historical]);
        stacks.Setup(repository => repository.GetReleaseSwarmResourcesAsync(historical.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([retained, retainedConfig]);
        stacks.Setup(repository => repository.UpdateAsync(stack, It.IsAny<CancellationToken>())).ReturnsAsync(1);
        stacks.Setup(repository => repository.ReplaceReleaseSwarmResourcesAsync(
                It.IsAny<Guid>(),
                It.IsAny<IReadOnlyCollection<StackReleaseSwarmResource>>(),
                It.IsAny<CancellationToken>()))
            .Callback<Guid, IReadOnlyCollection<StackReleaseSwarmResource>, CancellationToken>((_, resources, _) => copied = resources)
            .ReturnsAsync(1);
        var platforms = new Mock<IPlatformRepository>();
        platforms.Setup(repository => repository.GetByIdAsync(platform.Id, It.IsAny<CancellationToken>())).ReturnsAsync(platform);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(value => value.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(value => value.Platforms).Returns(platforms.Object);
        unitOfWork.Setup(value => value.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        var apply = new Mock<IApplyStackService>();
        apply.Setup(service => service.ApplyAsync(
                stack.Id,
                actorId,
                null,
                false,
                false,
                true,
                StackApplyOperation.Rollback,
                It.IsAny<StackSnapshot?>(),
                It.IsAny<CancellationToken>()))
            .Returns(SuccessfulApplyStream());
        var userContext = new Mock<IUserContextAccessor>();
        userContext.Setup(value => value.Current).Returns(new TestUserContext(actorId));
        var handler = new RollbackStackHandler(unitOfWork.Object, apply.Object, userContext.Object);

        await foreach (var _ in handler.Handle(
                           new RollbackStack(stack.Id, historical.Id),
                           TestContext.Current.CancellationToken))
        {
        }

        Assert.Equal(2, copied!.Count);
        Assert.All(copied, resource => Assert.Equal(stack.CurrentStackReleaseId, resource.StackReleaseId));
        Assert.Contains(copied, resource => resource.DockerResourceId == retained.DockerResourceId);
        Assert.Contains(copied, resource => resource.DockerResourceId == retainedConfig.DockerResourceId);
        apply.VerifyAll();
    }

    private static async IAsyncEnumerable<StackStreamItem> SuccessfulApplyStream()
    {
        yield return StackStreamItem.SystemMessage("applied", 0);
        await Task.CompletedTask;
    }

    private sealed record TestUserContext(Guid ActorId) : IUserContext
    {
        public Guid UserId { get; init; } = Guid.CreateVersion7();
        public Hosting.Common.AuthenticatedPrincipalType PrincipalType { get; init; } = Hosting.Common.AuthenticatedPrincipalType.User;
        public Guid PrincipalResourceId { get; init; } = Guid.CreateVersion7();
        public Guid? CredentialId { get; init; }
        public bool IsAdmin { get; init; } = true;
        public bool IsAuthenticated { get; init; } = true;
        public string[] Roles { get; init; } = ["admin"];
    }
}

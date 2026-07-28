using Application.Features.Deployments.Commands;
using Application.Features.Stacks.Commands;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.Features;

public sealed class UpdateCheckCommandTests
{
    [Fact]
    public async Task CheckDeploymentUpdates_PersistsAvailableStateAndNotifies()
    {
        var registry = CreateRegistry();
        var deployment = CreateDeployment(registry.Id);
        var dependencies = CreateDeploymentDependencies(deployment, registry);
        dependencies.Scanner
            .Setup(x => x.ScanAsync(It.IsAny<ImageScanTask>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success("sha256:new"));
        dependencies.Deployments
            .Setup(x => x.TryCompleteUpdateCheckAsync(
                deployment.Id,
                It.IsAny<AutoUpdateState>(),
                deployment.RowVersion + 1,
                deployment.PlatformId,
                deployment.Status,
                deployment.Spec!,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var result = await dependencies.Handler.Handle(
            new CheckDeploymentUpdates(deployment.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var updated, out var error), error?.Message);
        Assert.Equal(AutoUpdateStatus.UpdateAvailable, updated.AutoUpdateState!.Status);
        Assert.Equal("sha256:current", updated.AutoUpdateState.CurrentDigest);
        Assert.Equal("sha256:new", updated.AutoUpdateState.RemoteDigest);
        Assert.Equal(ResourceControlState.Idle, updated.ControlState);
        dependencies.Deployments.Verify(
            x => x.UpdateProcessingAsync(
                deployment.Id,
                deployment.Status,
                ResourceControlState.Processing,
                It.IsAny<long>(),
                deployment.RowVersion,
                true,
                Constants.SystemId,
                It.IsAny<CancellationToken>()),
            Times.Once);
        dependencies.Stream.Verify(
            x => x.SendDeploymentInfo(
                It.Is<Deployment>(value =>
                    value.Id == deployment.Id
                    && value.ControlState == ResourceControlState.Processing
                    && value.ControlTriggeredBy == Constants.SystemId
                    && value.ControlStartedAt != null),
                "update"),
            Times.Once);
        dependencies.Notifications.Verify(
            x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Once);
        dependencies.UnitOfWork.Verify(
            x => x.CommitAsync(It.IsAny<CancellationToken>()),
            Times.Exactly(2));
    }

    [Fact]
    public async Task CheckDeploymentUpdates_RemoteFailurePersistsSafeFailedState()
    {
        var registry = CreateRegistry();
        var deployment = CreateDeployment(registry.Id);
        var dependencies = CreateDeploymentDependencies(deployment, registry);
        dependencies.Scanner
            .Setup(x => x.ScanAsync(It.IsAny<ImageScanTask>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<string>(
                new BadGatewayError("The registry update check failed. Verify registry connectivity and credentials.")));
        dependencies.Deployments
            .Setup(x => x.TryCompleteUpdateCheckAsync(
                deployment.Id,
                It.IsAny<AutoUpdateState>(),
                deployment.RowVersion + 1,
                deployment.PlatformId,
                deployment.Status,
                deployment.Spec!,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var result = await dependencies.Handler.Handle(
            new CheckDeploymentUpdates(deployment.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error, out _));
        Assert.IsType<BadGatewayError>(error);
        Assert.Equal(AutoUpdateStatus.Failed, deployment.AutoUpdateState!.Status);
        Assert.Equal("sha256:current", deployment.AutoUpdateState.CurrentDigest);
        Assert.DoesNotContain("token", deployment.AutoUpdateState.LastError ?? string.Empty, StringComparison.OrdinalIgnoreCase);
        dependencies.Notifications.Verify(
            x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task CheckManualStackUpdates_ComparesDeployedDigestsAndScansDuplicateImageOnce()
    {
        var registry = CreateRegistry();
        var stack = Stack.Create(
            "manual-stack",
            Guid.CreateVersion7(),
            StackSource.WebEditor,
            Guid.CreateVersion7(),
            new ManualStack(
                """
                services:
                  api:
                    image: example/app:latest
                  worker:
                    image: example/app:latest
                """,
                StackUpdateBehavior.Disabled,
                RegistryId: registry.Id));
        stack.PartialUpdate(StackReleaseStatus.Healthy);

        var dependencies = CreateStackDependencies(stack);
        dependencies.Registries
            .Setup(x => x.GetAsync(registry.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(registry);
        dependencies.Scanner
            .Setup(x => x.ScanAsync(It.IsAny<ImageScanTask>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success("sha256:remote"));
        dependencies.Stacks
            .Setup(x => x.TryCompleteUpdateCheckAsync(
                stack.Id,
                It.IsAny<StackUpdateState>(),
                stack.RowVersion + 1,
                stack.CurrentStackReleaseId,
                stack.CurrentStackRelease!.Status,
                stack.CurrentStackRelease!.Spec,
                stack.CurrentStackRelease.Source,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var result = await dependencies.Handler.Handle(
            new CheckStackUpdates(stack.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var updated, out var error), error?.Message);
        var state = Assert.IsType<ManualStackUpdateState>(updated.StackUpdateState);
        Assert.Equal(2, state.RecreateStackOnNewImageState.AutoUpdateStates.Count);
        Assert.All(
            state.RecreateStackOnNewImageState.AutoUpdateStates,
            item =>
            {
                Assert.Equal("sha256:deployed", item.CurrentDigest);
                Assert.Equal("sha256:remote", item.RemoteDigest);
                Assert.True(item.UpdateAvailable);
            });
        Assert.Equal(ResourceControlState.Idle, updated.ControlState);
        dependencies.Stacks.Verify(
            x => x.UpdateProcessingAsync(
                stack.Id,
                StackReleaseStatus.Healthy,
                ResourceControlState.Processing,
                It.IsAny<long>(),
                stack.RowVersion,
                true,
                Constants.SystemId,
                It.IsAny<CancellationToken>()),
            Times.Once);
        dependencies.Stream.Verify(
            x => x.SendStackInfo(stack, "update"),
            Times.Once);
        dependencies.Scanner.Verify(
            x => x.ScanAsync(It.IsAny<ImageScanTask>(), It.IsAny<CancellationToken>()),
            Times.Once);
        dependencies.Notifications.Verify(
            x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task CheckManualStackUpdates_MissingDeployedDigestFailsWithoutScanningRegistry()
    {
        var registry = CreateRegistry();
        var stack = Stack.Create(
            "manual-stack",
            Guid.CreateVersion7(),
            StackSource.WebEditor,
            Guid.CreateVersion7(),
            new ManualStack(
                """
                services:
                  api:
                    image: example/app:latest
                """,
                StackUpdateBehavior.Disabled,
                RegistryId: registry.Id));
        stack.PartialUpdate(StackReleaseStatus.Healthy);

        var dependencies = CreateStackDependencies(stack);
        dependencies.Registries
            .Setup(x => x.GetAsync(registry.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(registry);
        dependencies.DeployedImageResolver
            .Setup(x => x.ResolveAsync(
                stack,
                It.IsAny<IReadOnlyList<ManualStackImageCheck>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<IReadOnlyDictionary<string, string>>(
                new ConflictError("The deployed image digest is unavailable.")));

        var result = await dependencies.Handler.Handle(
            new CheckStackUpdates(stack.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error, out _));
        Assert.IsType<ConflictError>(error);
        dependencies.Scanner.Verify(
            x => x.ScanAsync(It.IsAny<ImageScanTask>(), It.IsAny<CancellationToken>()),
            Times.Never);
        dependencies.Stacks.Verify(
            x => x.TryReleaseUpdateCheckAsync(
                stack.Id,
                It.IsAny<long>(),
                Constants.SystemId,
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task CheckGitStackUpdates_DisablesHooksAndPreservesImageState()
    {
        var repository = new GitRepository(
            "repo",
            null,
            "https://example.test/repo.git",
            "main",
            null,
            Guid.CreateVersion7());
        var gitSpec = new GitStack(
            repository.Id,
            "main",
            null,
            StackUpdateBehavior.Disabled,
            ComposePaths: ["compose.yml"]);
        var stack = Stack.Create(
            "git-stack",
            Guid.CreateVersion7(),
            StackSource.Git,
            Guid.CreateVersion7(),
            gitSpec);
        stack.PartialUpdate(StackReleaseStatus.Healthy);
        stack.CurrentStackRelease!.UpdateSource(new StackReleaseSource(
            StackSource.Git,
            repository.Id,
            repository.Name,
            "main",
            null,
            "commit-current",
            ["compose.yml"],
            [],
            repository.Url));
        var imageState = new RecreateStackOnNewImageState(
            [new ImageUpdateState(
                "api",
                "example/app:latest",
                "sha256:old",
                "sha256:new",
                DateTime.UtcNow,
                true)]);
        stack.SetStackUpdateState(new GitStackUpdateState(
            imageState,
            new RecreateStackOnNewCommitState("commit-current", null, DateTime.MinValue)));

        var dependencies = CreateStackDependencies(stack);
        dependencies.GitRepositories
            .Setup(x => x.GetWithAccountAsync(repository.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repository);
        dependencies.RepoCache
            .Setup(x => x.GetRemoteUrl(repository, repository.GitAccount))
            .Returns(repository.Url);
        dependencies.GitCli
            .Setup(x => x.TestConnectionAsync(repository.Url, repository.GitAccount, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        dependencies.RepoCache
            .Setup(x => x.SynchronizeAsync(
                repository,
                repository.GitAccount,
                "main",
                It.Is<RepoSyncOptions>(options => !options.ExecuteHooks),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new RepoSyncResult(
                GitOperation.Pull,
                "commit-remote",
                Success: true,
                CachePath: "repo-cache"));
        dependencies.GitCli
            .Setup(x => x.GetChangedPathsAsync(
                "repo-cache",
                "commit-current",
                "commit-remote",
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<string>>(["compose.yml"]));
        dependencies.Stacks
            .Setup(x => x.TryCompleteUpdateCheckAsync(
                stack.Id,
                It.IsAny<StackUpdateState>(),
                stack.RowVersion + 1,
                stack.CurrentStackReleaseId,
                stack.CurrentStackRelease.Status,
                stack.CurrentStackRelease.Spec,
                stack.CurrentStackRelease.Source,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var result = await dependencies.Handler.Handle(
            new CheckStackUpdates(stack.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var updated, out var error), error?.Message);
        var state = Assert.IsType<GitStackUpdateState>(updated.StackUpdateState);
        Assert.Same(imageState, state.RecreateStackOnNewImageState);
        Assert.Equal("commit-current", state.RecreateStackOnNewCommitState.CurrentCommitSha);
        Assert.Equal("commit-remote", state.RecreateStackOnNewCommitState.RemoteCommitSha);
        dependencies.RepoCache.Verify(x => x.SynchronizeAsync(
            repository,
            repository.GitAccount,
            "main",
            It.IsAny<CancellationToken>()), Times.Never);
        dependencies.Notifications.Verify(
            x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task CheckGitStackUpdates_UnexpectedRemoteExceptionReturnsSafeFailureAndReleasesProcessing()
    {
        var repository = new GitRepository(
            "repo",
            null,
            "https://example.test/repo.git",
            "main",
            null,
            Guid.CreateVersion7());
        var gitSpec = new GitStack(
            repository.Id,
            "main",
            null,
            StackUpdateBehavior.Disabled,
            ComposePaths: ["compose.yml"]);
        var stack = Stack.Create(
            "git-stack",
            Guid.CreateVersion7(),
            StackSource.Git,
            Guid.CreateVersion7(),
            gitSpec);
        stack.PartialUpdate(StackReleaseStatus.Healthy);
        stack.CurrentStackRelease!.UpdateSource(new StackReleaseSource(
            StackSource.Git,
            repository.Id,
            repository.Name,
            "main",
            null,
            "commit-current",
            ["compose.yml"],
            [],
            repository.Url));

        var dependencies = CreateStackDependencies(stack);
        dependencies.GitRepositories
            .Setup(x => x.GetWithAccountAsync(repository.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repository);
        dependencies.RepoCache
            .Setup(x => x.GetRemoteUrl(repository, repository.GitAccount))
            .Returns(repository.Url);
        dependencies.GitCli
            .Setup(x => x.TestConnectionAsync(
                repository.Url,
                repository.GitAccount,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        dependencies.RepoCache
            .Setup(x => x.SynchronizeAsync(
                repository,
                repository.GitAccount,
                "main",
                It.Is<RepoSyncOptions>(options => !options.ExecuteHooks),
                It.IsAny<CancellationToken>()))
            .ThrowsAsync(new IOException("remote response included sensitive details"));

        var result = await dependencies.Handler.Handle(
            new CheckStackUpdates(stack.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error, out _));
        Assert.IsType<BadGatewayError>(error);
        Assert.DoesNotContain("sensitive", error.Message, StringComparison.OrdinalIgnoreCase);
        Assert.Equal(ResourceControlState.Idle, stack.ControlState);
        dependencies.Stacks.Verify(
            x => x.TryCompleteUpdateCheckAsync(
                It.IsAny<Guid>(),
                It.IsAny<StackUpdateState>(),
                It.IsAny<long>(),
                It.IsAny<Guid>(),
                It.IsAny<StackReleaseStatus>(),
                It.IsAny<StackSpec>(),
                It.IsAny<StackReleaseSource?>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        dependencies.Stacks.Verify(
            x => x.TryReleaseUpdateCheckAsync(
                stack.Id,
                It.IsAny<long>(),
                Constants.SystemId,
                It.IsAny<CancellationToken>()),
            Times.Once);
        dependencies.Notifications.Verify(
            x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()),
            Times.Once);
    }

    private static DeploymentDependencies CreateDeploymentDependencies(
        Deployment deployment,
        Registry registry)
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        var deployments = new Mock<IDeploymentRepository>();
        var registries = new Mock<IRegistryRepository>();
        var scanner = new Mock<IImageDigestScanner>();
        var notifications = new Mock<INotificationQueue>();
        var stream = new Mock<IDeploymentStreamManager>();

        deployments
            .Setup(x => x.GetAsync(deployment.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(deployment);
        registries
            .Setup(x => x.GetAsync(registry.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(registry);
        unitOfWork.SetupGet(x => x.Deployments).Returns(deployments.Object);
        unitOfWork.SetupGet(x => x.Registries).Returns(registries.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        deployments
            .Setup(x => x.UpdateProcessingAsync(
                It.IsAny<Guid>(),
                It.IsAny<DeploymentStatus>(),
                It.IsAny<ResourceControlState>(),
                It.IsAny<long?>(),
                It.IsAny<long>(),
                It.IsAny<bool?>(),
                It.IsAny<Guid?>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        deployments
            .Setup(x => x.TryCompleteUpdateCheckAsync(
                It.IsAny<Guid>(),
                It.IsAny<AutoUpdateState>(),
                It.IsAny<long>(),
                It.IsAny<Guid>(),
                It.IsAny<DeploymentStatus>(),
                It.IsAny<DeploymentSpec>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        deployments
            .Setup(x => x.TryReleaseUpdateCheckAsync(
                It.IsAny<Guid>(),
                It.IsAny<long>(),
                It.IsAny<Guid>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        notifications
            .Setup(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);
        stream
            .Setup(x => x.SendDeploymentInfo(It.IsAny<Deployment>(), It.IsAny<string>()))
            .Returns(Task.CompletedTask);

        var handler = new CheckDeploymentUpdatesHandler(
            unitOfWork.Object,
            new ImageCheckBuilder(),
            scanner.Object,
            new DeploymentUpdateEvaluator(),
            new UpdateCheckLeaseManager(),
            CreateUserContext(),
            stream.Object,
            notifications.Object,
            TimeProvider.System);

        return new DeploymentDependencies(
            handler,
            unitOfWork,
            deployments,
            scanner,
            notifications,
            stream);
    }

    private static StackDependencies CreateStackDependencies(Stack stack)
    {
        var unitOfWork = new Mock<IUnitOfWork>();
        var stacks = new Mock<IStackRepository>();
        var registries = new Mock<IRegistryRepository>();
        var gitRepositories = new Mock<IGitReposRepository>();
        var scanner = new Mock<IImageDigestScanner>();
        var deployedImageResolver = new Mock<IManualStackDeployedImageResolver>();
        var repoCache = new Mock<IRepoCacheManager>();
        var gitCli = new Mock<IGitCliRepository>();
        var notifications = new Mock<INotificationQueue>();
        var stream = new Mock<IStackStreamManager>();

        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        unitOfWork.SetupGet(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.SetupGet(x => x.Registries).Returns(registries.Object);
        unitOfWork.SetupGet(x => x.GitRepositories).Returns(gitRepositories.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        stacks
            .Setup(x => x.UpdateProcessingAsync(
                It.IsAny<Guid>(),
                It.IsAny<StackReleaseStatus>(),
                It.IsAny<ResourceControlState>(),
                It.IsAny<long?>(),
                It.IsAny<long>(),
                It.IsAny<bool?>(),
                It.IsAny<Guid?>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);
        stacks
            .Setup(x => x.TryCompleteUpdateCheckAsync(
                It.IsAny<Guid>(),
                It.IsAny<StackUpdateState>(),
                It.IsAny<long>(),
                It.IsAny<Guid>(),
                It.IsAny<StackReleaseStatus>(),
                It.IsAny<StackSpec>(),
                It.IsAny<StackReleaseSource?>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        stacks
            .Setup(x => x.TryReleaseUpdateCheckAsync(
                It.IsAny<Guid>(),
                It.IsAny<long>(),
                It.IsAny<Guid>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        notifications
            .Setup(x => x.EnqueueAsync(It.IsAny<INotificationWorkItem>(), It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);
        stream
            .Setup(x => x.SendStackInfo(It.IsAny<Stack>(), It.IsAny<string>()))
            .Returns(Task.CompletedTask);
        deployedImageResolver
            .Setup(x => x.ResolveAsync(
                It.IsAny<Stack>(),
                It.IsAny<IReadOnlyList<ManualStackImageCheck>>(),
                It.IsAny<CancellationToken>()))
            .Returns((
                Stack _,
                IReadOnlyList<ManualStackImageCheck> checks,
                CancellationToken _) =>
            {
                IReadOnlyDictionary<string, string> digests = checks.ToDictionary(
                    check => ManualStackUpdateEvaluator.StateKey(check.ServiceName, check.ImageName),
                    _ => "sha256:deployed",
                    StringComparer.OrdinalIgnoreCase);
                return Task.FromResult(Result.Success(digests));
            });

        var handler = new CheckStackUpdatesHandler(
            unitOfWork.Object,
            new ImageCheckBuilder(),
            scanner.Object,
            new ManualStackUpdateEvaluator(),
            deployedImageResolver.Object,
            new UpdateCheckLeaseManager(),
            repoCache.Object,
            gitCli.Object,
            CreateUserContext(),
            stream.Object,
            notifications.Object,
            TimeProvider.System,
            NullLogger<CheckStackUpdatesHandler>.Instance);

        return new StackDependencies(
            handler,
            stacks,
            registries,
            gitRepositories,
            scanner,
            deployedImageResolver,
            repoCache,
            gitCli,
            notifications,
            stream);
    }

    private static Registry CreateRegistry()
        => new(
            "registry",
            "registry.example.test",
            RegistryStatus.Active,
            Guid.CreateVersion7(),
            new CustomRegistry());

    private static Deployment CreateDeployment(Guid registryId)
        => new(
            "deployment",
            Guid.CreateVersion7(),
            Guid.CreateVersion7(),
            new DeploymentSpec(
                new ExternalImage(registryId, "example/app:latest", "sha256:current"),
                UpdateBehavior.Disabled));

    private static IUserContextAccessor CreateUserContext()
    {
        var current = Mock.Of<IUserContext>(x => x.ActorId == Constants.SystemId);
        return Mock.Of<IUserContextAccessor>(x => x.Current == current);
    }

    private sealed record DeploymentDependencies(
        CheckDeploymentUpdatesHandler Handler,
        Mock<IUnitOfWork> UnitOfWork,
        Mock<IDeploymentRepository> Deployments,
        Mock<IImageDigestScanner> Scanner,
        Mock<INotificationQueue> Notifications,
        Mock<IDeploymentStreamManager> Stream);

    private sealed record StackDependencies(
        CheckStackUpdatesHandler Handler,
        Mock<IStackRepository> Stacks,
        Mock<IRegistryRepository> Registries,
        Mock<IGitReposRepository> GitRepositories,
        Mock<IImageDigestScanner> Scanner,
        Mock<IManualStackDeployedImageResolver> DeployedImageResolver,
        Mock<IRepoCacheManager> RepoCache,
        Mock<IGitCliRepository> GitCli,
        Mock<INotificationQueue> Notifications,
        Mock<IStackStreamManager> Stream);
}

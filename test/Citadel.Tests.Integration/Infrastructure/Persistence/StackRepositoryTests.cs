using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class StackRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task SwarmNamespaceReservation_ShouldBeAtomicStableAndReleasedWithStack()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var suffix = Guid.NewGuid().ToString("N")[..8];
        Guid platformId;
        Guid firstStackId;
        Guid secondStackId;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = CreatePlatform($"stack-namespace-platform-{suffix}");
            await uow.Platforms.AddAsync(platform, cancellationToken);
            var first = Stack.Create(
                $"stack-namespace-first-{suffix}",
                Constants.SystemId,
                StackSource.WebEditor,
                platform.Id,
                new ManualStack("services: {}", StackUpdateBehavior.Disabled, ProjectName: $"shared-{suffix}"));
            var second = Stack.Create(
                $"stack-namespace-second-{suffix}",
                Constants.SystemId,
                StackSource.WebEditor,
                platform.Id,
                new ManualStack("services: {}", StackUpdateBehavior.Disabled, ProjectName: $"shared-{suffix}"));
            await uow.Stacks.AddAsync(first, cancellationToken);
            await uow.Stacks.AddAsync(second, cancellationToken);
            await uow.CommitAsync(cancellationToken);
            platformId = platform.Id;
            firstStackId = first.Id;
            secondStackId = second.Id;
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            Assert.True(await uow.Stacks.TryReserveSwarmNamespaceAsync(firstStackId, platformId, $"shared-{suffix}", cancellationToken));
            Assert.True(await uow.Stacks.TryReserveSwarmNamespaceAsync(firstStackId, platformId, $"shared-{suffix}", cancellationToken));
            Assert.False(await uow.Stacks.TryReserveSwarmNamespaceAsync(secondStackId, platformId, $"shared-{suffix}", cancellationToken));
            Assert.False(await uow.Stacks.TryReserveSwarmNamespaceAsync(firstStackId, platformId, $"renamed-{suffix}", cancellationToken));
            await uow.CommitAsync(cancellationToken);

            var reservation = await uow.Stacks.GetSwarmNamespaceReservationAsync(firstStackId, cancellationToken);
            Assert.NotNull(reservation);
            Assert.Equal(platformId, reservation.PlatformId);
            Assert.Equal($"shared-{suffix}", reservation.Namespace);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            Assert.Equal(1, await uow.Stacks.RemoveRangeAsync([firstStackId], cancellationToken));
            await uow.CommitAsync(cancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            Assert.True(await uow.Stacks.TryReserveSwarmNamespaceAsync(secondStackId, platformId, $"shared-{suffix}", cancellationToken));
            await uow.CommitAsync(cancellationToken);
        }
    }

    [Fact]
    public async Task TryCompleteUpdateCheckAsync_ShouldReleaseProcessingAndRejectStaleReleaseOrStatus()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var suffix = Guid.NewGuid().ToString("N")[..8];
        Stack stack;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = CreatePlatform($"stack-update-state-platform-{suffix}");
            await uow.Platforms.AddAsync(platform, cancellationToken);

            var repositoryId = Guid.CreateVersion7();
            var spec = new GitStack(
                repositoryId,
                "main",
                null,
                StackUpdateBehavior.Disabled,
                ComposePaths: ["compose.yml"]);
            stack = Stack.Create(
                $"stack-update-state-{suffix}",
                Constants.SystemId,
                StackSource.Git,
                platform.Id,
                spec);
            stack.PartialUpdate(StackReleaseStatus.Healthy);
            stack.CurrentStackRelease!.UpdateSource(new StackReleaseSource(
                StackSource.Git,
                repositoryId,
                "repository",
                "main",
                null,
                "commit-current",
                ["compose.yml"],
                []));
            await uow.Stacks.AddAsync(stack, cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }

        var nextState = new GitStackUpdateState(
            new RecreateStackOnNewImageState([]),
            new RecreateStackOnNewCommitState(
                "commit-current",
                "commit-remote",
                DateTime.UtcNow));

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stored = (await uow.Stacks.GetAsync(stack.Id, cancellationToken))!;
            Assert.True(stored.MarkUpdateCheckProcessing(Constants.SystemId));
            var claimed = await uow.Stacks.UpdateProcessingAsync(
                stored.Id,
                stored.CurrentStackRelease!.Status,
                stored.ControlState,
                stored.ControlStartedAt,
                stored.RowVersion,
                checkRowVersion: true,
                Constants.SystemId,
                cancellationToken);
            var operationRowVersion = stored.RowVersion + 1;

            var affected = await uow.Stacks.TryCompleteUpdateCheckAsync(
                stored.Id,
                nextState,
                operationRowVersion,
                stored.CurrentStackReleaseId,
                stored.CurrentStackRelease.Status,
                stored.CurrentStackRelease!.Spec,
                stored.CurrentStackRelease.Source,
                cancellationToken);
            var stale = await uow.Stacks.TryCompleteUpdateCheckAsync(
                stored.Id,
                nextState,
                operationRowVersion,
                Guid.CreateVersion7(),
                stored.CurrentStackRelease.Status,
                stored.CurrentStackRelease.Spec,
                stored.CurrentStackRelease.Source,
                cancellationToken);
            await uow.Stacks.UpdateReleaseStatusAsync(
                stored.CurrentStackReleaseId,
                StackReleaseStatus.Failed,
                cancellationToken);
            var staleStatus = await uow.Stacks.TryCompleteUpdateCheckAsync(
                stored.Id,
                nextState,
                operationRowVersion,
                stored.CurrentStackReleaseId,
                StackReleaseStatus.Healthy,
                stored.CurrentStackRelease.Spec,
                stored.CurrentStackRelease.Source,
                cancellationToken);
            await uow.CommitAsync(cancellationToken);

            Assert.True(claimed);
            Assert.Equal(1, affected);
            Assert.Equal(0, stale);
            Assert.Equal(0, staleStatus);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stored = (await uow.Stacks.GetAsync(stack.Id, cancellationToken))!;
            var state = Assert.IsType<GitStackUpdateState>(stored.StackUpdateState);

            Assert.Equal("commit-remote", state.RecreateStackOnNewCommitState.RemoteCommitSha);
            Assert.Equal(StackReleaseStatus.Failed, stored.CurrentStackRelease!.Status);
            Assert.Equal(ResourceControlState.Idle, stored.ControlState);
            Assert.Null(stored.ControlStartedAt);
            Assert.Null(stored.ControlTriggeredBy);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stored = (await uow.Stacks.GetAsync(stack.Id, cancellationToken))!;
            Assert.True(stored.MarkUpdateCheckProcessing(Constants.SystemId));
            var claimed = await uow.Stacks.UpdateProcessingAsync(
                stored.Id,
                stored.CurrentStackRelease!.Status,
                stored.ControlState,
                stored.ControlStartedAt,
                stored.RowVersion,
                checkRowVersion: true,
                Constants.SystemId,
                cancellationToken);
            await uow.Stacks.UpdateReleaseStatusAsync(
                stored.CurrentStackReleaseId,
                StackReleaseStatus.Healthy,
                cancellationToken);
            var staleCompletion = await uow.Stacks.TryCompleteUpdateCheckAsync(
                stored.Id,
                nextState,
                stored.RowVersion + 1,
                stored.CurrentStackReleaseId,
                stored.CurrentStackRelease.Status,
                stored.CurrentStackRelease.Spec,
                stored.CurrentStackRelease.Source,
                cancellationToken);
            var released = await uow.Stacks.TryReleaseUpdateCheckAsync(
                stored.Id,
                stored.ControlStartedAt!.Value,
                Constants.SystemId,
                cancellationToken);
            await uow.CommitAsync(cancellationToken);

            Assert.True(claimed);
            Assert.Equal(0, staleCompletion);
            Assert.Equal(1, released);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stored = (await uow.Stacks.GetAsync(stack.Id, cancellationToken))!;

            Assert.Equal(StackReleaseStatus.Healthy, stored.CurrentStackRelease!.Status);
            Assert.Equal(ResourceControlState.Idle, stored.ControlState);
        }
    }

    [Fact]
    public async Task ReplaceReleaseVolumeBindingsAsync_ShouldReplaceExistingBindingWithSameReleaseAndVolumeName()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;
        var suffix = Guid.NewGuid().ToString("N")[..8];

        Guid releaseId;
        Guid platformId;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = CreatePlatform($"stack-volume-replace-platform-{suffix}");
            await uow.Platforms.AddAsync(platform, cancellationToken);

            var stack = Stack.Create(
                $"stack-volume-replace-{suffix}",
                actorId,
                StackSource.WebEditor,
                platform.Id,
                new ManualStack(
                    """
                    services:
                      db:
                        image: postgres
                        volumes:
                          - db-data:/var/lib/postgresql/data
                    volumes:
                      db-data:
                    """,
                    StackUpdateBehavior.Disabled));
            await uow.Stacks.AddAsync(stack, cancellationToken);
            await uow.Stacks.ReplaceReleaseVolumeBindingsAsync(
                stack.CurrentStackReleaseId,
                [
                    new StackReleaseVolumeBinding(
                        stack.CurrentStackReleaseId,
                        platform.Id,
                        "db-data",
                        "db-data",
                        isExternal: false,
                        isAnonymous: false)
                ],
                cancellationToken);

            await uow.CommitAsync(cancellationToken);
            releaseId = stack.CurrentStackReleaseId;
            platformId = platform.Id;
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Stacks.ReplaceReleaseVolumeBindingsAsync(
                releaseId,
                [
                    new StackReleaseVolumeBinding(
                        releaseId,
                        platformId,
                        "db-data",
                        "db-data",
                        isExternal: true,
                        isAnonymous: false)
                ],
                cancellationToken);
            await uow.CommitAsync(cancellationToken);

            var storedBindings = await uow.Stacks.GetReleaseVolumeBindingsAsync(releaseId, cancellationToken);
            var binding = Assert.Single(storedBindings);
            Assert.Equal("db-data", binding.VolumeName);
            Assert.True(binding.IsExternal);
        }
    }

    [Fact]
    public async Task ReleaseSwarmResources_ShouldRoundTripMountsReplaceAndCascadeWithStack()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var suffix = Guid.NewGuid().ToString("N")[..8];
        Guid stackId;
        Guid releaseId;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = CreatePlatform($"stack-swarm-resource-platform-{suffix}");
            await uow.Platforms.AddAsync(platform, cancellationToken);
            var stack = Stack.Create(
                $"stack-swarm-resource-{suffix}",
                Constants.SystemId,
                StackSource.WebEditor,
                platform.Id,
                new ManualStack("services:\n  app:\n    image: nginx", StackUpdateBehavior.Disabled));
            await uow.Stacks.AddAsync(stack, cancellationToken);
            await uow.Stacks.ReplaceReleaseSwarmResourcesAsync(
                stack.CurrentStackReleaseId,
                [
                    new StackReleaseSwarmResource(
                        stack.CurrentStackReleaseId,
                        platform.Id,
                        StackReleaseSwarmResourceKind.Secret,
                        "secret-id-1",
                        "demo_citadel_api_key_v1",
                        "citadel_api_key_v1",
                        [new StackReleaseSwarmResourceMount("app", "api-key")])
                ],
                cancellationToken);
            await uow.CommitAsync(cancellationToken);
            stackId = stack.Id;
            releaseId = stack.CurrentStackReleaseId;
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var byRelease = await uow.Stacks.GetReleaseSwarmResourcesAsync(releaseId, cancellationToken);
            var byStack = await uow.Stacks.GetStackSwarmResourcesAsync(stackId, cancellationToken);
            var resource = Assert.Single(byRelease);
            Assert.Single(byStack);
            Assert.Equal("secret-id-1", resource.DockerResourceId);
            Assert.Equal(new StackReleaseSwarmResourceMount("app", "api-key"), Assert.Single(resource.Mounts));

            await uow.Stacks.ReplaceReleaseSwarmResourcesAsync(releaseId, [], cancellationToken);
            Assert.Empty(await uow.Stacks.GetReleaseSwarmResourcesAsync(releaseId, cancellationToken));
            await uow.Stacks.ReplaceReleaseSwarmResourcesAsync(
                releaseId,
                [
                    new StackReleaseSwarmResource(
                        releaseId,
                        resource.PlatformId,
                        StackReleaseSwarmResourceKind.Config,
                        "config-id-1",
                        "demo_config_v1",
                        "config_v1")
                ],
                cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Stacks.RemoveRangeAsync([stackId], cancellationToken);
            await uow.CommitAsync(cancellationToken);
            Assert.Empty(await uow.Stacks.GetStackSwarmResourcesAsync(stackId, cancellationToken));
        }
    }

    private static Platform CreatePlatform(string name)
        => new(
            name,
            "unix:///var/run/docker.sock",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1024,
            serverVersion: "test",
            agentVersion: null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));
}

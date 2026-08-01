using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class DeploymentRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task GetInfoAsync_ShouldUseConfiguredLocalImageWhenRuntimeImageIsUnavailable()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var suffix = Guid.NewGuid().ToString("N")[..8];

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = CreatePlatform($"deployment-image-platform-{suffix}");
        var image = new Image(
            $"nginx-{suffix}",
            ["nginx:latest"],
            $"sha256:{Guid.NewGuid():N}",
            1024,
            0,
            platform.Id,
            DateTime.UtcNow);
        var deployment = new Deployment(
            $"deployment-image-{suffix}",
            Constants.SystemId,
            platform.Id,
            new DeploymentSpec(new LocalImage(image.Id.ToString()), UpdateBehavior.Disabled));

        await uow.Platforms.AddAsync(platform, cancellationToken);
        await uow.Images.AddOrUpdateAsync(image, cancellationToken);
        await uow.Deployments.AddAsync(deployment, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var detail = await uow.Deployments.GetInfoAsync(deployment.Id, cancellationToken);
        var listed = (await uow.Deployments.GetInfoAsync(cancellationToken))
            .Single(item => item.Id == deployment.Id);

        Assert.NotNull(detail);
        Assert.NotNull(detail.Image);
        Assert.Equal(image.Id, detail.Image.Id);
        Assert.Equal(image.Name, detail.Image.Name);
        Assert.NotNull(listed.Image);
        Assert.Equal(image.Id, listed.Image.Id);
        Assert.Equal(image.Name, listed.Image.Name);
    }

    [Fact]
    public async Task GetInfoAsync_ShouldIgnoreNonUuidLocalImageReference()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var suffix = Guid.NewGuid().ToString("N")[..8];

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = CreatePlatform($"deployment-raw-image-platform-{suffix}");
        var deployment = new Deployment(
            $"deployment-raw-image-{suffix}",
            Constants.SystemId,
            platform.Id,
            new DeploymentSpec(new LocalImage("sha256:unresolved"), UpdateBehavior.Disabled));

        await uow.Platforms.AddAsync(platform, cancellationToken);
        await uow.Deployments.AddAsync(deployment, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var stored = (await uow.Deployments.GetInfoAsync(cancellationToken))
            .Single(item => item.Id == deployment.Id);

        Assert.Null(stored.Image);
    }

    [Fact]
    public async Task UpdateAutoUpdateStateAsync_ShouldPersistAllFields()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var suffix = Guid.NewGuid().ToString("N")[..8];

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = CreatePlatform($"deployment-direct-update-platform-{suffix}");
        var deployment = new Deployment(
            $"deployment-direct-update-{suffix}",
            Constants.SystemId,
            platform.Id,
            new DeploymentSpec(new LocalImage("nginx:latest"), UpdateBehavior.Disabled));

        await uow.Platforms.AddAsync(platform, cancellationToken);
        await uow.Deployments.AddAsync(deployment, cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var checkedAt = DateTime.UtcNow;
        var state = new AutoUpdateState(
            checkedAt,
            AutoUpdateStatus.Failed,
            "sha256:current",
            "sha256:remote",
            "registry unavailable");

        var affected = await uow.Deployments.UpdateAutoUpdateStateAsync(
            deployment.Id,
            state,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        var stored = await uow.Deployments.GetAsync(deployment.Id, cancellationToken);

        Assert.Equal(1, affected);
        Assert.NotNull(stored?.AutoUpdateState);
        Assert.Equal(AutoUpdateStatus.Failed, stored.AutoUpdateState.Status);
        Assert.Equal("sha256:current", stored.AutoUpdateState.CurrentDigest);
        Assert.Equal("sha256:remote", stored.AutoUpdateState.RemoteDigest);
        Assert.Equal("registry unavailable", stored.AutoUpdateState.LastError);
        Assert.Equal(checkedAt, stored.AutoUpdateState.LastCheckedAt, TimeSpan.FromMilliseconds(1));
    }

    [Fact]
    public async Task TryCompleteUpdateCheckAsync_ShouldReleaseProcessingAndRejectStaleConfigurationOrStatus()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var suffix = Guid.NewGuid().ToString("N")[..8];
        Deployment deployment;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = CreatePlatform($"deployment-update-state-platform-{suffix}");
            await uow.Platforms.AddAsync(platform, cancellationToken);

            deployment = new Deployment(
                $"deployment-update-state-{suffix}",
                Constants.SystemId,
                platform.Id,
                new DeploymentSpec(
                    new ExternalImage(
                        Constants.DefaultRegistryId,
                        "example/app:latest",
                        "sha256:current"),
                    UpdateBehavior.Disabled));
            await uow.Deployments.AddAsync(deployment, cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }

        var nextState = new AutoUpdateState(
            DateTime.UtcNow,
            AutoUpdateStatus.UpdateAvailable,
            "sha256:current",
            "sha256:remote");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stored = (await uow.Deployments.GetAsync(deployment.Id, cancellationToken))!;
            var startedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var claimed = await uow.Deployments.UpdateProcessingAsync(
                stored.Id,
                stored.Status,
                ResourceControlState.Processing,
                startedAt,
                stored.RowVersion,
                checkRowVersion: true,
                Constants.SystemId,
                cancellationToken);
            var operationRowVersion = stored.RowVersion + 1;

            var affected = await uow.Deployments.TryCompleteUpdateCheckAsync(
                stored.Id,
                nextState,
                operationRowVersion,
                stored.PlatformId,
                stored.Status,
                stored.Spec!,
                cancellationToken);
            var stale = await uow.Deployments.TryCompleteUpdateCheckAsync(
                stored.Id,
                nextState,
                operationRowVersion,
                stored.PlatformId,
                stored.Status,
                stored.Spec! with { UpdateBehavior = UpdateBehavior.Notify },
                cancellationToken);
            await uow.Deployments.UpdateStatusAsync(
                [stored.Id],
                DeploymentStatus.Degraded,
                cancellationToken);
            var staleStatus = await uow.Deployments.TryCompleteUpdateCheckAsync(
                stored.Id,
                nextState,
                operationRowVersion,
                stored.PlatformId,
                DeploymentStatus.Created,
                stored.Spec!,
                cancellationToken);
            await uow.CommitAsync(cancellationToken);

            Assert.Equal(1, claimed);
            Assert.Equal(1, affected);
            Assert.Equal(0, stale);
            Assert.Equal(0, staleStatus);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stored = (await uow.Deployments.GetAsync(deployment.Id, cancellationToken))!;

            Assert.Equal(AutoUpdateStatus.UpdateAvailable, stored.AutoUpdateState!.Status);
            Assert.Equal("sha256:remote", stored.AutoUpdateState.RemoteDigest);
            Assert.Equal(DeploymentStatus.Degraded, stored.Status);
            Assert.Equal(UpdateBehavior.Disabled, stored.Spec!.UpdateBehavior);
            Assert.Equal(ResourceControlState.Idle, stored.ControlState);
            Assert.Null(stored.ControlStartedAt);
            Assert.Null(stored.ControlTriggeredBy);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stored = (await uow.Deployments.GetAsync(deployment.Id, cancellationToken))!;
            var startedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
            var claimed = await uow.Deployments.UpdateProcessingAsync(
                stored.Id,
                stored.Status,
                ResourceControlState.Processing,
                startedAt,
                stored.RowVersion,
                checkRowVersion: true,
                Constants.SystemId,
                cancellationToken);
            await uow.Deployments.UpdateStatusAsync(
                [stored.Id],
                DeploymentStatus.Failed,
                cancellationToken);
            var staleCompletion = await uow.Deployments.TryCompleteUpdateCheckAsync(
                stored.Id,
                nextState,
                stored.RowVersion + 1,
                stored.PlatformId,
                stored.Status,
                stored.Spec!,
                cancellationToken);
            var released = await uow.Deployments.TryReleaseUpdateCheckAsync(
                stored.Id,
                startedAt,
                Constants.SystemId,
                cancellationToken);
            await uow.CommitAsync(cancellationToken);

            Assert.Equal(1, claimed);
            Assert.Equal(0, staleCompletion);
            Assert.Equal(1, released);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stored = (await uow.Deployments.GetAsync(deployment.Id, cancellationToken))!;

            Assert.Equal(DeploymentStatus.Failed, stored.Status);
            Assert.Equal(ResourceControlState.Idle, stored.ControlState);
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

using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class DeploymentRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
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

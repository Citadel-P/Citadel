using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class StackWebhookDeployQueueRepositoryTests(PostgresTestFixture fixture)
    : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task QueueItem_CanBeClaimedRetriedAndRecoveredAfterInterruption()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var now = DateTime.UtcNow;
        StackWebhookDeployQueueItem item;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var repository = new GitRepository(
                $"webhook-queue-repository-{Guid.NewGuid():N}",
                null,
                "https://example.test/repository.git",
                "main",
                null,
                Constants.SystemId);
            var platform = CreatePlatform($"webhook-queue-platform-{Guid.NewGuid():N}");
            var spec = new GitStack(
                repository.Id,
                "main",
                CommitSha: null,
                StackUpdateBehavior.StackAutoDeploy,
                Webhook: new StackWebhookConfig(Enabled: true));
            var stack = Stack.Create(
                $"webhook-queue-stack-{Guid.NewGuid():N}",
                Constants.SystemId,
                StackSource.Git,
                platform.Id,
                spec);
            item = StackWebhookDeployQueueItem.Create(
                stack.Id,
                repository.Id,
                stack.CurrentStackReleaseId,
                "main",
                StackWebhookDeployFingerprint.Compute(spec),
                "0123456789abcdef",
                now);

            await uow.Platforms.AddAsync(platform, cancellationToken);
            await uow.GitRepositories.AddAsync(repository, cancellationToken);
            await uow.Stacks.AddAsync(stack, cancellationToken);
            await uow.StackWebhookDeployQueue.AddAsync(item, cancellationToken);
            await uow.CommitAsync(cancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            Assert.Contains(
                item.Id,
                await uow.StackWebhookDeployQueue.GetReadyIdsAsync(10, now, cancellationToken));

            var claimed = await uow.StackWebhookDeployQueue.TryClaimAsync(
                item.Id,
                now,
                cancellationToken);
            Assert.NotNull(claimed);
            Assert.Equal(StackWebhookDeployQueueStatus.Processing, claimed.Status);
            Assert.Equal(1, claimed.Attempts);

            var availableAt = now.AddMinutes(1);
            Assert.Equal(
                1,
                await uow.StackWebhookDeployQueue.RetryAsync(
                    item.Id,
                    "temporary failure",
                    availableAt,
                    cancellationToken));
            await uow.CommitAsync(cancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            Assert.Empty(await uow.StackWebhookDeployQueue.GetReadyIdsAsync(
                10,
                now.AddSeconds(30),
                cancellationToken));
            Assert.Contains(
                item.Id,
                await uow.StackWebhookDeployQueue.GetReadyIdsAsync(
                    10,
                    now.AddMinutes(1),
                    cancellationToken));

            var claimed = await uow.StackWebhookDeployQueue.TryClaimAsync(
                item.Id,
                now.AddMinutes(1),
                cancellationToken);
            Assert.Equal(2, claimed?.Attempts);
            Assert.Equal(
                1,
                await uow.StackWebhookDeployQueue.RequeueInterruptedAsync(
                    now.AddMinutes(2),
                    cancellationToken));
            await uow.CommitAsync(cancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            Assert.Contains(
                item.Id,
                await uow.StackWebhookDeployQueue.GetReadyIdsAsync(
                    10,
                    now.AddMinutes(2),
                    cancellationToken));
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

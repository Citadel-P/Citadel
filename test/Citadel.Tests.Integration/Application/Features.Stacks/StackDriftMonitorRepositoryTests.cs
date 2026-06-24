using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;
using Stack = Domain.Entities.Stacks.Stack;

namespace Tests.Integration.Application.Features.Stacks;

public class StackDriftMonitorRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task GetDriftMonitorStacksAsync_returns_only_deployed_stacks_with_drift_monitoring_enabled()
    {
        var platform = Fakes.GetDummyPlatform();
        var eligibleHealthy = CreateStack("eligible-healthy", platform.Id, StackReleaseStatus.Healthy);
        var eligibleDegraded = CreateStack("eligible-degraded", platform.Id, StackReleaseStatus.Degraded);
        var intentionallyPaused = CreateStack("intentionally-paused", platform.Id, StackReleaseStatus.Paused);
        var intentionallyStopped = CreateStack("intentionally-stopped", platform.Id, StackReleaseStatus.Stopped);
        var failedFirstDeploy = CreateStack("failed-first-deploy", platform.Id, StackReleaseStatus.Failed);
        var notYetDeployed = CreateStack("not-yet-deployed", platform.Id, StackReleaseStatus.Created);
        var disabledPolicy = CreateStack(
            "disabled-policy",
            platform.Id,
            StackReleaseStatus.Healthy,
            new StackDriftPolicy(
                StackDriftMode.Disabled,
                AlertOnDrift: false,
                MarkDegraded: false,
                AutoStartStoppedContainers: false,
                AutoResumePausedContainers: false,
                RemoveExtraContainers: false));

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
            foreach (var stack in new[]
                         {
                             eligibleHealthy,
                             eligibleDegraded,
                             intentionallyPaused,
                             intentionallyStopped,
                             failedFirstDeploy,
                             notYetDeployed,
                             disabledPolicy
                     })
            {
                await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
            }

            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stacks = await uow.Stacks.GetDriftMonitorStacksAsync(TestContext.Current.CancellationToken);
            var stackIds = stacks.Select(x => x.Id).ToHashSet();

            Assert.Contains(eligibleHealthy.Id, stackIds);
            Assert.Contains(eligibleDegraded.Id, stackIds);
            Assert.DoesNotContain(intentionallyPaused.Id, stackIds);
            Assert.DoesNotContain(intentionallyStopped.Id, stackIds);
            Assert.DoesNotContain(failedFirstDeploy.Id, stackIds);
            Assert.DoesNotContain(notYetDeployed.Id, stackIds);
            Assert.DoesNotContain(disabledPolicy.Id, stackIds);
        }
    }

    private static Stack CreateStack(
        string name,
        Guid platformId,
        StackReleaseStatus status,
        StackDriftPolicy? driftPolicy = null)
    {
        var stack = Stack.Create(
            name,
            Constants.SystemId,
            StackSource.WebEditor,
            platformId,
            new ManualStack(
                ComposeFile: "services:\n  api:\n    image: nginx\n",
                UpdateBehavior: StackUpdateBehavior.Disabled),
            driftPolicy: driftPolicy);

        stack.PartialUpdate(status);
        return stack;
    }
}

using Application.Configs;
using Application.Services.Builds;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Entities.Builds;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class BuildRunWorkerJobTests
{
    [Fact]
    public async Task Worker_ShouldExecuteQueuedRunsConcurrently()
    {
        var runs = new[] { CreateRun(), CreateRun() };
        var buildRuns = new Mock<IBuildRunRepository>();
        buildRuns
            .SetupSequence(repository => repository.GetQueuedAsync(
                It.IsAny<int>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(runs)
            .ReturnsAsync([]);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(uow => uow.BuildRuns).Returns(buildRuns.Object);

        var releaseRuns = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var bothStarted = new TaskCompletionSource(
            TaskCreationOptions.RunContinuationsAsynchronously);
        var currentConcurrency = 0;
        var maximumConcurrency = 0;
        var startedCount = 0;
        var executionService = new Mock<IBuildRunExecutionService>();
        executionService
            .Setup(service => service.ExecuteAsync(
                It.IsAny<Guid>(),
                It.IsAny<CancellationToken>()))
            .Returns<Guid, CancellationToken>((_, cancellationToken) =>
                new ValueTask<Result>(ExecuteAsync(cancellationToken)));

        var services = new ServiceCollection();
        services.AddScoped(_ => unitOfWork.Object);
        services.AddScoped(_ => executionService.Object);
        await using var provider = services.BuildServiceProvider();
        var worker = new BuildRunWorkerJob(
            provider.GetRequiredService<IServiceScopeFactory>(),
            Options.Create(new BuildOptions { MaxParallelRuns = 2 }),
            Mock.Of<ILogger<BuildRunWorkerJob>>());

        await worker.StartAsync(TestContext.Current.CancellationToken);
        await bothStarted.Task.WaitAsync(
            TimeSpan.FromSeconds(5),
            TestContext.Current.CancellationToken);

        Assert.Equal(2, Volatile.Read(ref maximumConcurrency));

        releaseRuns.TrySetResult();
        await worker.StopAsync(TestContext.Current.CancellationToken);

        async Task<Result> ExecuteAsync(CancellationToken cancellationToken)
        {
            var current = Interlocked.Increment(ref currentConcurrency);
            UpdateMaximum(current);
            if (Interlocked.Increment(ref startedCount) == runs.Length)
                bothStarted.TrySetResult();

            try
            {
                await releaseRuns.Task.WaitAsync(cancellationToken);
                return Result.Success();
            }
            finally
            {
                Interlocked.Decrement(ref currentConcurrency);
            }
        }

        void UpdateMaximum(int value)
        {
            var observed = Volatile.Read(ref maximumConcurrency);
            while (value > observed)
            {
                var previous = Interlocked.CompareExchange(
                    ref maximumConcurrency,
                    value,
                    observed);
                if (previous == observed)
                    return;

                observed = previous;
            }
        }
    }

    private static BuildRun CreateRun()
        => new(
            buildProjectId: Guid.CreateVersion7(),
            projectNameSnapshot: "project",
            gitRepositoryId: Guid.CreateVersion7(),
            gitRepositoryNameSnapshot: "repository",
            branch: "main",
            resolvedCommitSha: null,
            contextPath: ".",
            dockerfilePath: "Dockerfile",
            target: null,
            buildArgsSnapshot: [],
            buildSecretIdsSnapshot: [],
            platformSnapshot: new BuildPlatformSnapshot(
                Guid.CreateVersion7(),
                "local",
                "unix:///var/run/docker.sock",
                PlatformConnectorType.Local),
            registrySnapshot: new BuildRegistrySnapshot(
                Guid.CreateVersion7(),
                "registry",
                "registry.example.test"),
            imageRepository: "citadel/test",
            tagTemplatesSnapshot: ["latest"],
            imageReferences: ["registry.example.test/citadel/test:latest"],
            trigger: BuildRunTrigger.Manual,
            triggerSourceId: null,
            triggeredByActorId: Constants.SystemId,
            timeoutSeconds: BuildProject.DefaultTimeoutSeconds);
}

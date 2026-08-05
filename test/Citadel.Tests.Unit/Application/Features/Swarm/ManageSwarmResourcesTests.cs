using Application.Features.Swarm.Commands;
using Application.TaskJobs;
using Hosting.Common.ErrorTypes;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Features.Swarm;

public sealed class ManageSwarmResourcesTests
{
    [Fact]
    public async Task ExecuteAndRefreshAsync_ShouldReconcileAfterMutationFailure()
    {
        var platformId = Guid.CreateVersion7();
        var coordinator = CreateCoordinator();

        var result = await SwarmMutationExecution.ExecuteAndRefreshAsync(
            () => Task.FromResult(Result.Failure(new ConflictError("Docker rejected the mutation."))),
            coordinator.Object,
            platformId);

        Assert.True(result.IsFailure(out var error));
        Assert.NotNull(error);
        Assert.Equal("Docker rejected the mutation.", error.Message);
        coordinator.Verify(value => value.RefreshAsync(
            platformId,
            It.Is<CancellationToken>(token => !token.CanBeCanceled)), Times.Once);
    }

    [Fact]
    public async Task ExecuteAndRefreshAsync_ShouldReconcileAfterAmbiguousCancellation()
    {
        var platformId = Guid.CreateVersion7();
        var coordinator = CreateCoordinator();
        using var cancelled = new CancellationTokenSource();
        cancelled.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() =>
            SwarmMutationExecution.ExecuteAndRefreshAsync(
                () => Task.FromCanceled<Result>(cancelled.Token),
                coordinator.Object,
                platformId));

        coordinator.Verify(value => value.RefreshAsync(
            platformId,
            It.Is<CancellationToken>(token => !token.CanBeCanceled)), Times.Once);
    }

    private static Mock<ISwarmReconciliationCoordinator> CreateCoordinator()
    {
        var coordinator = new Mock<ISwarmReconciliationCoordinator>();
        coordinator
            .Setup(value => value.RefreshAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        return coordinator;
    }
}

using Application.Features.Stacks.Queries;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Moq;

namespace Tests.Unit.Application.Features.Stacks;

public class GetStackReleasesTests
{
    [Fact]
    public async Task GetStackReleases_Should_Return_Only_Previous_Healthy_Releases()
    {
        var actorId = Guid.CreateVersion7();
        var stack = CreateStack(actorId);
        var previousHealthyRelease = stack.CurrentStackRelease!;
        previousHealthyRelease.UpdateStackStatus(StackReleaseStatus.Healthy);

        stack.PrepareReleaseForApply(actorId);
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        var currentHealthyRelease = stack.CurrentStackRelease!;

        var failedRelease = StackRelease.Create(
            stackId: stack.Id,
            platformId: previousHealthyRelease.PlatformId,
            spec: previousHealthyRelease.Spec,
            createdByActorId: actorId,
            version: "3");
        failedRelease.UpdateStackStatus(StackReleaseStatus.Failed);

        var degradedRelease = StackRelease.Create(
            stackId: stack.Id,
            platformId: previousHealthyRelease.PlatformId,
            spec: previousHealthyRelease.Spec,
            createdByActorId: actorId,
            version: "4");
        degradedRelease.UpdateStackStatus(StackReleaseStatus.Degraded);

        var stackRepository = new Mock<IStackRepository>();
        stackRepository
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stackRepository
            .Setup(x => x.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([currentHealthyRelease, failedRelease, degradedRelease, previousHealthyRelease]);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stackRepository.Object);

        var handler = new GetStackReleasesHandler(unitOfWork.Object);

        var result = await handler.Handle(new GetStackReleases(stack.Id), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var releases, out var error), error?.Message);
        var release = Assert.Single(releases);
        Assert.Equal(previousHealthyRelease.Id, release.Id);
    }

    private static Stack CreateStack(Guid actorId)
        => Stack.Create(
            name: "stack",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: Guid.CreateVersion7(),
            spec: new ManualStack(
                ComposeFile: "services:\n  app:\n    image: nginx\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));
}

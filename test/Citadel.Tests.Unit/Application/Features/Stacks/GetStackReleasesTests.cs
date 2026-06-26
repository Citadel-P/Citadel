using Application.Features.Stacks.Queries;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Moq;

namespace Tests.Unit.Application.Features.Stacks;

public class GetStackReleasesTests
{
    [Fact]
    public async Task GetStackReleases_Should_Return_Only_Successfully_Applied_Releases()
    {
        var actorId = Guid.CreateVersion7();
        var stack = CreateStack(actorId);
        var healthyRelease = stack.CurrentStackRelease!;
        healthyRelease.UpdateStackStatus(StackReleaseStatus.Healthy);

        var failedRelease = StackRelease.Create(
            stackId: stack.Id,
            platformId: healthyRelease.PlatformId,
            spec: healthyRelease.Spec,
            createdByActorId: actorId,
            version: "2");
        failedRelease.UpdateStackStatus(StackReleaseStatus.Failed);

        var stackRepository = new Mock<IStackRepository>();
        stackRepository
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stackRepository
            .Setup(x => x.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([failedRelease, healthyRelease]);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stackRepository.Object);

        var handler = new GetStackReleasesHandler(unitOfWork.Object);

        var result = await handler.Handle(new GetStackReleases(stack.Id), TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var releases, out var error), error?.Message);
        var release = Assert.Single(releases);
        Assert.Equal(healthyRelease.Id, release.Id);
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

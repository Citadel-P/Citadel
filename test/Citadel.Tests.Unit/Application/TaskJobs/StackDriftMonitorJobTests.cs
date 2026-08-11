using Application.Services;
using Application.Services.Alerts;
using Application.Services.Licensing;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities;
using Domain.Entities.Stacks;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.TaskJobs;

public sealed class StackDriftMonitorJobTests
{
    [Fact]
    public async Task HandleContainerEventAsync_DieOnAutoFixStack_ReconcilesImmediately()
    {
        var fixture = CreateFixture(
            StackReleaseStatus.Degraded,
            ResourceControlState.Idle);
        await using var services = fixture.Services;

        await fixture.Job.HandleContainerEventAsync(
            new ContainerEvent(fixture.PlatformId, null, fixture.ContainerId, "die"),
            TestContext.Current.CancellationToken);

        fixture.DriftChecker.Verify(
            checker => checker.CheckAsync(
                It.Is<StackDriftStack>(stack => stack.Id == fixture.StackId),
                It.IsAny<CancellationToken>()),
            Times.Once);
        fixture.Reconciler.Verify(
            reconciler => reconciler.ReconcileAsync(
                fixture.StackId,
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Theory]
    [InlineData(StackReleaseStatus.Healthy, ResourceControlState.Processing)]
    [InlineData(StackReleaseStatus.Stopped, ResourceControlState.Idle)]
    public async Task HandleContainerEventAsync_DieDuringIntentionalStop_DoesNotRestartStack(
        StackReleaseStatus stackStatus,
        ResourceControlState controlState)
    {
        var fixture = CreateFixture(stackStatus, controlState);
        await using var services = fixture.Services;

        await fixture.Job.HandleContainerEventAsync(
            new ContainerEvent(fixture.PlatformId, null, fixture.ContainerId, "die"),
            TestContext.Current.CancellationToken);

        fixture.DriftChecker.Verify(
            checker => checker.CheckAsync(
                It.IsAny<StackDriftStack>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
        fixture.Reconciler.Verify(
            reconciler => reconciler.ReconcileAsync(
                It.IsAny<Guid>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    private static Fixture CreateFixture(
        StackReleaseStatus stackStatus,
        ResourceControlState controlState)
    {
        var platformId = Guid.CreateVersion7();
        var stackId = Guid.CreateVersion7();
        var releaseId = Guid.CreateVersion7();
        const string containerId = "container-1";

        var policy = new StackDriftPolicy(
            Mode: StackDriftMode.AutoFix,
            AlertOnDrift: false,
            MarkDegraded: true,
            AutoStartStoppedContainers: true,
            AutoResumePausedContainers: true,
            RemoveExtraContainers: false);
        var stack = new StackDriftStack(
            stackId,
            releaseId,
            "beszel",
            StackSource.WebEditor,
            platformId,
            "local",
            stackStatus,
            controlState,
            new ManualStack(
                ComposeFile: "services:\n  beszel-agent:\n    image: henrygd/beszel-agent:latest\n",
                UpdateBehavior: StackUpdateBehavior.Disabled),
            Source: null,
            policy);
        var container = new Container(
            name: "beszel-agent",
            dockerImageId: "image-1",
            platformId,
            containerId,
            ContainerStateStatus.Exited,
            stackId: stackId);
        var report = new StackDriftReport(
            stackId,
            platformId,
            HasDrift: true,
            HasAutoFixableDrift: true,
            HasStructuralDrift: false,
            Drifts: [new ContainerStopped(containerId, "beszel-agent")]);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(repository => repository.GetContainerInfoAsync(
                containerId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(container);

        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(repository => repository.GetDriftStackAsync(
                stackId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(uow => uow.Containers).Returns(containers.Object);
        unitOfWork.SetupGet(uow => uow.Stacks).Returns(stacks.Object);

        var services = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();

        var driftChecker = new Mock<IStackDriftChecker>();
        driftChecker
            .Setup(checker => checker.CheckAsync(
                stack,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(report);

        var reconciliationResult = new StackReconciliationResult(
            stackId,
            StackReconciliationStatus.Reconciled,
            report,
            AfterReport: null,
            Actions:
            [
                new StackReconciliationAction(
                    containerId,
                    "beszel-agent",
                    StackReconciliationActionType.StartContainer,
                    Succeeded: true)
            ]);
        var reconciler = new Mock<IStackReconciler>();
        reconciler
            .Setup(service => service.ReconcileAsync(
                stackId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(reconciliationResult);

        var dbQueue = new Mock<IDbWorkQueue>();
        dbQueue
            .Setup(queue => queue.EnqueueAndWaitAsync(
                It.IsAny<IDbWorkItem>(),
                It.IsAny<CancellationToken>()))
            .Returns(ValueTask.CompletedTask);

        var entitlementService = new Mock<ILicenseEntitlementService>();
        entitlementService
            .Setup(service => service.IsEnabledAsync(
                LicenseCapability.OperationalGuardrails,
                It.IsAny<CancellationToken>()))
            .Returns(new ValueTask<bool>(true));

        var job = new StackDriftMonitorJob(
            services.GetRequiredService<IServiceScopeFactory>(),
            dbQueue.Object,
            Mock.Of<IAlertService>(),
            driftChecker.Object,
            reconciler.Object,
            Mock.Of<INotificationQueue>(),
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            Mock.Of<IDelayWithJitterService>(),
            entitlementService.Object,
            new ContainerEventBroadcaster(),
            NullLogger<StackDriftMonitorJob>.Instance);

        return new Fixture(
            services,
            job,
            driftChecker,
            reconciler,
            platformId,
            stackId,
            containerId);
    }

    private sealed record Fixture(
        ServiceProvider Services,
        StackDriftMonitorJob Job,
        Mock<IStackDriftChecker> DriftChecker,
        Mock<IStackReconciler> Reconciler,
        Guid PlatformId,
        Guid StackId,
        string ContainerId);
}

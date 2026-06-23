using System.Collections.Immutable;
using System.Threading.Channels;
using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Stacks;
using Hosting.Common;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using YamlDotNet.RepresentationModel;
using ActivityEvent = global::Domain.Entities.Activities.ActivityEvent;
using Actor = global::Domain.Entities.Identity.Actor;

namespace Tests.Unit.Application.Features.Stacks;

public class StackDriftTests
{
    [Fact]
    public async Task CheckAsync_reports_runtime_and_structural_drift()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(platformId);
        var checker = CreateChecker(
            stack,
            new StackDesiredState(
                "demo",
                new Dictionary<string, StackDesiredService>(StringComparer.OrdinalIgnoreCase)
                {
                    ["api"] = new("api", "nginx:latest", "expected-hash"),
                    ["worker"] = new("worker", "busybox:latest", null),
                }),
            new StackRuntimeState(
                platformId,
                "http://docker.local",
                PlatformConnectorType.Local,
                "demo",
                [
                    Container("api-container", "api", ContainerStateStatus.Exited, configHash: "actual-hash"),
                    Container("orphan-container", "orphan", ContainerStateStatus.Running),
                ]));

        var report = await checker.CheckAsync(stack.Id, CancellationToken.None);

        Assert.True(report.HasDrift);
        Assert.True(report.HasAutoFixableDrift);
        Assert.True(report.HasStructuralDrift);
        Assert.Contains(report.Drifts, drift => drift is ContainerStopped { ContainerId: "api-container", ServiceName: "api" });
        Assert.Contains(report.Drifts, drift => drift is ConfigHashMismatch { ServiceName: "api" });
        Assert.Contains(report.Drifts, drift => drift is MissingContainer { ServiceName: "worker" });
        Assert.Contains(report.Drifts, drift => drift is ExtraContainer { ContainerId: "orphan-container", ServiceName: "orphan" });
    }

    [Fact]
    public async Task CheckAsync_reports_paused_container_as_auto_fixable_without_structural_drift()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(platformId);
        var checker = CreateChecker(
            stack,
            new StackDesiredState(
                "demo",
                new Dictionary<string, StackDesiredService>(StringComparer.OrdinalIgnoreCase)
                {
                    ["api"] = new("api", "nginx:latest", null),
                }),
            new StackRuntimeState(
                platformId,
                "http://docker.local",
                PlatformConnectorType.Local,
                "demo",
                [
                    Container("api-container", "api", ContainerStateStatus.Paused),
                ]));

        var report = await checker.CheckAsync(stack.Id, CancellationToken.None);

        Assert.True(report.HasDrift);
        Assert.True(report.HasAutoFixableDrift);
        Assert.False(report.HasStructuralDrift);
        Assert.Single(report.Drifts);
        Assert.IsType<ContainerPaused>(report.Drifts[0]);
    }

    [Fact]
    public async Task ReconcileAsync_starts_stopped_and_resumes_paused_containers_when_policy_allows()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(
            platformId,
            new StackDriftPolicy(
                StackDriftMode.AutoFix,
                AlertOnDrift: true,
                MarkDegraded: true,
                AutoStartStoppedContainers: true,
                AutoResumePausedContainers: true,
                RemoveExtraContainers: false));
        var before = Report(
            stack.Id,
            platformId,
            new ContainerStopped("api-container", "api"),
            new ContainerPaused("worker-container", "worker"));
        var after = Report(stack.Id, platformId);
        var driftChecker = new TestStackDriftChecker(before, after);
        var connector = new Mock<IContainerConnector>();
        connector
            .Setup(x => x.PatchAsync(
                It.Is<PatchContainerCommand>(command =>
                    command.Action == ContainerAction.START &&
                    command.PlatformAddress == "http://docker.local" &&
                    command.ContainerIds.SequenceEqual(new[] { "api-container" })),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        connector
            .Setup(x => x.PatchAsync(
                It.Is<PatchContainerCommand>(command =>
                    command.Action == ContainerAction.UNPAUSE &&
                    command.PlatformAddress == "http://docker.local" &&
                    command.ContainerIds.SequenceEqual(new[] { "worker-container" })),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        var reconciler = CreateReconciler(stack, driftChecker, connector.Object, platformId);

        var result = await reconciler.ReconcileAsync(stack.Id, CancellationToken.None);

        Assert.Equal(StackReconciliationStatus.Reconciled, result.Status);
        Assert.Same(before, result.BeforeReport);
        Assert.Same(after, result.AfterReport);
        Assert.Equal(2, result.Actions.Count);
        Assert.Contains(result.Actions, action => action.Action == StackReconciliationActionType.StartContainer && action.Succeeded);
        Assert.Contains(result.Actions, action => action.Action == StackReconciliationActionType.ResumeContainer && action.Succeeded);
        Assert.Equal(1, driftChecker.CheckByIdCount);
        Assert.Equal(1, driftChecker.CheckByStackCount);
        connector.Verify(x => x.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()), Times.Exactly(2));
    }

    [Fact]
    public async Task ReconcileAsync_does_not_start_stopped_container_when_policy_disables_start()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(
            platformId,
            new StackDriftPolicy(
                StackDriftMode.AutoFix,
                AlertOnDrift: true,
                MarkDegraded: true,
                AutoStartStoppedContainers: false,
                AutoResumePausedContainers: false,
                RemoveExtraContainers: false));
        var before = Report(stack.Id, platformId, new ContainerStopped("api-container", "api"));
        var driftChecker = new TestStackDriftChecker(before);
        var connector = new Mock<IContainerConnector>();
        var reconciler = CreateReconciler(stack, driftChecker, connector.Object, platformId);

        var result = await reconciler.ReconcileAsync(stack.Id, CancellationToken.None);

        Assert.Equal(StackReconciliationStatus.Partial, result.Status);
        Assert.Empty(result.Actions);
        Assert.Null(result.AfterReport);
        connector.Verify(x => x.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task ReconcileAsync_mixed_structural_and_runtime_drift_executes_safe_action_but_requires_reapply()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(
            platformId,
            new StackDriftPolicy(
                StackDriftMode.AutoFix,
                AlertOnDrift: true,
                MarkDegraded: true,
                AutoStartStoppedContainers: true,
                AutoResumePausedContainers: false,
                RemoveExtraContainers: false));
        var before = Report(
            stack.Id,
            platformId,
            new ContainerStopped("api-container", "api"),
            new MissingContainer("worker"));
        var after = Report(stack.Id, platformId, new MissingContainer("worker"));
        var driftChecker = new TestStackDriftChecker(before, after);
        var connector = new Mock<IContainerConnector>();
        connector
            .Setup(x => x.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        var reconciler = CreateReconciler(stack, driftChecker, connector.Object, platformId);

        var result = await reconciler.ReconcileAsync(stack.Id, CancellationToken.None);

        Assert.Equal(StackReconciliationStatus.RequiresReapply, result.Status);
        Assert.Single(result.Actions);
        Assert.Same(after, result.AfterReport);
        connector.Verify(x => x.PatchAsync(
            It.Is<PatchContainerCommand>(command => command.Action == ContainerAction.START),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task ReconcileAsync_patch_failure_returns_partial_action_with_error()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(
            platformId,
            new StackDriftPolicy(
                StackDriftMode.AutoFix,
                AlertOnDrift: true,
                MarkDegraded: true,
                AutoStartStoppedContainers: true,
                AutoResumePausedContainers: false,
                RemoveExtraContainers: false));
        var before = Report(stack.Id, platformId, new ContainerStopped("api-container", "api"));
        var after = Report(stack.Id, platformId, new ContainerStopped("api-container", "api"));
        var driftChecker = new TestStackDriftChecker(before, after);
        var connector = new Mock<IContainerConnector>();
        connector
            .Setup(x => x.PatchAsync(It.IsAny<PatchContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure(new Error("start failed")));
        var reconciler = CreateReconciler(stack, driftChecker, connector.Object, platformId);

        var result = await reconciler.ReconcileAsync(stack.Id, CancellationToken.None);

        Assert.Equal(StackReconciliationStatus.Partial, result.Status);
        var action = Assert.Single(result.Actions);
        Assert.False(action.Succeeded);
        Assert.Equal("start failed", action.ErrorMessage);
        Assert.Same(after, result.AfterReport);
    }

    [Fact]
    public async Task StatusWorkItem_no_drift_healthy_stack_does_not_commit_or_notify()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(platformId);
        stack.PartialUpdate(StackReleaseStatus.Healthy);
        var report = Report(stack.Id, platformId);
        var context = CreateStatusWorkItemContext(stack);
        var workItem = CreateStatusWorkItem(stack.Id, report, "fp", context.NotificationQueue);

        await workItem.ExecuteAsync(context.UnitOfWork.Object, CancellationToken.None);

        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
        context.Stacks.Verify(x => x.UpdateAsync(It.IsAny<Stack>(), It.IsAny<CancellationToken>()), Times.Never);
        context.ActivityEvents.Verify(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()), Times.Never);
        Assert.Empty(context.NotificationQueue.Items);
    }

    [Fact]
    public async Task StatusWorkItem_drift_detected_with_mark_degraded_disabled_writes_activity_without_status_update()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(
            platformId,
            new StackDriftPolicy(
                StackDriftMode.DetectOnly,
                AlertOnDrift: true,
                MarkDegraded: false,
                AutoStartStoppedContainers: false,
                AutoResumePausedContainers: false,
                RemoveExtraContainers: false));
        var report = Report(stack.Id, platformId, new ContainerStopped("api-container", "api"));
        var context = CreateStatusWorkItemContext(stack);
        ActivityEvent? activity = null;
        context.ActivityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((x, _) => activity = x)
            .ReturnsAsync(1);
        var workItem = CreateStatusWorkItem(stack.Id, report, "fp", context.NotificationQueue);

        await workItem.ExecuteAsync(context.UnitOfWork.Object, CancellationToken.None);

        Assert.Equal(StackReleaseStatus.Created, stack.CurrentStackRelease?.Status);
        Assert.IsType<StackDriftDetected>(activity?.Info);
        context.Stacks.Verify(x => x.UpdateAsync(It.IsAny<Stack>(), It.IsAny<CancellationToken>()), Times.Never);
        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        Assert.Single(context.NotificationQueue.Items);
    }

    [Fact]
    public async Task StatusWorkItem_no_drift_degraded_by_drift_marks_healthy_and_writes_resolved_activity()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(platformId);
        stack.PartialUpdate(StackReleaseStatus.Degraded);
        stack.AssignActivityEvent(CreateActivity(stack, ActivityEventType.StackDriftDetected, new StackDriftDetected("reason", "fp-old")));
        var report = Report(stack.Id, platformId);
        var context = CreateStatusWorkItemContext(stack);
        ActivityEvent? activity = null;
        context.ActivityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((x, _) => activity = x)
            .ReturnsAsync(1);
        var workItem = CreateStatusWorkItem(stack.Id, report, "fp-empty", context.NotificationQueue);

        await workItem.ExecuteAsync(context.UnitOfWork.Object, CancellationToken.None);

        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease?.Status);
        Assert.IsType<StackDriftResolved>(activity?.Info);
        context.Stacks.Verify(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()), Times.Once);
        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        Assert.Equal(2, context.NotificationQueue.Items.Count);
    }

    [Fact]
    public async Task StatusWorkItem_no_drift_degraded_by_non_drift_reason_does_not_heal()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(platformId);
        stack.PartialUpdate(StackReleaseStatus.Degraded);
        stack.AssignActivityEvent(CreateActivity(stack, ActivityEventType.StackDegraded, new StackDegraded("apply failed")));
        var report = Report(stack.Id, platformId);
        var context = CreateStatusWorkItemContext(stack);
        var workItem = CreateStatusWorkItem(stack.Id, report, "fp-empty", context.NotificationQueue);

        await workItem.ExecuteAsync(context.UnitOfWork.Object, CancellationToken.None);

        Assert.Equal(StackReleaseStatus.Degraded, stack.CurrentStackRelease?.Status);
        context.Stacks.Verify(x => x.UpdateAsync(It.IsAny<Stack>(), It.IsAny<CancellationToken>()), Times.Never);
        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
        Assert.Empty(context.NotificationQueue.Items);
    }

    [Fact]
    public async Task StatusWorkItem_no_drift_after_reconciliation_attempt_without_drift_does_not_heal()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(platformId);
        stack.PartialUpdate(StackReleaseStatus.Degraded);
        stack.AssignActivityEvent(CreateActivity(
            stack,
            ActivityEventType.StackReconciliationAttempted,
            new StackReconciliationAttempted(StackReconciliationStatus.Reconciled, [], "fp-old")));
        var report = Report(stack.Id, platformId);
        var context = CreateStatusWorkItemContext(stack);
        var workItem = CreateStatusWorkItem(stack.Id, report, "fp-empty", context.NotificationQueue);

        await workItem.ExecuteAsync(context.UnitOfWork.Object, CancellationToken.None);

        Assert.Equal(StackReleaseStatus.Degraded, stack.CurrentStackRelease?.Status);
        context.Stacks.Verify(x => x.UpdateAsync(It.IsAny<Stack>(), It.IsAny<CancellationToken>()), Times.Never);
        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
        Assert.Empty(context.NotificationQueue.Items);
    }

    [Fact]
    public async Task StatusWorkItem_no_drift_after_reconciliation_uses_previous_drift_fingerprint()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(platformId);
        stack.PartialUpdate(StackReleaseStatus.Degraded);
        stack.AssignActivityEvent(CreateActivity(
            stack,
            ActivityEventType.StackReconciliationAttempted,
            new StackReconciliationAttempted(StackReconciliationStatus.Reconciled, [], "fp-old")));
        var report = Report(stack.Id, platformId);
        var context = CreateStatusWorkItemContext(stack);
        ActivityEvent? activity = null;
        context.ActivityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((x, _) => activity = x)
            .ReturnsAsync(1);
        var workItem = CreateStatusWorkItem(stack.Id, report, "fp-empty", context.NotificationQueue, "fp-old");

        await workItem.ExecuteAsync(context.UnitOfWork.Object, CancellationToken.None);

        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease?.Status);
        var resolved = Assert.IsType<StackDriftResolved>(activity?.Info);
        Assert.Equal("fp-old", resolved.PreviousFingerprint);
        context.Stacks.Verify(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()), Times.Once);
        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        Assert.Equal(2, context.NotificationQueue.Items.Count);
    }

    [Fact]
    public async Task StatusWorkItem_repeated_same_drift_fingerprint_does_not_duplicate_activity()
    {
        var platformId = Guid.CreateVersion7();
        var stack = CreateStack(platformId);
        stack.PartialUpdate(StackReleaseStatus.Degraded);
        stack.AssignActivityEvent(CreateActivity(stack, ActivityEventType.StackDriftDetected, new StackDriftDetected("reason", "fp")));
        var report = Report(stack.Id, platformId, new ContainerStopped("api-container", "api"));
        var context = CreateStatusWorkItemContext(stack);
        var workItem = CreateStatusWorkItem(stack.Id, report, "fp", context.NotificationQueue);

        await workItem.ExecuteAsync(context.UnitOfWork.Object, CancellationToken.None);

        context.ActivityEvents.Verify(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()), Times.Never);
        context.UnitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
        Assert.Empty(context.NotificationQueue.Items);
    }

    [Fact]
    public void StackComposeLabelInjector_injects_ownership_labels_and_service_hashes()
    {
        var stackId = Guid.CreateVersion7();
        var releaseId = Guid.CreateVersion7();
        const string compose = """
            services:
              api:
                image: nginx:latest
                labels:
                  - com.example.role=edge
              worker:
                image: busybox:latest
                labels:
                  com.example.role: background
            """;

        var result = StackComposeLabelInjector.Inject(compose, stackId, releaseId);
        var services = ReadServices(result);
        var apiLabels = ReadServiceLabels(services, "api");
        var workerLabels = ReadServiceLabels(services, "worker");

        Assert.Equal("edge", apiLabels["com.example.role"]);
        Assert.Equal("background", workerLabels["com.example.role"]);
        Assert.Equal("true", apiLabels[CitadelLabels.Managed]);
        Assert.Equal(stackId.ToString("D"), apiLabels[CitadelLabels.StackId]);
        Assert.Equal(releaseId.ToString("D"), apiLabels[CitadelLabels.ReleaseId]);
        Assert.False(string.IsNullOrWhiteSpace(apiLabels[CitadelLabels.ServiceHash]));
        Assert.NotEqual(apiLabels[CitadelLabels.ServiceHash], workerLabels[CitadelLabels.ServiceHash]);
    }

    [Fact]
    public void StackComposeLabelInjector_rejects_reserved_user_labels()
    {
        const string compose = """
            services:
              api:
                image: nginx:latest
                labels:
                  x-citadel.stack-id: user-value
            """;

        var error = Assert.Throws<InvalidOperationException>(() =>
            StackComposeLabelInjector.Inject(compose, Guid.CreateVersion7(), Guid.CreateVersion7()));

        Assert.Contains("reserved by Citadel", error.Message);
    }

    [Fact]
    public void StackProjectNameResolver_rejects_invalid_explicit_project_name()
    {
        var stack = Stack.Create(
            name: "Demo",
            createdByActorId: Guid.CreateVersion7(),
            StackSource: StackSource.WebEditor,
            platformId: Guid.CreateVersion7(),
            spec: new ManualStack(
                ComposeFile: "services:\n  api:\n    image: nginx\n",
                UpdateBehavior: StackUpdateBehavior.Disabled,
                ProjectName: "Invalid.Name"),
            driftPolicy: StackDriftPolicy.Default);

        var error = Assert.Throws<InvalidOperationException>(() => StackProjectNameResolver.Resolve(stack));

        Assert.Contains("Docker Compose project name", error.Message);
    }

    [Fact]
    public void StackProjectNameResolver_uses_stack_id_when_normalized_name_is_empty()
    {
        var stack = Stack.Create(
            name: "!!!",
            createdByActorId: Guid.CreateVersion7(),
            StackSource: StackSource.WebEditor,
            platformId: Guid.CreateVersion7(),
            spec: new ManualStack(
                ComposeFile: "services:\n  api:\n    image: nginx\n",
                UpdateBehavior: StackUpdateBehavior.Disabled),
            driftPolicy: StackDriftPolicy.Default);

        var projectName = StackProjectNameResolver.Resolve(stack);

        Assert.Equal($"stack-{stack.Id:N}", projectName);
    }

    private static ManualStackDriftChecker CreateChecker(
        Stack stack,
        StackDesiredState desired,
        StackRuntimeState runtime)
    {
        return new ManualStackDriftChecker(
            CreateScopeFactory(stack),
            new TestDesiredStateProvider(desired),
            new TestRuntimeStateProvider(runtime));
    }

    private static StackReconciler CreateReconciler(
        Stack stack,
        IStackDriftChecker driftChecker,
        IContainerConnector connector,
        Guid platformId)
    {
        var platform = new PlatformCacheEntry(
            platformId,
            "http://docker.local",
            PlatformConnectorType.Local,
            ImmutableDictionary<string, Guid>.Empty);
        var connectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        connectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(connector);

        return new StackReconciler(
            CreateScopeFactory(stack),
            driftChecker,
            new TestPlatformContainerCache(platform),
            connectorFactory.Object);
    }

    private static StackDriftReport Report(Guid stackId, Guid platformId, params StackDrift[] drifts)
        => new(
            stackId,
            platformId,
            HasDrift: drifts.Length > 0,
            HasAutoFixableDrift: drifts.Any(StackDriftHelpers.IsAutoFixable),
            HasStructuralDrift: drifts.Any(StackDriftHelpers.IsStructural),
            Drifts: drifts);

    private static StackDriftStatusWorkItem CreateStatusWorkItem(
        Guid stackId,
        StackDriftReport report,
        string fingerprint,
        TestNotificationQueue notificationQueue,
        string? previousDriftFingerprint = null)
        => new(
            stackId,
            report,
            fingerprint,
            notificationQueue,
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            previousDriftFingerprint);

    private static YamlMappingNode ReadServices(string compose)
    {
        using var reader = new StringReader(compose);
        var yaml = new YamlStream();
        yaml.Load(reader);
        var root = Assert.IsType<YamlMappingNode>(yaml.Documents[0].RootNode);
        return Assert.IsType<YamlMappingNode>(root.Children[new YamlScalarNode("services")]);
    }

    private static Dictionary<string, string> ReadServiceLabels(YamlMappingNode services, string serviceName)
    {
        var service = Assert.IsType<YamlMappingNode>(services.Children[new YamlScalarNode(serviceName)]);
        var labels = Assert.IsType<YamlMappingNode>(service.Children[new YamlScalarNode("labels")]);
        return labels.Children.ToDictionary(
            x => Assert.IsType<YamlScalarNode>(x.Key).Value!,
            x => Assert.IsType<YamlScalarNode>(x.Value).Value ?? string.Empty);
    }

    private static StatusWorkItemContext CreateStatusWorkItemContext(Stack stack)
    {
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.UpdateAsync(It.IsAny<Stack>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        return new StatusWorkItemContext(
            unitOfWork,
            stacks,
            activityEvents,
            new TestNotificationQueue());
    }

    private static ActivityEvent CreateActivity(Stack stack, ActivityEventType type, ActivityEventInfo info)
    {
        var release = stack.CurrentStackRelease ?? throw new InvalidOperationException("Stack must have a current release.");
        return new ActivityEvent(
            actorId: Constants.SystemId,
            resourceId: stack.Id,
            platformId: release.PlatformId,
            resourceName: stack.Name,
            eventType: type,
            status: ActivityStatus.Warning,
            info: info);
    }

    private static IServiceScopeFactory CreateScopeFactory(Stack stack)
    {
        var stackRepository = new Mock<IStackRepository>();
        stackRepository
            .Setup(x => x.GetDriftStackAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(ToDriftStack(stack));
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stackRepository.Object);

        return new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider()
            .GetRequiredService<IServiceScopeFactory>();
    }

    private static StackDriftStack ToDriftStack(Stack stack)
    {
        var release = stack.CurrentStackRelease ?? throw new InvalidOperationException("Stack must have a current release.");

        return new StackDriftStack(
            stack.Id,
            stack.CurrentStackReleaseId,
            stack.Name,
            stack.StackSource,
            release.PlatformId,
            release.Platform?.Name,
            release.Status,
            stack.ControlState,
            release.Spec,
            stack.DriftPolicy);
    }

    private static Stack CreateStack(Guid platformId, StackDriftPolicy? driftPolicy = null)
        => Stack.Create(
            name: "demo",
            createdByActorId: Guid.CreateVersion7(),
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  api:\n    image: nginx:latest\n",
                UpdateBehavior: StackUpdateBehavior.Disabled),
            driftPolicy: driftPolicy);

    private static StackRuntimeContainer Container(
        string id,
        string serviceName,
        ContainerStateStatus state,
        string? healthStatus = null,
        string? configHash = null)
        => new(
            ContainerId: id,
            Name: $"{serviceName}-1",
            ServiceName: serviceName,
            Image: $"{serviceName}:latest",
            State: state,
            HealthStatus: healthStatus,
            ConfigHash: configHash,
            Labels: ImmutableDictionary<string, string>.Empty);

    private sealed class TestDesiredStateProvider(StackDesiredState state) : IStackDesiredStateProvider
    {
        public Task<StackDesiredState> GetDesiredStateAsync(StackDriftStack stack, CancellationToken cancellationToken)
            => Task.FromResult(state);
    }

    private sealed class TestRuntimeStateProvider(StackRuntimeState state) : IStackRuntimeStateProvider
    {
        public Task<StackRuntimeState> GetRuntimeStateAsync(StackDriftStack stack, CancellationToken cancellationToken)
            => Task.FromResult(state);
    }

    private sealed class TestStackDriftChecker(StackDriftReport beforeReport, StackDriftReport? afterReport = null) : IStackDriftChecker
    {
        public int CheckByIdCount { get; private set; }
        public int CheckByStackCount { get; private set; }

        public Task<StackDriftReport> CheckAsync(Guid stackId, CancellationToken cancellationToken)
        {
            CheckByIdCount++;
            return Task.FromResult(afterReport ?? beforeReport);
        }

        public Task<StackDriftReport> CheckAsync(StackDriftStack stack, CancellationToken cancellationToken)
        {
            CheckByStackCount++;
            return Task.FromResult(beforeReport);
        }
    }

    private sealed record StatusWorkItemContext(
        Mock<IUnitOfWork> UnitOfWork,
        Mock<IStackRepository> Stacks,
        Mock<IActivityEventRepository> ActivityEvents,
        TestNotificationQueue NotificationQueue);

    private sealed class TestNotificationQueue : INotificationQueue
    {
        public List<INotificationWorkItem> Items { get; } = [];

        public ChannelReader<INotificationWorkItem> Reader { get; } = Channel.CreateUnbounded<INotificationWorkItem>().Reader;

        public ValueTask EnqueueAsync(INotificationWorkItem item, CancellationToken ct)
        {
            Items.Add(item);
            return ValueTask.CompletedTask;
        }
    }

    private sealed class TestPlatformContainerCache(PlatformCacheEntry platform) : IPlatformContainerCache
    {
        public void ReplacePlatformContainers(Guid platformId, PlatformCacheEntry cacheEntry)
        {
        }

        public bool TryAddContainer(Guid platformId, string containerId, Guid dbId) => false;

        public bool TryRemoveContainer(Guid platformId, string containerId) => false;

        public bool EvictPlatform(Guid platformId) => false;

        public bool TryGetContainers(Guid platformId, out IReadOnlyDictionary<string, Guid> containers)
        {
            containers = platform.Id == platformId
                ? platform.Containers
                : ImmutableDictionary<string, Guid>.Empty;
            return platform.Id == platformId;
        }

        public bool TryGetCacheEntry(Guid platformId, out PlatformCacheEntry cacheEntry, out Error error)
        {
            if (platform.Id == platformId)
            {
                cacheEntry = platform;
                error = null!;
                return true;
            }

            cacheEntry = null!;
            error = new Error("not found");
            return false;
        }

        public bool TryGetCacheEntries(out IEnumerable<PlatformCacheEntry> cacheEntries, out Error error)
        {
            cacheEntries = [platform];
            error = null!;
            return true;
        }

        public bool TryGetPlatformWithContainer(string containerId, out PlatformCacheEntry cacheEntry)
        {
            cacheEntry = null!;
            return false;
        }

        public bool TryGetPlatformsWithContainers(string[] containersId, out List<PlatformCacheEntry> cacheEntries)
        {
            cacheEntries = [];
            return false;
        }
    }
}

using Application.Services;
using Domain;
using Domain.Entities.Stacks;
using Hosting.Common;

namespace Tests.Unit.Application.Services;

public sealed class UpdateCheckServicesTests
{
    [Fact]
    public void BuildManualStackChecks_AllowsDisabledOnDemandButNotScheduled()
    {
        var stack = CreateManualStack(
            StackUpdateBehavior.Disabled,
            """
            services:
              api:
                image: nginx:1.27
            """);
        var builder = new ImageCheckBuilder();

        var scheduled = builder.BuildManualStackChecks(stack, ImageCheckMode.Scheduled);
        var onDemand = builder.BuildManualStackChecks(stack, ImageCheckMode.OnDemand);

        Assert.False(scheduled.IsSuccess());
        Assert.True(onDemand.IsSuccess(out var checks, out var error), error?.Message);
        Assert.Single(checks);
    }

    [Fact]
    public void BuildManualStackChecks_ExcludesBuildBoundServices()
    {
        var stack = CreateManualStack(
            StackUpdateBehavior.Notify,
            """
            services:
              api:
                image: example/api:latest
              worker:
                image: example/worker:latest
            """,
            [new StackBuildImageBinding("api", Guid.CreateVersion7())]);
        var builder = new ImageCheckBuilder();

        var result = builder.BuildManualStackChecks(stack, ImageCheckMode.OnDemand);

        Assert.True(result.IsSuccess(out var checks, out var error), error?.Message);
        var check = Assert.Single(checks);
        Assert.Equal("worker", check.ServiceName);
        Assert.Equal("example/worker:latest", check.ImageName);
    }

    [Fact]
    public void DeploymentUpdateEvaluator_ReportsOnlyDifferentDigests()
    {
        var evaluator = new DeploymentUpdateEvaluator();
        var checkedAt = new DateTime(2026, 7, 26, 12, 0, 0, DateTimeKind.Utc);

        var current = evaluator.Evaluate("sha256:current", "SHA256:CURRENT", checkedAt);
        var available = evaluator.Evaluate("sha256:current", "sha256:new", checkedAt);

        Assert.False(current.UpdateAvailable);
        Assert.Equal(AutoUpdateStatus.UpToDate, current.State.Status);
        Assert.True(available.UpdateAvailable);
        Assert.Equal(AutoUpdateStatus.UpdateAvailable, available.State.Status);
    }

    [Fact]
    public void ManualStackUpdateEvaluator_RecordsBaselineBeforeReportingAnUpdate()
    {
        var stack = CreateManualStack(StackUpdateBehavior.Notify, "services:\n  api:\n    image: nginx:latest\n");
        var check = new ManualStackImageCheck(
            stack,
            "api",
            "nginx:latest",
            new ImageKey(stack.CurrentStackRelease!.Spec.RegistryId!.Value, "nginx", "latest"));
        var evaluator = new ManualStackUpdateEvaluator();
        var firstDigest = new Dictionary<ImageKey, string> { [check.Key] = "sha256:first" };

        var baseline = evaluator.Evaluate(
            new ManualStackUpdateState(new RecreateStackOnNewImageState([])),
            [check],
            firstDigest,
            DateTime.UtcNow);
        var secondDigest = new Dictionary<ImageKey, string> { [check.Key] = "sha256:second" };
        var update = evaluator.Evaluate(
            baseline.State,
            [check],
            secondDigest,
            DateTime.UtcNow.AddMinutes(1));

        Assert.Equal(1, baseline.BaselinesCreated);
        Assert.Empty(baseline.AvailableUpdates);
        Assert.Equal("sha256:first", Assert.Single(
            baseline.State.RecreateStackOnNewImageState.AutoUpdateStates).CurrentDigest);
        Assert.Equal(0, update.BaselinesCreated);
        Assert.Single(update.AvailableUpdates);
        Assert.Single(update.NewlyDetectedUpdates);
    }

    [Fact]
    public void UpdateCheckLeaseManager_RejectsDuplicateUntilLeaseIsReleased()
    {
        var manager = new UpdateCheckLeaseManager();
        var resourceId = Guid.CreateVersion7();

        Assert.True(manager.TryAcquire(ResourceType.Stack, resourceId, out var first));
        Assert.False(manager.TryAcquire(ResourceType.Stack, resourceId, out var duplicate));
        Assert.Null(duplicate);

        first!.Dispose();

        Assert.True(manager.TryAcquire(ResourceType.Stack, resourceId, out var next));
        next!.Dispose();
    }

    private static Stack CreateManualStack(
        StackUpdateBehavior updateBehavior,
        string compose,
        IReadOnlyList<StackBuildImageBinding>? buildImageBindings = null)
    {
        var stack = Stack.Create(
            $"stack-{Guid.NewGuid():N}",
            Guid.CreateVersion7(),
            StackSource.WebEditor,
            Guid.CreateVersion7(),
            new ManualStack(
                ComposeFile: compose,
                UpdateBehavior: updateBehavior,
                RegistryId: Guid.CreateVersion7(),
                BuildImageBindings: buildImageBindings));
        stack.PartialUpdate(StackReleaseStatus.Healthy);
        return stack;
    }
}

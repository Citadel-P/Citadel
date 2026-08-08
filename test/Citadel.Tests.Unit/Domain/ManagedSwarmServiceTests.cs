using Application.Services;
using Domain;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
using Hosting.Common;

namespace Tests.Unit.Domain;

public sealed class ManagedSwarmServiceTests
{
    [Fact]
    public void NewService_ShouldRemainCreatedUntilItHasDockerRuntimeIdentity()
    {
        var service = CreateService();

        Assert.Equal(SwarmServiceHealth.Created, service.Health);

        service.ApplyObservation(null);

        Assert.Equal(SwarmServiceHealth.Created, service.Health);
        Assert.Equal(SwarmServiceSynchronizationState.NeverApplied, service.SynchronizationState);
    }

    [Fact]
    public void MissingAppliedService_ShouldRemainUnknownRatherThanCreated()
    {
        var service = CreateService();
        Assert.True(service.TryPrepareOperation(
            SwarmServiceOperationKind.Apply,
            Guid.CreateVersion7(),
            Constants.SystemId));
        service.MarkOperationAttempted();
        service.MarkOperationAccepted("docker-service", 1);

        service.ApplyObservation(null);

        Assert.Equal(SwarmServiceHealth.Unknown, service.Health);
        Assert.Equal(SwarmServiceSynchronizationState.RuntimeMissing, service.SynchronizationState);
    }

    [Fact]
    public void DesiredHash_ShouldTrackSourceTagButIgnoreResolvedDigest()
    {
        var registryId = Guid.CreateVersion7();
        var first = CreateSpec(new SwarmExternalImage(registryId, "nginx:1.27", "sha256:old"));
        var refreshed = first with
        {
            Image = new SwarmExternalImage(registryId, "nginx:1.27", "sha256:new")
        };
        var changedTag = first with
        {
            Image = new SwarmExternalImage(registryId, "nginx:stable", "sha256:old")
        };

        Assert.Equal(SwarmServiceSpecHasher.Hash(first), SwarmServiceSpecHasher.Hash(refreshed));
        Assert.NotEqual(SwarmServiceSpecHasher.Hash(first), SwarmServiceSpecHasher.Hash(changedTag));
    }

    [Fact]
    public void DesiredAndRuntimeHashes_ShouldIgnoreWebhookConfiguration()
    {
        var first = CreateSpec(new SwarmExternalImage(Guid.CreateVersion7(), "nginx:1.27"));
        var withWebhook = first with
        {
            Webhook = new SwarmServiceWebhookConfig(
                Enabled: true,
                Provider: WebhookProvider.Generic,
                AuthScheme: WebhookAuthScheme.BearerToken,
                Secret: "shared-secret"),
        };

        Assert.Equal(SwarmServiceSpecHasher.Hash(first), SwarmServiceSpecHasher.Hash(withWebhook));
        Assert.Equal(
            SwarmServiceRuntimeHasher.Hash(first, "nginx@sha256:applied"),
            SwarmServiceRuntimeHasher.Hash(withWebhook, "nginx@sha256:applied"));
    }

    [Fact]
    public void DesiredAndRuntimeHashes_ShouldTrackUserDefinedLabelsRegardlessOfOrder()
    {
        var first = CreateSpec(new SwarmExternalImage(Guid.CreateVersion7(), "nginx:1.27")) with
        {
            Labels = new Dictionary<string, string> { ["team"] = "platform", ["tier"] = "web" }
        };
        var reordered = first with
        {
            Labels = new Dictionary<string, string> { ["tier"] = "web", ["team"] = "platform" }
        };
        var changed = first with
        {
            Labels = new Dictionary<string, string> { ["team"] = "operations", ["tier"] = "web" }
        };

        Assert.Equal(SwarmServiceSpecHasher.Hash(first), SwarmServiceSpecHasher.Hash(reordered));
        Assert.NotEqual(SwarmServiceSpecHasher.Hash(first), SwarmServiceSpecHasher.Hash(changed));
        Assert.Equal(
            SwarmServiceRuntimeHasher.Hash(first, "nginx@sha256:applied"),
            SwarmServiceRuntimeHasher.Hash(reordered, "nginx@sha256:applied"));
        Assert.NotEqual(
            SwarmServiceRuntimeHasher.Hash(first, "nginx@sha256:applied"),
            SwarmServiceRuntimeHasher.Hash(changed, "nginx@sha256:applied"));
    }

    [Fact]
    public void RuntimeHash_ShouldUseResolvedImageAndCanonicalizeSetLikeFields()
    {
        var registryId = Guid.CreateVersion7();
        var first = CreateSpec(new SwarmExternalImage(registryId, "nginx:1.27"));
        var reordered = first with
        {
            Image = new SwarmExternalImage(registryId, "unrelated-source-tag"),
            Environment = ["B=2", "A=1"],
            NetworkIds = ["network-b", "network-a"],
            PlacementConstraints = ["node.labels.disk==ssd", "node.role==worker"]
        };

        Assert.Equal(
            SwarmServiceRuntimeHasher.Hash(first, "nginx@sha256:applied"),
            SwarmServiceRuntimeHasher.Hash(reordered, "nginx@sha256:applied"));
    }

    [Fact]
    public void RuntimeHash_ShouldNormalizeDockerInjectedServiceDefaults()
    {
        const string image = "redis@sha256:39353c6a2f310da333374e1290c91805d15a85def073b5090f58e4ac646d284c";
        var spec = new SwarmServiceSpec
        {
            Image = new SwarmExternalImage(Guid.CreateVersion7(), "redis:latest"),
            SchedulingMode = SwarmServiceSchedulingMode.Replicated,
            Replicas = 1,
            Ports = [new SwarmServicePort(80, null)]
        };
        var observed = new SwarmServiceRuntimeState(
            Image: image,
            SchedulingMode: "replicated",
            Replicas: 1,
            Command: [],
            Arguments: [],
            Environment: [],
            User: null,
            WorkingDirectory: null,
            HealthCheckTest: [],
            HealthCheckInterval: null,
            HealthCheckTimeout: null,
            HealthCheckRetries: null,
            HealthCheckStartPeriod: null,
            StopGracePeriod: 10_000_000_000,
            Ports: ["80//tcp/ingress"],
            NetworkIds: [],
            Mounts: [],
            Secrets: [],
            Configs: [],
            LimitNanoCpus: null,
            LimitMemoryBytes: null,
            ReservationNanoCpus: null,
            ReservationMemoryBytes: null,
            PlacementConstraints: [],
            RestartCondition: "any",
            RestartDelay: 5_000_000_000,
            RestartMaximumAttempts: 0,
            RestartWindow: null,
            UpdateParallelism: null,
            UpdateDelay: null,
            UpdateOrder: null,
            UpdateFailureAction: null);

        var desiredHash = SwarmServiceRuntimeHasher.Hash(spec, image);

        // Runtime hashes are persisted with in-flight operations, so normalizing Docker's
        // defaults must not change the hash already produced for the desired specification.
        Assert.Equal("4916acd557546536146bcd70449df1214876c2502d01db90eba74c75c4bb34f9", desiredHash);
        Assert.Equal(desiredHash, SwarmServiceRuntimeStateHasher.Hash(observed));
    }

    [Fact]
    public void SchedulingMode_ShouldBecomeImmutableAfterSuccessfulApply()
    {
        var service = CreateService();
        var operationId = Guid.CreateVersion7();
        Assert.True(service.TryPrepareOperation(
            SwarmServiceOperationKind.Apply,
            operationId,
            Constants.SystemId,
            targetRuntimeHash: "runtime-hash"));
        service.MarkOperationAttempted();
        service.MarkOperationAccepted("docker-service", 1);
        service.CompleteOperation(
            SwarmServiceOperationState.Completed,
            runtimeHash: "runtime-hash",
            appliedImageDigest: "sha256:applied");

        Assert.False(service.UpdateSpec(service.Spec with
        {
            SchedulingMode = SwarmServiceSchedulingMode.Global,
            Replicas = null
        }));
        Assert.Equal(SwarmServiceSchedulingMode.Replicated, service.Spec.SchedulingMode);
    }

    [Theory]
    [InlineData(SwarmServiceOperationKind.Scale, true)]
    [InlineData(SwarmServiceOperationKind.ForceUpdate, true)]
    [InlineData(SwarmServiceOperationKind.Apply, false)]
    [InlineData(SwarmServiceOperationKind.Delete, false)]
    public void VersionConflictRetry_ShouldOnlyBeAllowedForAttemptedScaleAndForceUpdate(
        SwarmServiceOperationKind kind,
        bool expected)
    {
        var service = CreateService();
        Assert.True(service.TryPrepareOperation(kind, Guid.CreateVersion7(), Constants.SystemId));
        service.MarkOperationAttempted();

        Assert.Equal(expected, service.TryPrepareVersionConflictRetry(12, expectedForceUpdate: 7));
        if (expected)
        {
            Assert.Equal(12, service.CurrentOperation?.BaseDockerVersion);
            Assert.Equal(7, service.CurrentOperation?.ExpectedForceUpdate);
        }
    }

    [Fact]
    public void DesiredChangesAndRuntimeDrift_ShouldRemainIndependent()
    {
        var service = CreateService();
        Assert.True(service.TryPrepareOperation(
            SwarmServiceOperationKind.Apply,
            Guid.CreateVersion7(),
            Constants.SystemId,
            targetRuntimeHash: "applied-runtime"));
        service.MarkOperationAttempted();
        service.MarkOperationAccepted("docker-service", 1);
        service.CompleteOperation(SwarmServiceOperationState.Completed, "applied-runtime", "sha256:applied");
        Assert.True(service.UpdateSpec(service.Spec with { Replicas = 3 }));
        service.ApplyObservation(CreateProjection(service, "externally-changed-runtime"));

        Assert.True(service.HasPendingChanges);
        Assert.True(service.HasRuntimeDrift);
        Assert.Equal(SwarmServiceSynchronizationState.Drifted, service.SynchronizationState);
    }

    [Fact]
    public void DisabledUpdatePolicy_ShouldSkipScheduledChecksButAllowManualCheck()
    {
        var service = CreateService();
        Assert.True(service.TryPrepareOperation(
            SwarmServiceOperationKind.Apply,
            Guid.CreateVersion7(),
            Constants.SystemId,
            targetRuntimeHash: "runtime-hash"));
        service.MarkOperationAttempted();
        service.MarkOperationAccepted("docker-service", 1);
        service.CompleteOperation(SwarmServiceOperationState.Completed, "runtime-hash", "sha256:applied");
        Assert.True(service.UpdateSpec(service.Spec with { UpdateBehavior = UpdateBehavior.Disabled }));
        var builder = new ImageCheckBuilder();

        Assert.True(builder.BuildSwarmServiceCheck(service, ImageCheckMode.Scheduled).IsFailure());
        Assert.True(builder.BuildSwarmServiceCheck(service, ImageCheckMode.OnDemand).IsSuccess());
    }

    [Fact]
    public void OutcomeUnknown_ShouldBlockAnotherMutationUntilReconciliationProvesTheOutcome()
    {
        var service = CreateService();
        Assert.True(service.TryPrepareOperation(
            SwarmServiceOperationKind.Apply,
            Guid.CreateVersion7(),
            Constants.SystemId,
            clusterId: "cluster-a"));
        service.MarkOperationAttempted();
        service.CompleteOperation(SwarmServiceOperationState.OutcomeUnknown);

        Assert.False(service.TryPrepareOperation(
            SwarmServiceOperationKind.Apply,
            Guid.CreateVersion7(),
            Constants.SystemId,
            clusterId: "cluster-a"));
    }

    private static SwarmService CreateService() => new(
        "managed-service",
        Guid.CreateVersion7(),
        Constants.SystemId,
        CreateSpec(new SwarmExternalImage(Guid.CreateVersion7(), "nginx:1.27")));

    private static SwarmServiceSpec CreateSpec(SwarmServiceImageInfo image) => new()
    {
        Image = image,
        UpdateBehavior = UpdateBehavior.Notify,
        SchedulingMode = SwarmServiceSchedulingMode.Replicated,
        Replicas = 2,
        Environment = ["A=1", "B=2"],
        NetworkIds = ["network-a", "network-b"],
        PlacementConstraints = ["node.role==worker", "node.labels.disk==ssd"]
    };

    private static SwarmServiceProjection CreateProjection(SwarmService service, string runtimeHash) => new(
        service.PlatformId,
        "docker-service",
        2,
        service.DockerName,
        "Replicated",
        "nginx@sha256:applied",
        2,
        2,
        "Completed",
        null,
        [],
        service.Spec.NetworkIds,
        [],
        [],
        new Dictionary<string, string>
        {
            ["com.citadel.managed"] = "true",
            ["com.citadel.service-id"] = service.Id.ToString()
        },
        null,
        null,
        DateTimeOffset.UtcNow,
        false,
        SwarmServiceOwnership.CitadelService,
        SwarmServiceId: service.Id,
        LiveRuntimeHash: runtimeHash);
}

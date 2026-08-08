using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;
using Domain.Contracts.Resources;

namespace Domain.Entities.SwarmServices;

public sealed record SwarmServiceSpec
{
    public required SwarmServiceImageInfo Image { get; init; }
    public UpdateBehavior UpdateBehavior { get; init; } = UpdateBehavior.Disabled;
    public SwarmServiceSchedulingMode SchedulingMode { get; init; } = SwarmServiceSchedulingMode.Replicated;
    public int? Replicas { get; init; } = 1;
    public IReadOnlyList<string> Command { get; init; } = [];
    public IReadOnlyList<string> Arguments { get; init; } = [];
    public IReadOnlyList<string> Environment { get; init; } = [];
    public IReadOnlyDictionary<string, string> Labels { get; init; } = new Dictionary<string, string>(StringComparer.Ordinal);
    public string? User { get; init; }
    public string? WorkingDirectory { get; init; }
    public SwarmServiceHealthCheck? HealthCheck { get; init; }
    public long? StopGracePeriodNanoseconds { get; init; }
    public IReadOnlyList<SwarmServicePort> Ports { get; init; } = [];
    public IReadOnlyList<string> NetworkIds { get; init; } = [];
    public IReadOnlyList<SwarmServiceMount> Mounts { get; init; } = [];
    public IReadOnlyList<SwarmServiceSecretReference> Secrets { get; init; } = [];
    public IReadOnlyList<SwarmServiceConfigReference> Configs { get; init; } = [];
    public SwarmServiceResources? Resources { get; init; }
    public IReadOnlyList<string> PlacementConstraints { get; init; } = [];
    public SwarmServiceRestartPolicy? RestartPolicy { get; init; }
    public SwarmServiceUpdatePolicy? UpdatePolicy { get; init; }
    public SwarmServiceWebhookConfig? Webhook { get; init; }
}

public sealed record SwarmServiceWebhookConfig(
    bool Enabled = false,
    WebhookProvider Provider = WebhookProvider.GitHub,
    WebhookAuthScheme AuthScheme = WebhookAuthScheme.GitHubHmacSha256,
    string? Secret = null,
    string? BranchFilter = null)
    : WebhookConfig(Enabled, Provider, AuthScheme, Secret, BranchFilter);

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(SwarmExternalImage), "External")]
[JsonDerivedType(typeof(SwarmBuildImage), "Build")]
public abstract record SwarmServiceImageInfo;

public sealed record SwarmExternalImage(
    Guid RegistryId,
    string ImageTag,
    string? ResolvedDigest = null) : SwarmServiceImageInfo;

public sealed record SwarmBuildImage(
    Guid BuildProjectId,
    string? ResolvedImageReference = null,
    string? ResolvedDigest = null,
    Guid? ResolvedBuildRunId = null) : SwarmServiceImageInfo;

public sealed record SwarmServicePort(
    int TargetPort,
    int? PublishedPort = null,
    string Protocol = "tcp",
    SwarmServicePortPublishMode PublishMode = SwarmServicePortPublishMode.Ingress);

public sealed record SwarmServiceMount(
    SwarmServiceMountKind Kind,
    string Source,
    string Target,
    bool ReadOnly = false);

public sealed record SwarmServiceSecretReference(string SecretId, string SecretName, string TargetName);
public sealed record SwarmServiceConfigReference(string ConfigId, string ConfigName, string TargetName);

public sealed record SwarmServiceResources(
    long? LimitNanoCpus = null,
    long? LimitMemoryBytes = null,
    long? ReservationNanoCpus = null,
    long? ReservationMemoryBytes = null);

public sealed record SwarmServiceHealthCheck(
    IReadOnlyList<string> Test,
    long? IntervalNanoseconds = null,
    long? TimeoutNanoseconds = null,
    int? Retries = null,
    long? StartPeriodNanoseconds = null);

public sealed record SwarmServiceRestartPolicy(
    SwarmServiceRestartCondition Condition = SwarmServiceRestartCondition.Any,
    long? DelayNanoseconds = null,
    int? MaximumAttempts = null,
    long? WindowNanoseconds = null);

public sealed record SwarmServiceUpdatePolicy(
    int Parallelism = 1,
    long? DelayNanoseconds = null,
    SwarmServiceUpdateOrder Order = SwarmServiceUpdateOrder.StopFirst,
    SwarmServiceUpdateFailureAction FailureAction = SwarmServiceUpdateFailureAction.Pause);

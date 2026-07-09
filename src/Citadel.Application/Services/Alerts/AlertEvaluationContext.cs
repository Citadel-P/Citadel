using Domain;
using Domain.Entities.Alerts;
using Domain.Entities.Stacks;

namespace Application.Services.Alerts;

public sealed record AlertEvaluationContext(
    DateTime UtcNow,
    IReadOnlyCollection<PlatformAlertSnapshot> Platforms,
    IReadOnlyCollection<DeploymentAlertSnapshot> Deployments,
    IReadOnlyCollection<StackAlertSnapshot> Stacks,
    IReadOnlyCollection<ContainerAlertSnapshot>? Containers = null,
    IReadOnlyCollection<StackDriftAlertSnapshot>? StackDrifts = null,
    IReadOnlyCollection<StackGitUpdateAlertSnapshot>? StackGitUpdates = null,
    IReadOnlyCollection<WebhookAlertSnapshot>? Webhooks = null,
    IReadOnlyCollection<GitRepoWebhookSyncFailureAlertSnapshot>? GitRepoWebhookSyncFailures = null,
    IReadOnlyCollection<StackGitWebhookDeployFailureAlertSnapshot>? StackGitWebhookDeployFailures = null,
    IReadOnlyCollection<StackConfigurationResolutionFailureAlertSnapshot>? StackConfigurationFailures = null,
    IReadOnlyCollection<DeploymentConfigurationResolutionFailureAlertSnapshot>? DeploymentConfigurationFailures = null,
    IReadOnlyCollection<AutomationActionRunFailureAlertSnapshot>? AutomationActionRunFailures = null);

public sealed record AlertMatch(
    Guid ResourceId,
    string ResourceName,
    AlertResourceType ResourceType,
    AlertEventInfo? Info,
    string? DeduplicationComponent = null,
    AlertSeverity? Severity = null)
{
    public bool IsMatch => Info is not null;
}

public sealed record ContainerAlertSnapshot(Guid Id, Guid PlatformId, string Name, string PlatformName, string PlatformAddress, string ContainerId);
public sealed record PlatformAlertSnapshot(Guid Id, string Name, double CpuUsage, double RamUsage, string AgentVersion, string Address = "", bool IsOnline = true);
public sealed record DeploymentAlertSnapshot(Guid Id, string Name, string CurrentImage, string PreviousImage, string LatestImage, bool Failed, string? Raison = null);
public sealed record StackAlertSnapshot(
    Guid Id,
    string Name,
    string CurrentImage,
    string PreviousImage,
    string LatestImage,
    bool Failed,
    string? Raison = null,
    IReadOnlyList<StackImageUpdateItem>? Updates = null,
    IReadOnlyList<string>? ServiceNames = null);
public sealed record StackDriftAlertSnapshot(
    Guid Id,
    string Name,
    Guid PlatformId,
    string PlatformName,
    StackDriftReport Report,
    AlertSeverity Severity,
    string Fingerprint,
    IReadOnlyList<string> DriftSummaries,
    StackReconciliationResult? ReconciliationResult = null);
public sealed record StackGitUpdateAlertSnapshot(
    Guid Id,
    string Name,
    string GitRepositoryName,
    string Branch,
    string CurrentCommitSha,
    string RemoteCommitSha,
    bool Failed,
    string? Reason = null);

public sealed record WebhookAlertSnapshot(
    Guid ResourceId,
    string ResourceName,
    AlertResourceType ResourceType,
    string ResourceTypeName,
    string Provider,
    string Execution,
    string Reason,
    Guid RequestId,
    string? EventType,
    string? DeliveryId,
    string? Branch,
    string? CommitSha,
    string? RepositoryFullName);

public sealed record GitRepoWebhookSyncFailureAlertSnapshot(
    Guid Id,
    string Name,
    string Branch,
    string Reason);

public sealed record StackGitWebhookDeployFailureAlertSnapshot(
    Guid Id,
    string Name,
    string GitRepositoryName,
    string Branch,
    string Reason);

public sealed record StackConfigurationResolutionFailureAlertSnapshot(
    Guid Id,
    string Name,
    string Reason);

public sealed record DeploymentConfigurationResolutionFailureAlertSnapshot(
    Guid Id,
    string Name,
    string Reason);

public sealed record AutomationActionRunFailureAlertSnapshot(
    Guid Id,
    string Name,
    Guid RunId,
    ActionRunTrigger Trigger,
    ActionRunStatus Status,
    int? ExitCode,
    long? DurationMs,
    string Reason);

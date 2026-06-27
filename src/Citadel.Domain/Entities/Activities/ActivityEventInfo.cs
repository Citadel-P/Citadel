using Domain.Contracts.Resources.Alerts;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Git;
using Domain.Contracts.Resources.Registries;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Activities;

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(DeploymentCreated), nameof(ActivityEventType.DeploymentCreated))]
[JsonDerivedType(typeof(DeploymentUpdated), nameof(ActivityEventType.DeploymentUpdated))]
[JsonDerivedType(typeof(DeploymentRenamed), nameof(ActivityEventType.DeploymentRenamed))]
[JsonDerivedType(typeof(DeploymentDeleted), nameof(ActivityEventType.DeploymentDeleted))]
[JsonDerivedType(typeof(DeploymentStarted), nameof(ActivityEventType.DeploymentStarted))]
[JsonDerivedType(typeof(DeploymentStopped), nameof(ActivityEventType.DeploymentStopped))]
[JsonDerivedType(typeof(DeploymentPaused), nameof(ActivityEventType.DeploymentPaused))]
[JsonDerivedType(typeof(DeploymentApplied), nameof(ActivityEventType.DeploymentApplied))]
[JsonDerivedType(typeof(DeploymentDegraded), nameof(ActivityEventType.DeploymentDegraded))]
[JsonDerivedType(typeof(StackCreated), nameof(ActivityEventType.StackCreated))]
[JsonDerivedType(typeof(StackUpdated), nameof(ActivityEventType.StackUpdated))]
[JsonDerivedType(typeof(StackRenamed), nameof(ActivityEventType.StackRenamed))]
[JsonDerivedType(typeof(StackDeleted), nameof(ActivityEventType.StackDeleted))]
[JsonDerivedType(typeof(StackStarted), nameof(ActivityEventType.StackStarted))]
[JsonDerivedType(typeof(StackStopped), nameof(ActivityEventType.StackStopped))]
[JsonDerivedType(typeof(StackPaused), nameof(ActivityEventType.StackPaused))]
[JsonDerivedType(typeof(StackApplied), nameof(ActivityEventType.StackApplied))]
[JsonDerivedType(typeof(StackRollback), nameof(ActivityEventType.StackRollback))]
[JsonDerivedType(typeof(StackDegraded), nameof(ActivityEventType.StackDegraded))]
[JsonDerivedType(typeof(StackDriftDetected), nameof(ActivityEventType.StackDriftDetected))]
[JsonDerivedType(typeof(StackDriftResolved), nameof(ActivityEventType.StackDriftResolved))]
[JsonDerivedType(typeof(StackReconciliationAttempted), nameof(ActivityEventType.StackReconciliationAttempted))]
[JsonDerivedType(typeof(StackGitUpdateAvailable), nameof(ActivityEventType.StackGitUpdateAvailable))]
[JsonDerivedType(typeof(StackGitAutoUpdated), nameof(ActivityEventType.StackGitAutoUpdated))]
[JsonDerivedType(typeof(StackGitAutoDeployFailed), nameof(ActivityEventType.StackGitAutoDeployFailed))]
[JsonDerivedType(typeof(AlertRuleCreated), nameof(ActivityEventType.AlertRuleCreated))]
[JsonDerivedType(typeof(AlertRuleUpdated), nameof(ActivityEventType.AlertRuleUpdated))]
[JsonDerivedType(typeof(AlertRuleDeleted), nameof(ActivityEventType.AlertRuleDeleted))]
[JsonDerivedType(typeof(AlertRuleRenamed), nameof(ActivityEventType.AlertRuleRenamed))]
[JsonDerivedType(typeof(RegistryRenamed), nameof(ActivityEventType.RegistryRenamed))]
[JsonDerivedType(typeof(RegistryCreated), nameof(ActivityEventType.RegistryCreated))]
[JsonDerivedType(typeof(RegistryUpdated), nameof(ActivityEventType.RegistryUpdated))]
[JsonDerivedType(typeof(RegistryDeleted), nameof(ActivityEventType.RegistryDeleted))]
[JsonDerivedType(typeof(GitRepoCreated), nameof(ActivityEventType.GitRepoCreated))]
[JsonDerivedType(typeof(GitRepoUpdated), nameof(ActivityEventType.GitRepoUpdated))]
[JsonDerivedType(typeof(GitRepoRenamed), nameof(ActivityEventType.GitRepoRenamed))]
[JsonDerivedType(typeof(GitRepoDeleted), nameof(ActivityEventType.GitRepoDeleted))]
[JsonDerivedType(typeof(GitRepoCloned), nameof(ActivityEventType.GitRepoCloned))]
[JsonDerivedType(typeof(GitRepoPulled), nameof(ActivityEventType.GitRepoPulled))]
[JsonDerivedType(typeof(GitRepoWebhookReceived), nameof(ActivityEventType.GitRepoWebhookReceived))]
[JsonDerivedType(typeof(StackWebhookReceived), nameof(ActivityEventType.StackWebhookReceived))]

public abstract record ActivityEventInfo;

public sealed record DeploymentCreated(DeploymentSnapshot Deployment) : ActivityEventInfo;
public sealed record DeploymentUpdated(DeploymentSnapshot OldDeployment, DeploymentSnapshot NewDeployment) : ActivityEventInfo;
public sealed record DeploymentRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record DeploymentDeleted(DeploymentSnapshot Deployment) : ActivityEventInfo;
public sealed record DeploymentStarted(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentStopped(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentPaused(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentDegraded(string Reason) : ActivityEventInfo;
public sealed record DeploymentApplied(DeploymentSnapshot? Deployment, DeploymentResultSnapshot Result) : ActivityEventInfo;

public sealed record StackCreated(StackSnapshot Stack) : ActivityEventInfo;
public sealed record StackUpdated(StackSnapshot OldStack, StackSnapshot NewStack) : ActivityEventInfo;
public sealed record StackRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record StackDeleted(StackSnapshot Stack) : ActivityEventInfo;
public sealed record StackStarted(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record StackStopped(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record StackPaused(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record StackDegraded(string Reason) : ActivityEventInfo;
public sealed record StackDriftDetected(string Reason, string Fingerprint) : ActivityEventInfo;
public sealed record StackDriftResolved(string PreviousFingerprint) : ActivityEventInfo;
public sealed record StackReconciliationAttempted(
    StackReconciliationStatus Status,
    IReadOnlyList<StackReconciliationAction> Actions,
    string DriftFingerprint) : ActivityEventInfo;
public sealed record StackApplied(StackSnapshot? Stack, StackResultSnapshot Result) : ActivityEventInfo;
public sealed record StackRollback(StackSnapshot? OldStack, StackSnapshot? NewStack, StackResultSnapshot Result) : ActivityEventInfo;
public sealed record StackGitUpdateAvailable(
    string GitRepositoryName,
    string Branch,
    string CurrentCommitSha,
    string RemoteCommitSha) : ActivityEventInfo;
public sealed record StackGitAutoUpdated(
    string GitRepositoryName,
    string Branch,
    string PreviousCommitSha,
    string UpdatedCommitSha) : ActivityEventInfo;
public sealed record StackGitAutoDeployFailed(
    string GitRepositoryName,
    string Branch,
    string CurrentCommitSha,
    string RemoteCommitSha,
    string Reason) : ActivityEventInfo;
public sealed record StackWebhookReceived(
    Guid RequestId,
    string AuthType,
    string Execution,
    string Status,
    string? Reason,
    string? EventType,
    string? DeliveryId,
    string? Branch,
    string? CommitSha,
    string? RepositoryFullName,
    string? DispatchedBranch = null,
    string? DispatchedCommitSha = null) : ActivityEventInfo;


public sealed record AlertRuleCreated(AlertRuleSnapshot AlertRule) : ActivityEventInfo;
public sealed record AlertRuleUpdated(AlertRuleSnapshot OldRule, AlertRuleSnapshot NewRule) : ActivityEventInfo;
public sealed record AlertRuleDeleted(AlertRuleSnapshot AlertRule) : ActivityEventInfo;
public sealed record AlertRuleRenamed(string OldName, string NewName) : ActivityEventInfo;

public sealed record RegistryRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record RegistryDeleted(RegistrySnapshot Registry) : ActivityEventInfo;
public sealed record RegistryUpdated(RegistrySnapshot OldRegistry, RegistrySnapshot NewRegistry) : ActivityEventInfo;
public sealed record RegistryCreated(RegistrySnapshot Registry) : ActivityEventInfo;
public sealed record GitRepoCreated(GitRepositorySnapshot GitRepo) : ActivityEventInfo;
public sealed record GitRepoUpdated(GitRepositorySnapshot OldGitRepo, GitRepositorySnapshot NewGitRepo) : ActivityEventInfo;
public sealed record GitRepoRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record GitRepoDeleted(GitRepositorySnapshot GitRepo) : ActivityEventInfo;
public sealed record GitRepoCloned(GitRepositorySnapshot GitRepo, RepoSyncResultSnapshot Result) : ActivityEventInfo;
public sealed record GitRepoPulled(GitRepositorySnapshot GitRepo, RepoSyncResultSnapshot Result) : ActivityEventInfo;
public sealed record GitRepoWebhookReceived(
    Guid RequestId,
    string AuthType,
    string Execution,
    string Status,
    string? Reason,
    string? EventType,
    string? DeliveryId,
    string? Branch,
    string? CommitSha,
    string? RepositoryFullName,
    string? DispatchedBranch = null,
    string? DispatchedCommitSha = null) : ActivityEventInfo;

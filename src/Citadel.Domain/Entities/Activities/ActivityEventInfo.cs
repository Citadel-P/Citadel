using Domain.Contracts.Resources.Alerts;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Git;
using Domain.Contracts.Resources.Registries;
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

public abstract record ActivityEventInfo;

public sealed record DeploymentCreated(DeploymentSnapshot Deployment) : ActivityEventInfo;
public sealed record DeploymentUpdated(DeploymentSnapshot OldDeployment, DeploymentSnapshot NewDeployment) : ActivityEventInfo;
public sealed record DeploymentRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record DeploymentDeleted(DeploymentSnapshot Deployment) : ActivityEventInfo;
public sealed record DeploymentStarted(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentStopped(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentPaused(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentDegraded(string Reason) : ActivityEventInfo;
public sealed record DeploymentApplied(DeploymentSnapshot? Deployment, IEnumerable<string>? ContainerIds, string? Reason) : ActivityEventInfo;
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
public sealed record GitRepoCloned(GitRepositorySnapshot GitRepo, string? Reason) : ActivityEventInfo;
public sealed record GitRepoPulled(GitRepositorySnapshot GitRepo, string? Reason) : ActivityEventInfo;
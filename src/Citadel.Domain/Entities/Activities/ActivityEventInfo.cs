using Domain.Contracts.Resources.Alerts;
using Domain.Entities.Deployments;
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
public abstract record ActivityEventInfo;

public sealed record DeploymentCreated(DeploymentSpec Spec) : ActivityEventInfo;
public sealed record DeploymentUpdated(DeploymentSpec OldSpec, DeploymentSpec NewSpec) : ActivityEventInfo;
public sealed record DeploymentRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record DeploymentDeleted(DeploymentSpec Spec) : ActivityEventInfo;
public sealed record DeploymentStarted(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentStopped(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentPaused(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentDegraded(string Reason) : ActivityEventInfo;
public sealed record DeploymentApplied(DeploymentSpec? Spec, IEnumerable<string>? ContainerIds, string? Reason) : ActivityEventInfo;
public sealed record AlertRuleCreated(AlertRuleSnapshot AlertRule) : ActivityEventInfo;
public sealed record AlertRuleUpdated(AlertRuleSnapshot OldRule, AlertRuleSnapshot NewRule) : ActivityEventInfo;
public sealed record AlertRuleDeleted(AlertRuleSnapshot AlertRule) : ActivityEventInfo;



using Domain;
using Domain.Entities;
using Domain.Entities.Alerts;
using Hosting.Common;

namespace Application.Services.Alerts;

public interface IAlertEvaluator
{
    AlertType Type { get; }

    IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context);
}

/// <summary>
/// Attribute used to mark alert evaluators. 
/// The source generator <see cref="AlertEvaluatorRegistryGenerator"/> will find all classes marked with this attribute and register them in the DI container.
/// </summary>
/// <param name="type"></param>
[AttributeUsage(AttributeTargets.Class)]
public sealed class AlertEvaluatorAttribute(AlertType type) : Attribute;

#region Platform

[AlertEvaluator(AlertType.PlatformCpuHigh)]
public sealed class PlatformCpuHighEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.PlatformCpuHigh;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var platform in context.Platforms)
        {
            if (platform.CpuUsage < rule.Threshold)
                continue;

            yield return new AlertMatch(
                platform.Id,
                platform.Name,
                AlertResourceType.Platform,
                new PlatformCpuHighAlertInfo(platform.Name, platform.CpuUsage)
            );
        }
    }
}

[AlertEvaluator(AlertType.PlatformRamHigh)]
public sealed class PlatformRamHighEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.PlatformRamHigh;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var platform in context.Platforms)
        {
            if (platform.RamUsage < rule.Threshold)
                continue;

            yield return new AlertMatch(
                platform.Id,
                platform.Name,
                AlertResourceType.Platform,
                new PlatformRamHighAlertInfo(platform.Name, platform.RamUsage)
            );
        }
    }
}

[AlertEvaluator(AlertType.PlatformVersionMismatch)]
public sealed class PlatformVersionMismatchEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.PlatformVersionMismatch;
    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var platform in context.Platforms)
        {
            if (IsLocal(platform.AgentVersion) || IsCompatible(platform.AgentVersion))
                continue;

            yield return new AlertMatch(
                platform.Id,
                platform.Name,
                AlertResourceType.Platform,
                new PlatformVersionMismatchAlertInfo(platform.Name, platform.AgentVersion, Constants.CompatibilityVersion)
            );
        }
    }

    public static bool IsCompatible(string agentVersion) =>
        agentVersion.Equals(Constants.CompatibilityVersion, StringComparison.Ordinal);

    public static bool IsLocal(string agentVersion) => string.IsNullOrEmpty(agentVersion);
}

[AlertEvaluator(AlertType.PlatformUnreachable)]
public sealed class PlatformUnreachableEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.PlatformUnreachable;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var platform in context.Platforms)
        {
            if (platform.IsOnline)
                continue;

            yield return new AlertMatch(
                platform.Id,
                platform.Name,
                AlertResourceType.Platform,
                new PlatformUnreachableAlertInfo(platform.Name, platform.Id, platform.Address));
        }
    }
}

[AlertEvaluator(AlertType.UnmanagedContainerCreated)]
public sealed class UnmanagedContainerCreatedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.UnmanagedContainerCreated;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        if (context.Containers is null)
            yield break;

        foreach (var container in context.Containers)
        {
            yield return new AlertMatch(
                container.PlatformId,
                container.PlatformName,
                AlertResourceType.Platform,
                new UnmanagedContainerCreatedAlertInfo(container.PlatformName, container.PlatformAddress, container.Name, container.ContainerId),
                container.Name);
        }
    }
}

#endregion

#region Deployment

[AlertEvaluator(AlertType.DeploymentAutoDeployFailed)]
public sealed class DeploymentFailedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.DeploymentAutoDeployFailed;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var deployment in context.Deployments)
        {
            if (!deployment.Failed)
                continue;

            yield return new AlertMatch(
                deployment.Id,
                deployment.Name,
                AlertResourceType.Deployment,
                new DeploymentAutoDeployFailedAlertInfo(deployment.Name, $"Deployment failed: {deployment.Raison}"));
        }
    }
}

[AlertEvaluator(AlertType.DeploymentImageUpdateAvailable)]
public sealed class DeploymentImageUpdateAvailableEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.DeploymentImageUpdateAvailable;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var deployment in context.Deployments)
        {
            yield return new AlertMatch(
                deployment.Id,
                deployment.Name,
                AlertResourceType.Deployment,
                new DeploymentImageUpdateAvailableAlertInfo(
                    deployment.Name,
                    deployment.PreviousImage,
                    deployment.LatestImage));
        }
    }
}

[AlertEvaluator(AlertType.DeploymentAutoUpdated)]
public sealed class DeploymentAutoUpdatedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.DeploymentAutoUpdated;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var deployment in context.Deployments)
        {
            yield return new AlertMatch(
                deployment.Id,
                deployment.Name,
                AlertResourceType.Deployment,
                new DeploymentAutoUpdatedAlertInfo(
                    deployment.Name,
                    deployment.PreviousImage,
                    deployment.CurrentImage));
        }
    }
}

#endregion

#region Stack
[AlertEvaluator(AlertType.StackAutoDeployFailed)]
public sealed class StackDeployFailedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.StackAutoDeployFailed;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var stack in context.Stacks)
        {
            if (!stack.Failed)
                continue;

            yield return new AlertMatch(
                stack.Id,
                stack.Name,
                AlertResourceType.Stack,
                new StackDeployFailedAlertInfo(stack.Name, $"Stack deployment failed: {stack.Raison}"));
        }
    }
}

[AlertEvaluator(AlertType.StackImageUpdateAvailable)]
public sealed class StackImageUpdateAvailableEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.StackImageUpdateAvailable;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var stack in context.Stacks)
        {
            yield return new AlertMatch(
                stack.Id,
                stack.Name,
                AlertResourceType.Stack,
                new StackImageUpdateAvailableAlertInfo(
                    stack.Name,
                    stack.Updates ?? []));
        }
    }
}

[AlertEvaluator(AlertType.StackAutoUpdated)]
public sealed class StackAutoUpdatedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.StackAutoUpdated;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var stack in context.Stacks)
        {
            yield return new AlertMatch(
                stack.Id,
                stack.Name,
                AlertResourceType.Stack,
                new StackAutoUpdatedAlertInfo(
                    stack.Name,
                    stack.Updates ?? []));
        }
    }
}

[AlertEvaluator(AlertType.StackServiceAutoDeployFailed)]
public sealed class StackServiceDeployFailedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.StackServiceAutoDeployFailed;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var stack in context.Stacks)
        {
            if (!stack.Failed)
                continue;

            yield return new AlertMatch(
                stack.Id,
                stack.Name,
                AlertResourceType.Stack,
                new StackServiceAutoDeployFailedAlertInfo(
                    stack.Name,
                    stack.ServiceNames ?? [],
                    $"Stack service deployment failed: {stack.Raison}"));
        }
    }
}

[AlertEvaluator(AlertType.StackServiceAutoUpdated)]
public sealed class StackServiceAutoUpdatedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.StackServiceAutoUpdated;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        foreach (var stack in context.Stacks)
        {
            yield return new AlertMatch(
                stack.Id,
                stack.Name,
                AlertResourceType.Stack,
                new StackServiceAutoUpdatedAlertInfo(
                    stack.Name,
                    stack.Updates ?? []));
        }
    }
}

[AlertEvaluator(AlertType.StackDriftDetected)]
public sealed class StackDriftDetectedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.StackDriftDetected;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        if (context.StackDrifts is null)
            yield break;

        foreach (var stack in context.StackDrifts)
        {
            if (!stack.Report.HasDrift)
                continue;

            yield return new AlertMatch(
                stack.Id,
                stack.Name,
                AlertResourceType.Stack,
                new StackDriftDetectedAlertInfo(
                    StackId: stack.Id,
                    StackName: stack.Name,
                    PlatformId: stack.PlatformId,
                    PlatformName: stack.PlatformName,
                    DriftCount: stack.Report.Drifts.Count,
                    HasAutoFixableDrift: stack.Report.HasAutoFixableDrift,
                    HasStructuralDrift: stack.Report.HasStructuralDrift,
                    DriftSummaries: stack.DriftSummaries),
                DeduplicationComponent: stack.Fingerprint,
                Severity: stack.Severity);
        }
    }
}

[AlertEvaluator(AlertType.StackDriftAutoReconciled)]
public sealed class StackDriftAutoReconciledEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.StackDriftAutoReconciled;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        if (context.StackDrifts is null)
            yield break;

        foreach (var stack in context.StackDrifts)
        {
            if (stack.ReconciliationResult?.Status != StackReconciliationStatus.Reconciled)
                continue;

            yield return new AlertMatch(
                stack.Id,
                stack.Name,
                AlertResourceType.Stack,
                new StackDriftAutoReconciledAlertInfo(
                    StackId: stack.Id,
                    StackName: stack.Name,
                    PlatformId: stack.PlatformId,
                    PlatformName: stack.PlatformName,
                    DriftCount: stack.Report.Drifts.Count,
                    DriftSummaries: stack.DriftSummaries,
                    Actions: stack.ReconciliationResult.Actions),
                DeduplicationComponent: stack.Fingerprint,
                Severity: AlertSeverity.Info);
        }
    }
}

[AlertEvaluator(AlertType.StackGitUpdateAvailable)]
public sealed class StackGitUpdateAvailableEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.StackGitUpdateAvailable;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        if (context.StackGitUpdates is null)
            yield break;

        foreach (var stack in context.StackGitUpdates)
        {
            if (stack.Failed)
                continue;

            yield return new AlertMatch(
                stack.Id,
                stack.Name,
                AlertResourceType.Stack,
                new StackGitUpdateAvailableAlertInfo(
                    stack.Name,
                    stack.GitRepositoryName,
                    stack.Branch,
                    stack.CurrentCommitSha,
                    stack.RemoteCommitSha),
                DeduplicationComponent: $"{stack.GitRepositoryName}:{stack.Branch}:{stack.RemoteCommitSha}");
        }
    }
}

[AlertEvaluator(AlertType.StackGitAutoUpdated)]
public sealed class StackGitAutoUpdatedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.StackGitAutoUpdated;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        if (context.StackGitUpdates is null)
            yield break;

        foreach (var stack in context.StackGitUpdates)
        {
            if (stack.Failed)
                continue;

            yield return new AlertMatch(
                stack.Id,
                stack.Name,
                AlertResourceType.Stack,
                new StackGitAutoUpdatedAlertInfo(
                    stack.Name,
                    stack.GitRepositoryName,
                    stack.Branch,
                    stack.CurrentCommitSha,
                    stack.RemoteCommitSha),
                DeduplicationComponent: $"{stack.GitRepositoryName}:{stack.Branch}:{stack.RemoteCommitSha}");
        }
    }
}

[AlertEvaluator(AlertType.StackGitAutoDeployFailed)]
public sealed class StackGitAutoDeployFailedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.StackGitAutoDeployFailed;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        if (context.StackGitUpdates is null)
            yield break;

        foreach (var stack in context.StackGitUpdates)
        {
            if (!stack.Failed)
                continue;

            yield return new AlertMatch(
                stack.Id,
                stack.Name,
                AlertResourceType.Stack,
                new StackGitAutoDeployFailedAlertInfo(
                    stack.Name,
                    stack.GitRepositoryName,
                    stack.Branch,
                    stack.CurrentCommitSha,
                    stack.RemoteCommitSha,
                    stack.Reason ?? "Auto-deploy failed."),
                DeduplicationComponent: $"{stack.GitRepositoryName}:{stack.Branch}:{stack.RemoteCommitSha}");
        }
    }
}

[AlertEvaluator(AlertType.WebhookAuthenticationFailed)]
public sealed class WebhookAuthenticationFailedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.WebhookAuthenticationFailed;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        if (context.Webhooks is null)
            yield break;

        foreach (var webhook in context.Webhooks)
        {
            yield return new AlertMatch(
                webhook.ResourceId,
                webhook.ResourceName,
                webhook.ResourceType,
                new WebhookAuthenticationFailedAlertInfo(
                    webhook.ResourceName,
                    webhook.ResourceTypeName,
                    webhook.Provider,
                    webhook.Execution,
                    webhook.Reason,
                    webhook.RequestId,
                    webhook.EventType,
                    webhook.DeliveryId,
                    webhook.Branch,
                    webhook.CommitSha,
                    webhook.RepositoryFullName),
                DeduplicationComponent: $"{webhook.Provider}:{webhook.Execution}:{webhook.Reason}:{webhook.DeliveryId ?? webhook.RequestId.ToString()}");
        }
    }
}

[AlertEvaluator(AlertType.WebhookDispatchFailed)]
public sealed class WebhookDispatchFailedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.WebhookDispatchFailed;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        if (context.Webhooks is null)
            yield break;

        foreach (var webhook in context.Webhooks)
        {
            yield return new AlertMatch(
                webhook.ResourceId,
                webhook.ResourceName,
                webhook.ResourceType,
                new WebhookDispatchFailedAlertInfo(
                    webhook.ResourceName,
                    webhook.ResourceTypeName,
                    webhook.Provider,
                    webhook.Execution,
                    webhook.Reason,
                    webhook.RequestId,
                    webhook.EventType,
                    webhook.DeliveryId,
                    webhook.Branch,
                    webhook.CommitSha,
                    webhook.RepositoryFullName),
                DeduplicationComponent: $"{webhook.Provider}:{webhook.Execution}:{webhook.Reason}:{webhook.DeliveryId ?? webhook.RequestId.ToString()}");
        }
    }
}

[AlertEvaluator(AlertType.WebhookGitRepoSyncFailed)]
public sealed class WebhookGitRepoSyncFailedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.WebhookGitRepoSyncFailed;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        if (context.GitRepoWebhookSyncFailures is null)
            yield break;

        foreach (var failure in context.GitRepoWebhookSyncFailures)
        {
            yield return new AlertMatch(
                failure.Id,
                failure.Name,
                AlertResourceType.Webhook,
                new WebhookGitRepoSyncFailedAlertInfo(
                    failure.Name,
                    failure.Branch,
                    failure.Reason),
                DeduplicationComponent: $"{failure.Branch}:{failure.Reason}");
        }
    }
}

[AlertEvaluator(AlertType.WebhookStackGitDeployFailed)]
public sealed class WebhookStackGitDeployFailedEvaluator : IAlertEvaluator
{
    public AlertType Type => AlertType.WebhookStackGitDeployFailed;

    public IEnumerable<AlertMatch> Evaluate(AlertRule rule, AlertEvaluationContext context)
    {
        if (context.StackGitWebhookDeployFailures is null)
            yield break;

        foreach (var failure in context.StackGitWebhookDeployFailures)
        {
            yield return new AlertMatch(
                failure.Id,
                failure.Name,
                AlertResourceType.Webhook,
                new WebhookStackGitDeployFailedAlertInfo(
                    failure.Name,
                    failure.GitRepositoryName,
                    failure.Branch,
                    failure.Reason),
                DeduplicationComponent: $"{failure.GitRepositoryName}:{failure.Branch}:{failure.Reason}");
        }
    }
}
#endregion

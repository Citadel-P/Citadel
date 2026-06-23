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
                    stack.CurrentImage,
                    stack.LatestImage));
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
                    stack.PreviousImage,
                    stack.CurrentImage));
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
#endregion

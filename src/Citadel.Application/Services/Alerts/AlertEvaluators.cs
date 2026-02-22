using Domain;
using Domain.Entities;
using Domain.Entities.Alerts;

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
                new PlatformCpuHighAlertInfo(platform.CpuUsage)
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
                new PlatformRamHighAlertInfo(platform.RamUsage)
            );
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
                new DeploymentFailedAlertInfo($"Deployment failed: {deployment.Raison}"));
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
                new StackDeployFailedAlertInfo($"Stack deployment failed: {stack.Raison}"));
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
                    stack.PreviousImage,
                    stack.CurrentImage));
        }
    }
}
#endregion
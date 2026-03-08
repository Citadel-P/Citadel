using Domain;
using Domain.Entities.Alerts;

namespace Application.Services.Alerts;

public sealed record AlertEvaluationContext(
    DateTime UtcNow,
    IReadOnlyCollection<PlatformAlertSnapshot> Platforms,
    IReadOnlyCollection<DeploymentAlertSnapshot> Deployments,
    IReadOnlyCollection<StackAlertSnapshot> Stacks);

public sealed record AlertMatch(Guid ResourceId, string ResourceName, AlertResourceType ResourceType, AlertEventInfo? Info)
{
    public bool IsMatch => Info is not null;
}

public sealed record PlatformAlertSnapshot(Guid Id, string Name, double CpuUsage, double RamUsage, string AgentVersion);
public sealed record DeploymentAlertSnapshot(Guid Id, string Name, string CurrentImage, string PreviousImage, string LatestImage, bool Failed, string? Raison = null);
public sealed record StackAlertSnapshot(Guid Id, string Name, string CurrentImage, string PreviousImage, string LatestImage, bool Failed, string? Raison = null);

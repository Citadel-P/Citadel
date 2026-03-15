using Domain;
using Domain.Entities.Alerts;

namespace Application.Services.Alerts;

public sealed record AlertEvaluationContext(
    DateTime UtcNow,
    IReadOnlyCollection<PlatformAlertSnapshot> Platforms,
    IReadOnlyCollection<DeploymentAlertSnapshot> Deployments,
    IReadOnlyCollection<StackAlertSnapshot> Stacks,
    IReadOnlyCollection<ContainerAlertSnapshot>? Containers = null);

public sealed record AlertMatch(Guid ResourceId, string ResourceName, AlertResourceType ResourceType, AlertEventInfo? Info)
{
    public bool IsMatch => Info is not null;
}

public sealed record ContainerAlertSnapshot(Guid Id, Guid PlatformId, string Name, string PlatformName, string PlatformAddress);
public sealed record PlatformAlertSnapshot(Guid Id, string Name, double CpuUsage, double RamUsage, string AgentVersion, string Address = "", bool IsOnline = true);
public sealed record DeploymentAlertSnapshot(Guid Id, string Name, string CurrentImage, string PreviousImage, string LatestImage, bool Failed, string? Raison = null);
public sealed record StackAlertSnapshot(Guid Id, string Name, string CurrentImage, string PreviousImage, string LatestImage, bool Failed, string? Raison = null);

using Domain;
using Domain.Entities;

namespace Application.Services.Alerts;

public sealed record AlertEvaluationContext(
    DateTime UtcNow,
    IReadOnlyCollection<PlatformAlertSnapshot> Platforms,
    IReadOnlyCollection<DeploymentAlertSnapshot> Deployments,
    IReadOnlyCollection<StackAlertSnapshot> Stacks);

public sealed record AlertMatch(Guid? ResourceId, AlertResourceType ResourceType, AlertInfo? Info)
{
    public bool IsMatch => Info is not null;
}

public sealed record PlatformAlertSnapshot(Guid Id, double CpuUsage, double RamUsage, string Version);
public sealed record DeploymentAlertSnapshot(Guid Id, string CurrentImage, string PreviousImage, string LatestImage, bool Failed);
public sealed record StackAlertSnapshot(Guid Id, string CurrentImage, string PreviousImage, string LatestImage, bool Failed);

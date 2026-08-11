using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Swarm.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmOverview(Guid PlatformId) : IQuery<Result<SwarmOverviewResult>>;

public sealed record SwarmOverviewResult(
    PlatformStatus ConnectionStatus,
    bool ControlAvailable,
    string? Error,
    SwarmProjectionSummary Summary,
    SwarmQuorumResult Quorum);

public sealed record SwarmQuorumResult(
    SwarmQuorumState State,
    int ReachableManagers,
    int RequiredManagers,
    bool HasLeader)
{
    public static SwarmQuorumResult Calculate(
        PlatformStatus connectionStatus,
        SwarmProjectionSummary summary)
    {
        var requiredManagers = summary.ManagerCount == 0 ? 0 : (summary.ManagerCount / 2) + 1;
        var state = connectionStatus != PlatformStatus.Online ||
                    summary.ManagerCount == 0 ||
                    summary.IsManagerInventoryStale
            ? SwarmQuorumState.Unknown
            : !summary.HasLeader || summary.ReachableManagerCount < requiredManagers
                ? SwarmQuorumState.Lost
                : summary.ReachableManagerCount < summary.ManagerCount
                    ? SwarmQuorumState.Degraded
                    : SwarmQuorumState.Healthy;

        return new SwarmQuorumResult(
            state,
            summary.ReachableManagerCount,
            requiredManagers,
            summary.HasLeader);
    }
}

internal sealed class GetSwarmOverviewHandler(
    IUnitOfWork unitOfWork,
    ISwarmReconciliationCoordinator reconciliationCoordinator)
    : IQueryHandler<GetSwarmOverview, Result<SwarmOverviewResult>>
{
    public async ValueTask<Result<SwarmOverviewResult>> Handle(
        GetSwarmOverview query,
        CancellationToken cancellationToken)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(
            unitOfWork,
            reconciliationCoordinator,
            query.PlatformId,
            cancellationToken);
        if (error is not null)
            return Result.Failure<SwarmOverviewResult>(error);

        var platform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        var descriptor = (DockerSwarmPlatformDescriptor)platform!.PlatformDescriptor;
        var summary = await unitOfWork.Swarm.GetSummaryAsync(query.PlatformId, cancellationToken);
        return Result.Success(new SwarmOverviewResult(
            platform.Status,
            descriptor.ControlAvailable,
            descriptor.Error,
            summary,
            SwarmQuorumResult.Calculate(platform.Status, summary)));
    }
}

using Domain.Contracts.Interfaces;
using Domain.Entities;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Queries;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read)]
public sealed record GetDeploymentStats(Guid Id) : IQuery<Result<IEnumerable<ContainerStat>>>;

internal sealed class GetDeploymentStatsHandler(
    IUnitOfWork unitOfWork) : IQueryHandler<GetDeploymentStats, Result<IEnumerable<ContainerStat>>>
{
    public async ValueTask<Result<IEnumerable<ContainerStat>>> Handle(GetDeploymentStats query, CancellationToken cancellationToken)
    {
        var containerId = await unitOfWork.Deployments.GetContainerIdAsync(query.Id, cancellationToken);
        if (containerId is null)
        {
            return Result.Failure<IEnumerable<ContainerStat>>(new NotFoundError("Container does not exist"));
        }

        var result = await unitOfWork.ContainerStats.GetStatsAggregatedLast24HoursAsync(containerId, cancellationToken);
        return Result.Success(result);
    }
}

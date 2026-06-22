using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Queries;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read)]
public sealed record GetDeploymentStats(Guid Id, int Hours = 24) : IQuery<Result<IEnumerable<ContainerStat>>>
{
    internal sealed class Validator : AbstractValidator<GetDeploymentStats>
    {
        public Validator()
            => RuleFor(s => s.Hours)
                .Must(hours => hours is 24 or 48 or 72)
                .WithMessage("Hours must be one of: 24, 48, 72.");
    }
}

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

        var result = await unitOfWork.ContainerStats.GetStatsAggregatedAsync(containerId, query.Hours, cancellationToken);
        return Result.Success(result);
    }
}

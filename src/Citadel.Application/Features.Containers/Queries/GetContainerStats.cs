using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Queries;

public sealed record GetContainerStats(string ContainerId) : IQuery<Result<IEnumerable<ContainerStat>>>
{
    internal class Validator : AbstractValidator<GetContainerById>
    {
        public Validator()
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal sealed class GetContainerStatsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetContainerStats, Result<IEnumerable<ContainerStat>>>
{
    public async ValueTask<Result<IEnumerable<ContainerStat>>> Handle(GetContainerStats query, CancellationToken cancellationToken)
    {
        var result = await unitOfWork.ContainerStats.GetStatsAggregatedLast24HoursAsync(query.ContainerId, cancellationToken);
        return Result.Success(result);
    }
}
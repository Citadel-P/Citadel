using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

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
        var containerId = await unitOfWork.Containers
            .Query().AsNoTracking().SingleOrDefaultAsync(s => s.ContainerId.StartsWith(query.ContainerId), cancellationToken);

        var last24h = DateTimeOffset.UtcNow.AddHours(-24).ToUnixTimeSeconds();
        return containerId == null
            ? Result.Failure<IEnumerable<ContainerStat>>(new NotFoundError($"Container with id {query.ContainerId} does not exist"))
            : await unitOfWork.ContainerStats.Query().AsNoTracking()
                    .Where(s => s.ContainerId == containerId.Id && s.Created > last24h)
                    .ToArrayAsync(cancellationToken);
    }
}
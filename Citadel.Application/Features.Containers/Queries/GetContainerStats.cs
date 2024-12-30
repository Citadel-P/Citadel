using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
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

internal sealed class GetContainerStatsHandler(ApplicationDbContext dbContext)
    : IQueryHandler<GetContainerStats, Result<IEnumerable<ContainerStat>>>
{
    public async ValueTask<Result<IEnumerable<ContainerStat>>> Handle(GetContainerStats query, CancellationToken cancellationToken)
    {
        var containerId = await dbContext.ContainersInfo.AsNoTracking()
                        .SingleOrDefaultAsync(s => s.ContainerId.StartsWith(query.ContainerId), cancellationToken);

        var last24h = DateTimeOffset.UtcNow.AddHours(-24).ToUnixTimeSeconds();
        return containerId == null
            ? Result.Failure<IEnumerable<ContainerStat>>(new NotFoundError($"Container with id {query.ContainerId} does not exists"))
            : await dbContext.ContainerStats.AsNoTracking()
                    .Where(s => s.ContainerInfoId == containerId.Id && s.Created > last24h)
                    .ToListAsync(cancellationToken);
    }
}
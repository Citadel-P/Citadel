using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
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

internal sealed class GetContainerStatsHandler(
    IUnitOfWork unitOfWork,
    IContainerPlatformAuthorizationService containerPlatformAuthorizationService) : IQueryHandler<GetContainerStats, Result<IEnumerable<ContainerStat>>>
{
    public async ValueTask<Result<IEnumerable<ContainerStat>>> Handle(GetContainerStats query, CancellationToken cancellationToken)
    {
        var hasAccess = await containerPlatformAuthorizationService.HasAccessAsync([query.ContainerId], PermissionLevel.Read, SpecificPermission.None, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure<IEnumerable<ContainerStat>>(new ForbiddenError("Missing permission [Read] on [Platform]"));
        }

        var result = await unitOfWork.ContainerStats.GetStatsAggregatedLast24HoursAsync(query.ContainerId, cancellationToken);
        return Result.Success(result);
    }
}
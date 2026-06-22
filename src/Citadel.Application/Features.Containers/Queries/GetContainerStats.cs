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

public sealed record GetContainerStats(string ContainerId, int Hours = 24) : IQuery<Result<IEnumerable<ContainerStat>>>
{
    internal class Validator : AbstractValidator<GetContainerStats>
    {
        public Validator()
        {
            RuleFor(s => s.ContainerId).ValidContainerId();
            RuleFor(s => s.Hours)
                .Must(hours => hours is 24 or 48 or 72)
                .WithMessage("Hours must be one of: 24, 48, 72.");
        }
    }
}

internal sealed class GetContainerStatsHandler(
    IUnitOfWork unitOfWork,
    IContainerAuthorizationService containerAuthorizationService) : IQueryHandler<GetContainerStats, Result<IEnumerable<ContainerStat>>>
{
    public async ValueTask<Result<IEnumerable<ContainerStat>>> Handle(GetContainerStats query, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync([query.ContainerId], ResourceType.Platform, PermissionLevel.Read, SpecificPermission.None, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure<IEnumerable<ContainerStat>>(new ForbiddenError("Missing permission [Read] on [Platform]"));
        }

        var result = await unitOfWork.ContainerStats.GetStatsAggregatedAsync(query.ContainerId, query.Hours, cancellationToken);
        return Result.Success(result);
    }
}

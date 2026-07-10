using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetPlatformStats(Guid Id, int Hours = 24) : IQuery<Result<IEnumerable<PlatformStat>>>
{
    internal sealed class Validator : AbstractValidator<GetPlatformStats>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Hours)
                .Must(hours => hours is 24 or 48 or 72)
                .WithMessage("Hours must be one of: 24, 48, 72.");
        }
    }
}

internal sealed class GetPlatformStatsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetPlatformStats, Result<IEnumerable<PlatformStat>>>
{
    public async ValueTask<Result<IEnumerable<PlatformStat>>> Handle(GetPlatformStats query, CancellationToken cancellationToken)
    {
        if (!await unitOfWork.Platforms.ExistsAsync(query.Id, cancellationToken))
        {
            return Result.Failure<IEnumerable<PlatformStat>>(new NotFoundError("Platform does not exist"));
        }

        var result = await unitOfWork.PlatformStats.GetStatsAggregatedAsync(query.Id, query.Hours, cancellationToken);
        return Result.Success(result);
    }
}

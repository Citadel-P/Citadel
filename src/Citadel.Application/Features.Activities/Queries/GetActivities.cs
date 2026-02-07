using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common.Models;
using LightResults;
using Mediator;

namespace Application.Features.Activities.Queries;

public sealed record GetActivities(
    Guid? ResourceId = null,
    ActivityResourceType? ResourceType = null,
    ActivityEventType? EventType = null,
    int Page = 1, 
    int PageSize = 50) : IQuery<Result<PagedResult<ActivityEvent>>>
{
    internal class Validator : AbstractValidator<GetActivities>
    {
        public Validator()
        {
            RuleFor(s => s.Page).GreaterThan(0);
            RuleFor(s => s.PageSize).GreaterThan(0).LessThanOrEqualTo(500);
        }
    }
}

internal sealed class GetActivitiesHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetActivities, Result<PagedResult<ActivityEvent>>>
{
    public async ValueTask<Result<PagedResult<ActivityEvent>>> Handle(GetActivities query, CancellationToken cancellationToken)
    {
        var activities = await unitOfWork.ActivityEventRepository.GetPagedAsync(
            resourceId: query.ResourceId,
            resourceType: query.ResourceType,
            eventType: query.EventType,
            page: query.Page,
            pageSize: query.PageSize,
            cancellationToken);
        return Result.Success(activities);
    }
}

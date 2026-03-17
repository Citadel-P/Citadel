using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.Models;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

public sealed record GetAlertEvents(
    Guid? ResourceId = null,
    AlertType? AlertType = null,
    AlertResourceType? ResourceType = null,
    bool? UnresolvedOnly = null,
    int Page = 1,
    int PageSize = 50) : IQuery<Result<PagedResult<AlertEvent>>>
{
    internal sealed class Validator : AbstractValidator<GetAlertEvents>
    {
        public Validator()
        {
            RuleFor(x => x.Page).GreaterThan(0);
            RuleFor(x => x.PageSize).GreaterThan(0).LessThanOrEqualTo(500);
        }
    }
}

internal sealed class GetAlertEventsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAlertEvents, Result<PagedResult<AlertEvent>>>
{
    public async ValueTask<Result<PagedResult<AlertEvent>>> Handle(GetAlertEvents query, CancellationToken cancellationToken)
    {
        var alertEvents = await unitOfWork.AlertEvents.GetPagedAsync(
            resourceId: query.ResourceId,
            alertType: query.AlertType,
            resourceType: query.ResourceType,
            page: query.Page,
            pageSize: query.PageSize,
            cancellationToken: cancellationToken,
            unresolvedOnly: query.UnresolvedOnly);

        return Result.Success(alertEvents);
    }
}

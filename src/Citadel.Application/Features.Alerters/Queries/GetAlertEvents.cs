using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.Extensions;
using Hosting.Common.Models;
using LightResults;
using Mediator;
using Hosting.Common;
using Microsoft.AspNetCore.Http;

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

internal sealed class GetAlertEventsHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<GetAlertEvents, Result<PagedResult<AlertEvent>>>
{
    public async ValueTask<Result<PagedResult<AlertEvent>>> Handle(GetAlertEvents query, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        var alertEvents = user is not null && !user.IsAdmin()
            ? await unitOfWork.AlertEvents.GetAuthorizedPagedAsync(
                userId: user.GetUserId(),
                permissionResourceType: Hosting.Common.ResourceType.Alert,
                action: ResourceAction.View,
                resourceId: query.ResourceId,
                alertType: query.AlertType,
                resourceType: query.ResourceType,
                page: query.Page,
                pageSize: query.PageSize,
                cancellationToken: cancellationToken,
                unresolvedOnly: query.UnresolvedOnly)
            : await unitOfWork.AlertEvents.GetPagedAsync(
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

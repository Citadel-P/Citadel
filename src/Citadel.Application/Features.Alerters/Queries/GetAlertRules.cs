using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.Models;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

public sealed record GetAlertRules(int Page = 1, int PageSize = 50) : IQuery<Result<PagedResult<AlertRule>>>
{
    internal class Validator : AbstractValidator<GetAlertRules>
    {
        public Validator()
        {
            RuleFor(s => s.Page).GreaterThan(0);
            RuleFor(s => s.PageSize).GreaterThan(0).LessThanOrEqualTo(500);
        }
    }
}

internal sealed class GetAlertRulesHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAlertRules, Result<PagedResult<AlertRule>>>
{
    public async ValueTask<Result<PagedResult<AlertRule>>> Handle(GetAlertRules query, CancellationToken cancellationToken)
    {
        var rules = await unitOfWork.AlertRules.GetPagedAsync(query.Page, query.PageSize, cancellationToken);
        return Result.Success(rules);
    }
}

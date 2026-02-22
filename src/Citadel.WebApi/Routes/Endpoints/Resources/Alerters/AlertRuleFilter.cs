using Application.Features.Alerters.Queries;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertRuleFilter(
    [FromQuery] int Page = 1,
    [FromQuery] int PageSize = 50)
{
    internal GetAlertRules ToQuery() => new(Page, PageSize);
}

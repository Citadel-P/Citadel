using Application.Features.Identity.Teams.Queries;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record TeamSearchFilter(
    [FromQuery] string Query = null,
    [FromQuery] int Limit = 20)
{
    internal SearchTeams ToQuery() => new(Query ?? string.Empty, Limit);

    public static ValueTask<TeamSearchFilter> BindAsync(HttpContext context)
    {
        var query = context.Request.Query;
        var value = query.TryGetValue("query", out var queryStr) ? queryStr.ToString() : null;
        var limit = query.TryGetValue("limit", out var limitStr) && int.TryParse(limitStr, out var parsedLimit)
            ? parsedLimit
            : 20;

        return ValueTask.FromResult(new TeamSearchFilter(value, limit));
    }
}
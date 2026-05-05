using Application.Features.Identity.Users.Queries;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record UserSearchFilter(
    [FromQuery] string Query = null,
    [FromQuery] int Limit = 20)
{
    internal SearchUsers ToQuery() => new(Query ?? string.Empty, Limit);

    public static ValueTask<UserSearchFilter> BindAsync(HttpContext context)
    {
        var query = context.Request.Query;
        var value = query.TryGetValue("query", out var queryStr) ? queryStr.ToString() : null;
        var limit = query.TryGetValue("limit", out var limitStr) && int.TryParse(limitStr, out var parsedLimit)
            ? parsedLimit
            : 20;

        return ValueTask.FromResult(new UserSearchFilter(value, limit));
    }
}
using Application.Features.Identity.Teams.Queries;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record TeamsFilter([FromQuery] int Page = 1, [FromQuery] int PageSize = 50)
{
    internal GetTeams ToQuery() => new(Page, PageSize);

    public static ValueTask<TeamsFilter> BindAsync(HttpContext context)
    {
        var query = context.Request.Query;
        var page = query.TryGetValue("page", out var pageStr) && int.TryParse(pageStr, out var parsedPage)
            ? parsedPage
            : 1;
        var pageSize = query.TryGetValue("pageSize", out var pageSizeStr) && int.TryParse(pageSizeStr, out var parsedPageSize)
            ? parsedPageSize
            : 50;

        return ValueTask.FromResult(new TeamsFilter(page, pageSize));
    }
}

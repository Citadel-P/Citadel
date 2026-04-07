using Application.Features.Identity.Users.Queries;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record UsersFilter([FromQuery] int Page = 1, [FromQuery] int PageSize = 50)
{
    internal GetUsers ToQuery() => new(Page, PageSize);

    public static ValueTask<UsersFilter> BindAsync(HttpContext context)
    {
        var query = context.Request.Query;
        var page = query.TryGetValue("page", out var pageStr) && int.TryParse(pageStr, out var parsedPage)
            ? parsedPage
            : 1;
        var pageSize = query.TryGetValue("pageSize", out var pageSizeStr) && int.TryParse(pageSizeStr, out var parsedPageSize)
            ? parsedPageSize
            : 50;

        return ValueTask.FromResult(new UsersFilter(page, pageSize));
    }
}

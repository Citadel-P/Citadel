using Domain.Contracts.Resources.Identity;
using Hosting.Common.Models;
using WebApi.Routes.Endpoints.Resources.Paging;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record TeamsView(PagedResultView<TeamView> PagedResult)
{
    internal static TeamsView Map(PagedResult<TeamDetails> pagedResult) => new(
        new PagedResultView<TeamView>(
            pagedResult.Items.Select(TeamView.Map),
            pagedResult.TotalCount,
            pagedResult.Page,
            pagedResult.PageSize));
}

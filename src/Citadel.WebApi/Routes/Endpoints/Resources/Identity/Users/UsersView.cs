using Domain.Contracts.Resources.Identity;
using Hosting.Common.Models;
using WebApi.Routes.Endpoints.Resources.Paging;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record UsersView(PagedResultView<UserView> PagedResult)
{
    internal static UsersView Map(PagedResult<UserDetails> pagedResult) => new(
        new PagedResultView<UserView>(
            pagedResult.Items.Select(UserView.Map),
            pagedResult.TotalCount,
            pagedResult.Page,
            pagedResult.PageSize));
}

using Application.Permissions;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;
using Hosting.Common.Models;
using WebApi.Routes.Endpoints.Resources.Paging;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record UsersView(PagedResultView<UserView> PagedResult, ResourceCapabilities Capabilities)
{
    internal static async Task<UsersView> Map(PagedResult<UserDetails> pagedResult, IPermissionEvaluator permissionEvaluator)
    {
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.User);
        if (!pagedResult.Items.Any())
            return new UsersView(new PagedResultView<UserView>([], 0, 0, 0), CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var items = pagedResult.Items.Select(UserView.Map);
        return new UsersView(
            new PagedResultView<UserView>(
                items,
                pagedResult.TotalCount,
                pagedResult.Page,
                pagedResult.PageSize),
            CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}

using Application.Permissions;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;
using Hosting.Common.Models;
using WebApi.Routes.Endpoints.Resources.Paging;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record TeamsView(PagedResultView<TeamView> PagedResult, ResourceCapabilities Capabilities)
{
    internal static async Task<TeamsView> Map(PagedResult<TeamDetails> pagedResult, IPermissionEvaluator permissionEvaluator)
    {
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.Team);
        if (!pagedResult.Items.Any())
            return new TeamsView(new PagedResultView<TeamView>([], 0, 0, 0), CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var items = pagedResult.Items.Select(TeamView.Map);
        return new TeamsView(
            new PagedResultView<TeamView>(
                items,
                pagedResult.TotalCount,
                pagedResult.Page,
                pagedResult.PageSize),
            CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}

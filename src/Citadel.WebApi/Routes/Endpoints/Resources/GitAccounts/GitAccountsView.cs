using Application.Permissions;
using Domain.Entities.Git;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.GitAccounts;

public sealed record GitAccountsView(IEnumerable<GitAccountView> GitAccounts, ResourceCapabilities Capabilities)
{
    internal static async Task<GitAccountsView> Map(IEnumerable<GitAccount> gitAccounts, IPermissionEvaluator permissionEvaluator)
    {
        var list = gitAccounts as GitAccount[] ?? [.. gitAccounts];
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.GitAccount);
        if (list.Length == 0)
            return new GitAccountsView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var ids = new Guid[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            ids[i] = list[i].Id;
        }

        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.GitAccount);

        var views = new GitAccountView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            var gitAccount = list[i];

            var baseView = GitAccountView.Map(gitAccount);

            perms.TryGetValue(gitAccount.Id, out var meta);

            views[i] = baseView with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(
                    meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new GitAccountsView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}

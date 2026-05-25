using Application.Permissions;
using Domain.Entities.Git;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record GitRepositoriesView(IEnumerable<GitRepositoryView> GitRepositories)
{
    internal static async Task<GitRepositoriesView> Map(IEnumerable<GitRepository> gitRepositories, IPermissionEvaluator permissionEvaluator)
    {
        var list = gitRepositories as GitRepository[] ?? [.. gitRepositories];

        if (list.Length == 0)
            return new GitRepositoriesView([]);

        var ids = new Guid[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            ids[i] = list[i].Id;
        }

        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.GitRepository);

        var views = new GitRepositoryView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            var gitRepository = list[i];

            var baseView = GitRepositoryView.Map(gitRepository);

            perms.TryGetValue(gitRepository.Id, out var meta);

            views[i] = baseView with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(
                    meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new GitRepositoriesView(views);
    }
}

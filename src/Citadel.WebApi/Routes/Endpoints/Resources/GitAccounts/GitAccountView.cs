using Application.Permissions;
using Domain;
using Domain.Entities.Git;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.GitAccounts;

public sealed record GitAccountView(
    Guid Id,
    Guid CreatedByActorId,
    string Name,
    string Domain,
    GitTransport Transport,
    GitAuthType AuthType,
    DateTime CreatedAt,
    ResourceCapabilities? Capabilities = null)
{
    internal static GitAccountView Map(GitAccount gitAccount) => new(
        gitAccount.Id,
        gitAccount.CreatedByActorId,
        gitAccount.Name,
        gitAccount.Domain,
        gitAccount.Transport,
        gitAccount.AuthType,
        gitAccount.CreatedAt);

    internal static async Task<GitAccountView> Map(GitAccount gitAccount, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(gitAccount.Id, ResourceType.GitAccount);
        return Map(gitAccount) with
        {
            Capabilities = CapabilityMapper.ToResourceCapabilities(permissions)
        };
    }
}

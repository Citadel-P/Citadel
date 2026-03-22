using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitAccounts;

public sealed record GitAccountView(
    Guid Id,
    Guid CreatedByActorId,
    string Name,
    string Domain,
    GitAuthType AuthType,
    DateTime CreatedAt)
{
    internal static GitAccountView Map(GitAccount gitAccount) => new(
        gitAccount.Id,
        gitAccount.CreatedByActorId,
        gitAccount.Name,
        gitAccount.Domain,
        gitAccount.AuthType,
        gitAccount.CreatedAt);
}

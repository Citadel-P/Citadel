using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitAccounts;

public sealed record GitAccountView(
    Guid Id,
    Guid CreatedByActorId,
    string Name,
    string Domain,
    GitTransport Transport,
    GitAuthType AuthType,
    DateTime CreatedAt)
{
    internal static GitAccountView Map(GitAccount gitAccount) => new(
        gitAccount.Id,
        gitAccount.CreatedByActorId,
        gitAccount.Name,
        gitAccount.Domain,
        gitAccount.Transport,
        gitAccount.AuthType,
        gitAccount.CreatedAt);
}

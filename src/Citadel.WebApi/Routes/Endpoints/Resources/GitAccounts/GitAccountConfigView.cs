using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitAccounts;

public sealed record GitAccountConfigView(
    Guid Id,
    string Name,
    string Domain,
    GitAuthType AuthType,
    GitAccountConfiguration? Configuration)
{
    internal static GitAccountConfigView Map(GitAccount gitAccount) => new(
        gitAccount.Id,
        gitAccount.Name,
        gitAccount.Domain,
        gitAccount.AuthType,
        gitAccount.Configuration);
}

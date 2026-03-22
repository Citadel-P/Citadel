using Application.Features.GitAccounts.Commands;
using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitAccounts;

public sealed record GitAccountInput(
    string Name,
    string Domain,
    GitAuthType AuthType,
    GitAccountConfiguration Configuration)
{
    internal CreateGitAccount ToCommand() => new(Name, Domain, AuthType, Configuration);
}

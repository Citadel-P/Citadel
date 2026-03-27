using Application.Features.GitAccounts.Commands;
using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitAccounts;

public sealed record GitAccountInput(
    string Name,
    string Domain,
    GitTransport Transport,
    GitAuthType AuthType,
    GitAuthConfiguration Configuration)
{
    internal CreateGitAccount ToCommand() => new(Name, Domain, Transport, AuthType, Configuration);
}

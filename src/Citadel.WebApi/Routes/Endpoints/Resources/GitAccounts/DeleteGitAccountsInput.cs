using Application.Features.GitAccounts.Commands;

namespace WebApi.Routes.Endpoints.Resources.GitAccounts;

public sealed record DeleteGitAccountsInput(IEnumerable<Guid> Ids)
{
    internal DeleteGitAccounts ToCommand() => new(Ids);
}

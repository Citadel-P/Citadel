using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitAccounts;

public sealed record GitAccountsView(IEnumerable<GitAccountView> GitAccounts)
{
    internal static GitAccountsView Map(IEnumerable<GitAccount> gitAccounts) => new([.. gitAccounts.Select(GitAccountView.Map)]);
}

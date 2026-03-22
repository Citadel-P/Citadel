using Application.Features.GitRepositories.Commands;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record DeleteGitRepositoriesInput(IEnumerable<Guid> Ids)
{
    internal DeleteGitRepositories ToCommand() => new(Ids);
}


using Application.Features.Deployments.Commands;

namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record DeleteDeploymentsInput(IEnumerable<Guid> Ids)
{
    internal DeleteDeployments ToCommand() => new(Ids);
}

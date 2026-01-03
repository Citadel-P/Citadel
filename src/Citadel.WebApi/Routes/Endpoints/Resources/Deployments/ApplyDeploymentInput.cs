using Application.Features.Deployments.Commands;

namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record ApplyDeploymentInput (Guid Id, bool? Recreate = false)
{
    internal ApplyDeployment ToCommand() => new(Id, Recreate);
}
using Application.Features.Deployments.Commands;

namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record ApplyDeploymentInput (Guid Id)
{
    internal ApplyDeployment ToCommand() => new(Id);
}
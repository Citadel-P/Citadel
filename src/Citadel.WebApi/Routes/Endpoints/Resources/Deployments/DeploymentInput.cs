using Application.Features.Deployments.Commands;
using Domain.Entities;

namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record DeploymentInput(
    string Name,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec
    )
{
    internal CreateDeployment ToCommand() => new(Name, PlatformId, Description, Spec);
}
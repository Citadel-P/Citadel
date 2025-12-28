using Application.Features.Deployments.Commands;
using Domain;
using Domain.Entities;

namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record DeploymentInput(
    string Name,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec,
    UpdateBehavior UpdateBehavior
    )
{
    internal CreateDeployment ToCommand() => new(Name, PlatformId, Description, UpdateBehavior, Spec);
}


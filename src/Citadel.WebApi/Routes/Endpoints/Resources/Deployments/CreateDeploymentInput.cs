using Application.Features.Deployments.Commands;
using Domain.Entities.Deployments;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record CreateDeploymentInput(
    string Name,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec,
    IReadOnlyCollection<Guid>? TagIds = null,
    DuplicateSourceInput? DuplicateSource = null)
{
    internal CreateDeployment ToCommand() => new(Name, PlatformId, Description, Spec, TagIds, DuplicateSource?.ToActivitySourceResource());
}

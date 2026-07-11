using Application.Features.Deployments.Queries;
using Domain;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record DeploymentDuplicateDraftView(
    CreateDeploymentInput Draft,
    IReadOnlyCollection<DuplicateDraftWarningView> Warnings)
{
    internal static DeploymentDuplicateDraftView Map(DeploymentDuplicateDraft draft, Guid sourceId) => new(
        Draft: new CreateDeploymentInput(
            Name: draft.Name,
            PlatformId: draft.PlatformId,
            Description: draft.Description,
            Spec: draft.Spec,
            TagIds: draft.TagIds,
            DuplicateSource: new DuplicateSourceInput(ActivityResourceType.Deployment, sourceId, draft.SourceName)),
        Warnings: [.. draft.Warnings.Select(DuplicateDraftWarningView.Map)]);
}

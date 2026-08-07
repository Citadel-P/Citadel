using Application.Features.SwarmServices.Queries;
using Domain;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Routes.Endpoints.Resources.SwarmServices;

public sealed record SwarmServiceDuplicateDraftView(
    CreateSwarmServiceInput Draft,
    IReadOnlyCollection<DuplicateDraftWarningView> Warnings)
{
    internal static SwarmServiceDuplicateDraftView Map(SwarmServiceDuplicateDraft draft, Guid sourceId) => new(
        Draft: new CreateSwarmServiceInput(
            Name: draft.Name,
            PlatformId: draft.PlatformId,
            Description: draft.Description,
            Spec: draft.Spec,
            TagIds: draft.TagIds,
            DuplicateSource: new DuplicateSourceInput(
                ActivityResourceType.SwarmService,
                sourceId,
                draft.SourceName)),
        Warnings: [.. draft.Warnings.Select(DuplicateDraftWarningView.Map)]);
}

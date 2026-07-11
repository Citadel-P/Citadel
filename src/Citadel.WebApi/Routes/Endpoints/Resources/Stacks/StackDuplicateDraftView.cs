using Application.Features.Stacks.Queries;
using Domain;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record StackDuplicateDraftView(
    CreateStackInput Draft,
    IReadOnlyCollection<DuplicateDraftWarningView> Warnings)
{
    internal static StackDuplicateDraftView Map(StackDuplicateDraft draft, Guid sourceId) => new(
        Draft: new CreateStackInput(
            Name: draft.Name,
            PlatformId: draft.PlatformId,
            Description: draft.Description,
            StackSource: draft.StackSource,
            Spec: draft.Spec,
            DriftPolicy: draft.DriftPolicy,
            TagIds: draft.TagIds,
            DuplicateSource: new DuplicateSourceInput(ActivityResourceType.Stack, sourceId, draft.SourceName)),
        Warnings: [.. draft.Warnings.Select(DuplicateDraftWarningView.Map)]);
}

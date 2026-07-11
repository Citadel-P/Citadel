using Domain;
using Domain.Entities.Activities;

namespace WebApi.Routes.Endpoints.Resources;

public sealed record DuplicateSourceInput(
    ActivityResourceType ResourceType,
    Guid ResourceId,
    string ResourceName)
{
    internal ActivitySourceResource ToActivitySourceResource() => new(ResourceType, ResourceId, ResourceName);
}

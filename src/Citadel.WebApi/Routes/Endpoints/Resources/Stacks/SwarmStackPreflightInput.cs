using Application.Features.Stacks.Queries;
using Domain;
using Domain.Entities.Stacks;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record SwarmStackPreflightInput(
    string Name,
    Guid PlatformId,
    StackSource StackSource,
    StackSpec Spec,
    StackDriftPolicy? DriftPolicy = null)
{
    internal PreflightSwarmStack ToQuery()
        => new(Name, PlatformId, StackSource, Spec, DriftPolicy);
}

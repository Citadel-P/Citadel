using Domain.Entities.Stacks;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record PatchStackInput(
    Guid PlatformId,
    StackSpec Spec);

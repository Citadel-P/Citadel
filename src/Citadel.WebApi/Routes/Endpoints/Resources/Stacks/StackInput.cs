using Application.Features.Stacks.Commands;
using Domain;
using Domain.Entities.Stacks;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record StackInput(
    string Name,
    Guid PlatformId,
    string? Description,
    StackSource StackSource,
    StackSpec Spec)
{
    internal CreateStack ToCommand() => new(Name, PlatformId, Description, StackSource, Spec);
}
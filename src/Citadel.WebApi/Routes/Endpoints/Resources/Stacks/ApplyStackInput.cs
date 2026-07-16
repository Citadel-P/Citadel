using Application.Features.Stacks.Commands;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record ApplyStackInput(Guid Id, bool? Recreate = false)
{
    internal ApplyStack ToCommand() => new(Id, Recreate);
}

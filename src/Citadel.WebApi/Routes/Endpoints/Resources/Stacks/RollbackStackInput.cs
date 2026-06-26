using Application.Features.Stacks.Commands;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record RollbackStackInput(Guid StackId, Guid ReleaseId)
{
    internal RollbackStack ToCommand() => new(StackId, ReleaseId);
}

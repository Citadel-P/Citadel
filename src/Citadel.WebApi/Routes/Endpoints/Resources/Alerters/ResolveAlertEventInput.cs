using Application.Features.Alerters.Commands;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record ResolveAlertEventsInput(IReadOnlyCollection<Guid> Ids, string? ResolutionNote)
{
    internal ResolveAlertEvents ToCommand()
        => new(Ids, ResolutionNote);
}

using Application.Features.Alerters.Commands;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record ResolveAlertEventsInput(IEnumerable<Guid> Ids, string? ResolutionNote)
{
    internal ResolveAlertEvents ToCommand()
        => new(Ids, ResolutionNote);
}

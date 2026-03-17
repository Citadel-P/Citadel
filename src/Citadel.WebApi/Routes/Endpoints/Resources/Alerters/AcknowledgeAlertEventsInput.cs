using Application.Features.Alerters.Commands;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AcknowledgeAlertEventsInput(IReadOnlyCollection<Guid> Ids)
{
    internal AcknowledgeAlertEvents ToCommand()
        => new(Ids);
}

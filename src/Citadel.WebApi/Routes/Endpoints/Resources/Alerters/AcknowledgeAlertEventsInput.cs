using Application.Features.Alerters.Commands;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AcknowledgeAlertEventsInput(IEnumerable<Guid> Ids)
{
    internal AcknowledgeAlertEvents ToCommand()
        => new(Ids);
}

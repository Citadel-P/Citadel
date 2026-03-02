using Application.Features.Alerters.Commands;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record DeleteAlertChannelsInput(IEnumerable<Guid> Ids)
{
    internal DeleteAlertChannels ToCommand() => new(Ids);
}

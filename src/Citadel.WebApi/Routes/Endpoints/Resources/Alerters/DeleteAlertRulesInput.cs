using Application.Features.Alerters.Commands;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record DeleteAlertRulesInput(IEnumerable<Guid> Ids)
{
    internal DeleteAlertRules ToCommand() => new(Ids);
}

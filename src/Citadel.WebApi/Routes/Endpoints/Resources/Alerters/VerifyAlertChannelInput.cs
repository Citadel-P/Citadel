using Application.Features.Alerters.Commands;
using Domain;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record VerifyAlertChannelInput(
    AlertDestination AlertDestination,
    string Name,
    string Url)
{
    internal VerifyAlertChannel ToCommand() => new(AlertDestination, Name, Url);
}

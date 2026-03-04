using Application.Features.Alerters.Commands;
using Domain;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record VerifyAlertChannelInput(
    AlertDestination AlertDestination,
    string Url)
{
    internal VerifyAlertChannel ToCommand() => new(AlertDestination, Url);
}

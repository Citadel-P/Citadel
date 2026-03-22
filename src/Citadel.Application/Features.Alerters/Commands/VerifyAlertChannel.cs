using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

public sealed record VerifyAlertChannel(
    AlertDestination AlertDestination,
    string Name,
    string Url) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<VerifyAlertChannel>
    {
        public Validator()
        {
            RuleFor(x => x.Url).NotEmpty();
            RuleFor(x => x.Name).NotEmpty();
        }
    }
}

internal sealed class VerifyAlertChannelHandler(IShoutrrrCliRepository notificationRepository) : ICommandHandler<VerifyAlertChannel, Result>
{
    public async ValueTask<Result> Handle(VerifyAlertChannel command, CancellationToken cancellationToken)
    {
        var channel = new AlertChannel(
            name: command.Name,
            alertDestination: command.AlertDestination,
            url: command.Url,
            isActive: true,
            createdByActorId: Constants.SystemId);

        return await notificationRepository.SendTestNotificationAsync(channel, cancellationToken);
    }
}

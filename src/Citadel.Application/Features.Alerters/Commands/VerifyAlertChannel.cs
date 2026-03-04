using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

public sealed record VerifyAlertChannel(
    AlertDestination AlertDestination,
    string Url) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<VerifyAlertChannel>
    {
        public Validator()
        {
            RuleFor(x => x.Url).NotEmpty();
        }
    }
}

internal sealed class VerifyAlertChannelHandler(INotificationRepository notificationRepository) : ICommandHandler<VerifyAlertChannel, Result>
{
    public async ValueTask<Result> Handle(VerifyAlertChannel command, CancellationToken cancellationToken)
    {
        var channel = new AlertChannel(
            name: "Verification",
            alertDestination: command.AlertDestination,
            url: command.Url,
            isActive: true,
            createdByActorId: Constants.SystemId);

        var verifyResult = await notificationRepository.SendTestNotificationAsync(channel, cancellationToken);
        if (!verifyResult.IsSuccess)
        {
            return Result.Failure(new BadRequestError(verifyResult.ErrorMessage ?? "Channel verification failed."));
        }

        return Result.Success();
    }
}

using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

[RequirePermission(ResourceType.AlertChannel, PermissionLevel.Write)]
public sealed record CreateAlertChannel(
    string Name,
    AlertDestination AlertDestination,
    string Url,
    bool IsActive) : ICommand<Result<AlertChannel>>
{
    internal sealed class Validator : AbstractValidator<CreateAlertChannel>
    {
        public Validator()
        {
            RuleFor(x => x.Url).NotEmpty();
        }
    }
}

internal sealed class CreateAlertChannelHandler(
    IUnitOfWork unitOfWork,
    AlertRuleCache alertRuleCache,
    IUserContextAccessor userContext)
    : ICommandHandler<CreateAlertChannel, Result<AlertChannel>>
{
    public async ValueTask<Result<AlertChannel>> Handle(CreateAlertChannel command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var channel = new AlertChannel(command.Name, command.AlertDestination, command.Url, command.IsActive, actorId);
        await unitOfWork.AlertRules.AddChannelAsync(channel, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        alertRuleCache.UpsertChannel(channel);

        return channel;
    }
}

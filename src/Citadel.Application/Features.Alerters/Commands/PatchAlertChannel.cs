using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using FluentValidation;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Hosting.Common;

namespace Application.Features.Alerters.Commands;

[RequirePermission(ResourceType.AlertChannel, ResourceAction.Update)]
public sealed record PatchAlertChannel(Guid Id, JsonMergePatchDocument<AlertChannel> Patch) : ICommand<Result<AlertChannel>>
{
    internal sealed class Validator : AbstractValidator<PatchAlertChannel>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Patch).NotNull();
        }
    }
}

internal sealed class PatchAlertChannelHandler(
    IUnitOfWork unitOfWork,
    AlertRuleCache alertRuleCache) : ICommandHandler<PatchAlertChannel, Result<AlertChannel>>
{
    public async ValueTask<Result<AlertChannel>> Handle(PatchAlertChannel command, CancellationToken cancellationToken)
    {
        var channel = await unitOfWork.AlertRules.GetChannelByIdAsync(command.Id, cancellationToken);
        if (channel is null)
        {
            return Result.Failure<AlertChannel>(new NotFoundError("The provided alert channel does not exist"));
        }

        AlertChannel patchedChannel;
        try
        {
            patchedChannel = command.Patch.ApplyTo(channel, AlertRuleJsonContext.Default.AlertChannel);
        }
        catch (Exception ex) when (ex is ArgumentException or InvalidOperationException)
        {
            return Result.Failure<AlertChannel>(new BadRequestError(ex.Message));
        }

        channel.PartialUpdate(
            name: patchedChannel.Name,
            alertDestination: patchedChannel.AlertDestination,
            url: patchedChannel.Url,
            isActive: patchedChannel.IsActive);

        await unitOfWork.AlertRules.UpdateChannelAsync(channel, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        alertRuleCache.UpsertChannel(channel);

        return channel;
    }
}

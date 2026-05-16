using Application.Services;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record SendContainerExecInput(string ContainerId, string SessionId, byte[] Data) : ICommand<Result>;

internal sealed class SendContainerExecInputHandler(
    IExecSessionManager execSessionManager,
    IContainerAuthorizationService containerAuthorizationService)
    : ICommandHandler<SendContainerExecInput, Result>
{
    public async ValueTask<Result> Handle(SendContainerExecInput command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync([command.ContainerId], ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Terminal, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing specific permission [Terminal] on [Platform]"));
        }

        await execSessionManager.SendInputAsync(command.ContainerId, command.SessionId, command.Data, cancellationToken);
        return Result.Success();
    }
}
using Application.Services;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record ResizeContainerExecSession(string ContainerId, string SessionId, int Cols, int Rows) : ICommand<Result>;

internal sealed class ResizeContainerExecSessionHandler(
    IExecSessionManager execSessionManager,
    IContainerAuthorizationService containerAuthorizationService): ICommandHandler<ResizeContainerExecSession, Result>
{
    public async ValueTask<Result> Handle(ResizeContainerExecSession command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync([command.ContainerId], ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Terminal, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing specific permission [Terminal] on [Platform]"));
        }

        await execSessionManager.ResizeAsync(command.ContainerId, command.SessionId, command.Cols, command.Rows, cancellationToken);
        return Result.Success();
    }
}
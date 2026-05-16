using Application.Services;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record StartContainerShellSession(string ContainerId, string SessionId, string Shell) : ICommand<Result>;

internal sealed class StartContainerShellSessionHandler(
    IExecSessionManager execSessionManager,
    IContainerAuthorizationService containerAuthorizationService): ICommandHandler<StartContainerShellSession, Result>
{
    public async ValueTask<Result> Handle(StartContainerShellSession command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync([command.ContainerId], ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Terminal, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing specific permission [Terminal] on [Platform]"));
        }

        await execSessionManager.StartExecProcess(command.ContainerId, command.SessionId, command.Shell, cancellationToken);
        return Result.Success();
    }
}
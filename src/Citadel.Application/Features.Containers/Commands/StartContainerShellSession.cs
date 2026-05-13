using Application.Services;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record StartContainerShellSession(string GroupId, string Shell) : ICommand<Result>;


internal sealed class StartContainerShellSessionHandler(
    IExecSessionManager execSessionManager,
    IContainerAuthorizationService containerAuthorizationService)
    : ICommandHandler<StartContainerShellSession, Result>
{
    public async ValueTask<Result> Handle(StartContainerShellSession command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasTerminalAccessAsync(ResourceType.Platform, command.GroupId, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing specific permission [Terminal] on [Platform]"));
        }

        await execSessionManager.StartExecProcess(command.GroupId, command.Shell, cancellationToken);
        return Result.Success();
    }
}
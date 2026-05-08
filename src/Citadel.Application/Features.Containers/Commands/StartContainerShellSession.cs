using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute, SpecificPermission.Terminal)]
public sealed record StartContainerShellSession(string GroupId, string Shell) : ICommand<Result>;


internal sealed class StartContainerShellSessionHandler(IExecSessionManager execSessionManager)
    : ICommandHandler<StartContainerShellSession, Result>
{
    public async ValueTask<Result> Handle(StartContainerShellSession command, CancellationToken cancellationToken)
    {
        await execSessionManager.StartExecProcess(command.GroupId, command.Shell, cancellationToken);
        return Result.Success();
    }
}
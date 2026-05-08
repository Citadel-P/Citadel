using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute, SpecificPermission.Terminal)]
public sealed record ResizeContainerExecSession(string GroupId, int Cols, int Rows) : ICommand<Result>;

internal sealed class ResizeContainerExecSessionHandler(IExecSessionManager execSessionManager)
    : ICommandHandler<ResizeContainerExecSession, Result>
{
    public async ValueTask<Result> Handle(ResizeContainerExecSession command, CancellationToken cancellationToken)
    {
        await execSessionManager.ResizeAsync(command.GroupId, command.Cols, command.Rows, cancellationToken);
        return Result.Success();
    }
}
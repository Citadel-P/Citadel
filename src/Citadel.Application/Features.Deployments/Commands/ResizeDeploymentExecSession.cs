using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Execute, SpecificPermission.Terminal)]
public sealed record ResizeDeploymentExecSession(string GroupId, int Cols, int Rows) : ICommand<Result>;

internal sealed class ResizeDeploymentExecSessionHandler(IExecSessionManager execSessionManager)
    : ICommandHandler<ResizeDeploymentExecSession, Result>
{
    public async ValueTask<Result> Handle(ResizeDeploymentExecSession command, CancellationToken cancellationToken)
    {
        await execSessionManager.ResizeAsync(command.GroupId, command.Cols, command.Rows, cancellationToken);
        return Result.Success();
    }
}
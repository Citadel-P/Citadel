using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Execute, SpecificPermission.Terminal)]
public sealed record StartDeploymentShellSession(string GroupId, string Shell) : ICommand<Result>;


internal sealed class StartDeploymentShellSessionHandler(IExecSessionManager execSessionManager)
    : ICommandHandler<StartDeploymentShellSession, Result>
{
    public async ValueTask<Result> Handle(StartDeploymentShellSession command, CancellationToken cancellationToken)
    {
        await execSessionManager.StartExecProcess(command.GroupId, command.Shell, cancellationToken);
        return Result.Success();
    }
}

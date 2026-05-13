using Application.Services;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

public sealed record StartDeploymentShellSession(string GroupId, string Shell) : ICommand<Result>;

internal sealed class StartDeploymentShellSessionHandler(
    IExecSessionManager execSessionManager,
    IContainerAuthorizationService containerAuthorizationService)
    : ICommandHandler<StartDeploymentShellSession, Result>
{
    public async ValueTask<Result> Handle(StartDeploymentShellSession command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasTerminalAccessAsync(ResourceType.Deployment, command.GroupId, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing specific permission [Terminal] on [Deployment]"));
        }

        await execSessionManager.StartExecProcess(command.GroupId, command.Shell, cancellationToken);
        return Result.Success();
    }
}

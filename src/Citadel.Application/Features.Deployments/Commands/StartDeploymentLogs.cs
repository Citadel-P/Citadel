using Application.Services;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

public sealed record StartDeploymentLogs(string ContainerId) : ICommand<Result>;

internal sealed class StartContainerLogsHandler(
    IContainerLogStreamManager containerLogStreamManager,
    IContainerAuthorizationService containerAuthorizationService)
    : ICommandHandler<StartDeploymentLogs, Result>
{
    public async ValueTask<Result> Handle(StartDeploymentLogs command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync([command.ContainerId], ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Logs, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing specific permission [Logs] on [Deployment]"));
        }

        containerLogStreamManager.StartContainerLogs(command.ContainerId);
        return Result.Success();
    }
}

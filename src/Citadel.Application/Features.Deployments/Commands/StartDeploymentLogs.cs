using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Logs)]
public sealed record StartDeploymentLogs(string ContainerId) : ICommand<Result>;

internal sealed class StartContainerLogsHandler(IContainerLogStreamManager containerLogStreamManager)
    : ICommandHandler<StartDeploymentLogs, Result>
{
    public ValueTask<Result> Handle(StartDeploymentLogs command, CancellationToken cancellationToken)
    {
        containerLogStreamManager.StartContainerLogs(command.ContainerId);
        return ValueTask.FromResult(Result.Success());
    }
}

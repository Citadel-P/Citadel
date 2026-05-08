using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Logs)]
public sealed record StartContainerLogs(string ContainerId): ICommand<Result>;

internal sealed class StartContainerLogsHandler(IContainerLogStreamManager containerLogStreamManager)
    : ICommandHandler<StartContainerLogs, Result>
{
    public ValueTask<Result> Handle(StartContainerLogs command, CancellationToken cancellationToken)
    {
        containerLogStreamManager.StartContainerLogs(command.ContainerId);
        return ValueTask.FromResult(Result.Success());
    }
}
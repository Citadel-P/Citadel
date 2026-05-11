using Application.Services;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record StartContainerLogs(string ContainerId): ICommand<Result>;

internal sealed class StartContainerLogsHandler(
    IContainerLogStreamManager containerLogStreamManager,
    IContainerPlatformAuthorizationService containerPlatformAuthorizationService)
    : ICommandHandler<StartContainerLogs, Result>
{
    public async ValueTask<Result> Handle(StartContainerLogs command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerPlatformAuthorizationService.HasAccessAsync([command.ContainerId], PermissionLevel.Read, SpecificPermission.Logs, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing permission [Read] on [Platform]"));
        }

        containerLogStreamManager.StartContainerLogs(command.ContainerId);
        return Result.Success();
    }
}
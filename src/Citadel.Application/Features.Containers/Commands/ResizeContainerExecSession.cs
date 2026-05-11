using Application.Services;
using Application.Services.SignalR;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record ResizeContainerExecSession(string GroupId, int Cols, int Rows) : ICommand<Result>;

internal sealed class ResizeContainerExecSessionHandler(
    IExecSessionManager execSessionManager,
    IContainerPlatformAuthorizationService containerPlatformAuthorizationService)
    : ICommandHandler<ResizeContainerExecSession, Result>
{
    public async ValueTask<Result> Handle(ResizeContainerExecSession command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerPlatformAuthorizationService.HasTerminalAccessAsync(command.GroupId, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing permission [Execute] on [Platform]"));
        }

        await execSessionManager.ResizeAsync(command.GroupId, command.Cols, command.Rows, cancellationToken);
        return Result.Success();
    }
}
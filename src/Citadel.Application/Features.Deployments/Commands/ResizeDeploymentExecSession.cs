using Application.Services;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

public sealed record ResizeDeploymentExecSession(string GroupId, int Cols, int Rows) : ICommand<Result>;

internal sealed class ResizeDeploymentExecSessionHandler(
    IExecSessionManager execSessionManager,
    IContainerAuthorizationService containerAuthorizationService): ICommandHandler<ResizeDeploymentExecSession, Result>
{
    public async ValueTask<Result> Handle(ResizeDeploymentExecSession command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasTerminalAccessAsync(ResourceType.Deployment, command.GroupId, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing specific permission [Terminal] on [Deployment]"));
        }

        await execSessionManager.ResizeAsync(command.GroupId, command.Cols, command.Rows, cancellationToken);
        return Result.Success();
    }
}
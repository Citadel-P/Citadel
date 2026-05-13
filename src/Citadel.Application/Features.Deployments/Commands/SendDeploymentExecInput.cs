using Application.Services;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

public sealed record SendDeploymentExecInput(string GroupId, byte[] Data) : ICommand<Result>;

internal sealed class SendDeploymentExecInputHandler(
    IExecSessionManager execSessionManager,
    IContainerAuthorizationService containerAuthorizationService)
    : ICommandHandler<SendDeploymentExecInput, Result>
{
    public async ValueTask<Result> Handle(SendDeploymentExecInput command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasTerminalAccessAsync(ResourceType.Deployment, command.GroupId, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing specific permission [Terminal] on [Deployment]"));
        }

        await execSessionManager.SendInputAsync(command.GroupId, command.Data, cancellationToken);
        return Result.Success();
    }
}

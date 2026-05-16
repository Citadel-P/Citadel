using Application.Services;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Terminal)]
public sealed record StartDeploymentShellSession(Guid Id, string SessionId, string Shell) : ICommand<Result>;

internal sealed class StartDeploymentShellSessionHandler(
     IUnitOfWork unitOfWork,
     IExecSessionManager execSessionManager): ICommandHandler<StartDeploymentShellSession, Result>
{
    public async ValueTask<Result> Handle(StartDeploymentShellSession command, CancellationToken cancellationToken)
    {
        var containerId = await unitOfWork.Deployments.GetContainerIdAsync(command.Id, cancellationToken);
        if (containerId is null)
        {
            return Result.Failure(new NotFoundError("Container does not exist"));
        }

        await execSessionManager.StartExecProcess(containerId, command.SessionId, command.Shell, cancellationToken);
        return Result.Success();
    }
}

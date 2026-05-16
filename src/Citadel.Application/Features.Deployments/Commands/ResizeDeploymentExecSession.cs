using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Terminal)]
public sealed record ResizeDeploymentExecSession(Guid Id, string SessionId, int Cols, int Rows) : ICommand<Result>;

internal sealed class ResizeDeploymentExecSessionHandler(
    IUnitOfWork unitOfWork,
    IExecSessionManager execSessionManager): ICommandHandler<ResizeDeploymentExecSession, Result>
{
    public async ValueTask<Result> Handle(ResizeDeploymentExecSession command, CancellationToken cancellationToken)
    {
        var containerId = await unitOfWork.Deployments.GetContainerIdAsync(command.Id, cancellationToken);
        if (containerId is null)
        {
            return Result.Failure(new NotFoundError("Container does not exist"));
        }

        await execSessionManager.ResizeAsync(containerId, command.SessionId, command.Cols, command.Rows, cancellationToken);
        return Result.Success();
    }
}
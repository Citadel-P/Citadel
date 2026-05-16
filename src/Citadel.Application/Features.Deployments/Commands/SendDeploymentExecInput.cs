using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Terminal)]
public sealed record SendDeploymentExecInput(Guid Id, string SessionId, byte[] Data) : ICommand<Result>;

internal sealed class SendDeploymentExecInputHandler(
    IUnitOfWork unitOfWork,
    IExecSessionManager execSessionManager)
    : ICommandHandler<SendDeploymentExecInput, Result>
{
    public async ValueTask<Result> Handle(SendDeploymentExecInput command, CancellationToken cancellationToken)
    {
        var containerId = await unitOfWork.Deployments.GetContainerIdAsync(command.Id, cancellationToken);
        if (containerId is null)
        {
            return Result.Failure(new NotFoundError("Container does not exist"));
        }

        await execSessionManager.SendInputAsync(containerId, command.SessionId, command.Data, cancellationToken);
        return Result.Success();
    }
}

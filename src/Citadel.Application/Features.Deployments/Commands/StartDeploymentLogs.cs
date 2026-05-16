using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Logs)]
public sealed record StartDeploymentLogs(Guid Id) : ICommand<Result>;

internal sealed class StartDeploymentLogsHandler(
    IUnitOfWork unitOfWork,
    IContainerLogStreamManager containerLogStreamManager): ICommandHandler<StartDeploymentLogs, Result>
{
    public async ValueTask<Result> Handle(StartDeploymentLogs command, CancellationToken cancellationToken)
    {
        var containerId = await unitOfWork.Deployments.GetContainerIdAsync(command.Id, cancellationToken);
        if (containerId is null)
        {
            return Result.Failure(new NotFoundError("Container does not exist"));
        }

        containerLogStreamManager.StartContainerLogs(containerId);
        return Result.Success();
    }
}

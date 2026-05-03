using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, ResourceAction.Exec)]
public sealed record SendDeploymentExecInput(string GroupId, byte[] Data) : ICommand<Result>;

internal sealed class SendDeploymentExecInputHandler(IExecSessionManager execSessionManager)
    : ICommandHandler<SendDeploymentExecInput, Result>
{
    public async ValueTask<Result> Handle(SendDeploymentExecInput command, CancellationToken cancellationToken)
    {
        await execSessionManager.SendInputAsync(command.GroupId, command.Data, cancellationToken);
        return Result.Success();
    }
}

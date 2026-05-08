using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute, SpecificPermission.Terminal)]
public sealed record SendContainerExecInput(string GroupId, byte[] Data) : ICommand<Result>;

internal sealed class SendContainerExecInputHandler(IExecSessionManager execSessionManager)
    : ICommandHandler<SendContainerExecInput, Result>
{
    public async ValueTask<Result> Handle(SendContainerExecInput command, CancellationToken cancellationToken)
    {
        await execSessionManager.SendInputAsync(command.GroupId, command.Data, cancellationToken);
        return Result.Success();
    }
}
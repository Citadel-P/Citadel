using Application.Services;
using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record SendContainerExecInput(string GroupId, byte[] Data) : ICommand<Result>;

internal sealed class SendContainerExecInputHandler(
    IExecSessionManager execSessionManager,
    IContainerPlatformAuthorizationService containerPlatformAuthorizationService)
    : ICommandHandler<SendContainerExecInput, Result>
{
    public async ValueTask<Result> Handle(SendContainerExecInput command, CancellationToken cancellationToken)
    {
        var hasAccess = await containerPlatformAuthorizationService.HasTerminalAccessAsync(command.GroupId, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing permission [Execute] on [Platform]"));
        }

        await execSessionManager.SendInputAsync(command.GroupId, command.Data, cancellationToken);
        return Result.Success();
    }
}
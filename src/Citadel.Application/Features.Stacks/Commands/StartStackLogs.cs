using Application.Services.SignalR;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Logs)]
public sealed record StartStackLogs(Guid Id) : ICommand<Result>;

internal sealed class StartStackLogsHandler(IStackLogStreamManager stackLogStreamManager) : ICommandHandler<StartStackLogs, Result>
{
    public ValueTask<Result> Handle(StartStackLogs command, CancellationToken cancellationToken)
    {
        stackLogStreamManager.StartStackLogs(command.Id);
        return ValueTask.FromResult(Result.Success());
    }
}

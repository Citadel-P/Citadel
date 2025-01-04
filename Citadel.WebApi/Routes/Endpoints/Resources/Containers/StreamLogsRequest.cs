using Application.Features.Platforms.Commands;

namespace WebApi.Routes.Endpoints.Resources.Containers;

/// <summary>
/// Request params for starting or stopping streaming logs
/// </summary>
/// <param name="ContainerId">containers id</param>
/// <param name="RequestId">Unique identifier of the request</param>
/// <param name="RequestedLogAction">Action to execute</param>
public sealed record StreamLogsRequest(string ContainerId, Guid RequestId, RequestedLogAction RequestedLogAction = RequestedLogAction.START)
{
    internal StreamContainerLogs ToCommand() => new(ContainerId, RequestId, RequestedLogAction);
}

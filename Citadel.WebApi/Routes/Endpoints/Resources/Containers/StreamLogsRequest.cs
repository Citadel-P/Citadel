using Application.Features.Platforms.Commands;

namespace WebApi.Routes.Endpoints.Resources.Containers;

/// <summary>
/// Request params for starting or stopping streaming logs
/// </summary>
/// <param name="ContainerId">containers id</param>
public sealed record StreamLogsRequest(string ContainerId)
{
    internal StreamContainerLogs ToCommand() => new(ContainerId);
}

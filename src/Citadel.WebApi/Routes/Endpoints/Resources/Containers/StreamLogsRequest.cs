using Application.Features.Containers.Commands;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record StreamLogsRequest(string ContainerId)
{
    internal StreamContainerLogs ToCommand() => new(ContainerId);
}

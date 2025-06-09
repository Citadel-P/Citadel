using Citadel.Agent.Containers.V1;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainerLogView(string Log);

internal static class MapperExtensions
{
    public static ContainerLogView Map(this ContainerLogReply log) => new(log.Log);
}

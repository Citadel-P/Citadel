namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainerLogView(string ContainerId, string RequestId, IEnumerable<string> Messages);

namespace WebApi.Controllers.V1.Resources.Containers;

public sealed record ContainerLogView(string ContainerId, string RequestId, IEnumerable<string> Messages);

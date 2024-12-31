namespace Application.Features.Containers.Models;

public sealed record ContainerLogRequest(string ContainerId, string RequestId, List<string> Messages, string Id);
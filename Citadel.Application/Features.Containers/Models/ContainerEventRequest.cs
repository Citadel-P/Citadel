namespace Application.Features.Containers.Models;

public sealed record ContainerEventRequest(string Id, string Action, string ContainerId, ContainerInfoRequest ContainerInfo);

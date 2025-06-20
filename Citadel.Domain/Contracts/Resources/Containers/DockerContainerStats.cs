namespace Domain.Contracts.Resources.Containers;

public sealed record DockerContainerStats(IReadOnlyDictionary<string, DockerContainerStat> Containers);

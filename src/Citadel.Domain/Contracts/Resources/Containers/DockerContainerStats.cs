namespace Domain.Contracts.Resources.Containers;

public sealed class DockerContainerStats
{
    private readonly Dictionary<string, DockerContainerStat> _containers = new(capacity: 64);

    public IReadOnlyDictionary<string, DockerContainerStat> Containers => _containers;

    public void Reset() => _containers.Clear();

    public void Add(string key, DockerContainerStat stat) => _containers[key] = stat;
}
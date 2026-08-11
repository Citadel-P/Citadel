namespace Domain.Contracts.Resources.Images;

public record ImageResult(
        string Id,
        double Size,
        int Containers,
        string ParentId,
        long SharedSize,
        long Created,
        double VirtualSize,
        IReadOnlyList<string>? RepoTags,
        IReadOnlyList<string>? RepoDigests,
        IReadOnlyDictionary<string, string>? Labels
    )
{
    public string? DockerNodeId { get; set; }
    public string? NodeHostname { get; set; }
    public bool IsStale { get; set; }
    public string? StaleReason { get; set; }
}



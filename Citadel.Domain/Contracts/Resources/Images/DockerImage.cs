namespace Domain.Contracts.Resources.Images;

public record DockerImage(
        string Id,
        double Size,
        int Containers,
        string ParentId,
        long SharedSize,
        DateTime Created,
        double VirtualSize,
        IReadOnlyList<string> RepoTags,
        IReadOnlyList<string> RepoDigests,
        IReadOnlyDictionary<string, string> Labels
    );



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
    );



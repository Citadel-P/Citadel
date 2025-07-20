namespace Domain.Contracts.Resources.Images;

public record DeleteImageCommand(
    IReadOnlyList<string> Ids,
    string PlatformAddress,
    bool Force,
    bool NoPrune
);


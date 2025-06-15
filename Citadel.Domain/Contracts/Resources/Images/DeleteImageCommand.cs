namespace Domain.Contracts.Resources.Images;

public record DeleteImageCommand(
    IReadOnlyList<string> Ids,
    bool Force,
    bool NoPrune
);


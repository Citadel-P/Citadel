namespace Domain.Contracts.Resources.Images;

public record DeleteImageResult(IReadOnlyList<DeleteImageResponseItem> Items);

public record DeleteImageResponseItem(IReadOnlyDictionary<string, string> Result);


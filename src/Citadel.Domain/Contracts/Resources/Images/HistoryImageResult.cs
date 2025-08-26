namespace Domain.Contracts.Resources.Images;

public sealed record HistoryImageResult(string Id, long Created, string CreatedBy, long Size, string Comment);

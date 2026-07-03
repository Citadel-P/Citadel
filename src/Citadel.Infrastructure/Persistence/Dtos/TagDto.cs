namespace Infrastructure.Persistence.Dtos;

internal sealed record TagDto(
    Guid Id,
    string Name,
    string NormalizedName,
    string Color,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt);

internal sealed record TagWithUsageDto(
    Guid Id,
    string Name,
    string NormalizedName,
    string Color,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    int UsageCount);

internal sealed record TagSummaryDto(Guid ResourceId, Guid Id, string Name, string Color);

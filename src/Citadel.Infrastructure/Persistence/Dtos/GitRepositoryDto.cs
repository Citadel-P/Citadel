namespace Infrastructure.Persistence.Dtos;

internal sealed record GitRepositoryDto(
    Guid Id,
    string Name,
    string Url,
    string DefaultBranch,
    Guid? GitAccountId,
    DateTime CreatedAt,
    Guid CreatedByActorId);

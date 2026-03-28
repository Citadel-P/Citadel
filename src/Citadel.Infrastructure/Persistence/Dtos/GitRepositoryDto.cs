namespace Infrastructure.Persistence.Dtos;

internal sealed record GitRepositoryDto(
    Guid Id,
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    string Status,
    Guid? GitAccountId,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    int WebHookEnabled,
    string? WebHookSecret,
    string? OnClone,
    string? OnPull);

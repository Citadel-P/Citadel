namespace Infrastructure.Persistence.Dtos;

internal sealed record GitAccountDto(
    Guid Id,
    string Name,
    string Domain,
    string Transport,
    string AuthType,
    DateTime CreatedAt,
    Guid CreatedByActorId,
    string Configuration);

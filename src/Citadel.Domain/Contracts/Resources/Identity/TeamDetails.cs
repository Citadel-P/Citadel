namespace Domain.Contracts.Resources.Identity;

public sealed record TeamDetails(
    Guid Id,
    string Name,
    Guid ActorId,
    bool IsEnabled);

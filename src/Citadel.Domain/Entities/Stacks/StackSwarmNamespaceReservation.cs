namespace Domain.Entities.Stacks;

public sealed record StackSwarmNamespaceReservation(
    Guid StackId,
    Guid PlatformId,
    string Namespace,
    DateTime CreatedAt);

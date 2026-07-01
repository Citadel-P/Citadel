using Domain.Entities.ResourceBindings;

namespace Domain.Contracts.Resources.ResourceBindings;

public sealed record ResourceBindingSnapshot(
    string Name,
    ResourceBindingKind Kind,
    ResourceBindingScope Scope,
    string? Value,
    Guid? SecretId,
    SecretDeliveryMode? SecretDeliveryMode,
    string? TargetPath);

public static class ResourceBindingSnapshotExtensions
{
    public static ResourceBindingSnapshot ToVariableSnapshot(this ResourceBinding entry, string value)
        => new(
            Name: entry.Name,
            Kind: entry.Kind,
            Scope: entry.Scope,
            Value: value,
            SecretId: null,
            SecretDeliveryMode: null,
            TargetPath: null);

    public static ResourceBindingSnapshot ToSecretSnapshot(
        this ResourceBinding entry,
        SecretDefinition secret,
        SecretProvider? provider)
        => new(
            Name: entry.Name,
            Kind: entry.Kind,
            Scope: entry.Scope,
            Value: "********",
            SecretId: secret.Id,
            SecretDeliveryMode: entry.SecretDeliveryMode,
            TargetPath: entry.TargetPath);
}

using Domain.Entities.Configuration;

namespace Domain.Contracts.Resources.Configuration;

public sealed record ConfigurationSnapshotEntry(
    string Name,
    ConfigurationEntryKind Kind,
    ConfigurationScope Scope,
    Guid? ResourceId,
    string? Value,
    Guid? SecretId,
    string? SecretName,
    SecretProviderType? SecretProviderType,
    string? SecretProviderName,
    string? ExternalPath,
    string? ExternalKey,
    int? ExternalVersion,
    SecretDeliveryMode? SecretDeliveryMode,
    string? TargetPath);

public static class ConfigurationSnapshotExtensions
{
    public static ConfigurationSnapshotEntry ToVariableSnapshot(this ConfigurationEntry entry, string value)
        => new(
            Name: entry.Name,
            Kind: entry.Kind,
            Scope: entry.Scope,
            ResourceId: entry.ResourceId,
            Value: value,
            SecretId: null,
            SecretName: null,
            SecretProviderType: null,
            SecretProviderName: null,
            ExternalPath: null,
            ExternalKey: null,
            ExternalVersion: null,
            SecretDeliveryMode: null,
            TargetPath: null);

    public static ConfigurationSnapshotEntry ToSecretSnapshot(
        this ConfigurationEntry entry,
        SecretDefinition secret,
        SecretProvider? provider)
        => new(
            Name: entry.Name,
            Kind: entry.Kind,
            Scope: entry.Scope,
            ResourceId: entry.ResourceId,
            Value: "********",
            SecretId: secret.Id,
            SecretName: secret.Name,
            SecretProviderType: secret.ProviderType,
            SecretProviderName: provider?.Name,
            ExternalPath: secret.ExternalPath,
            ExternalKey: secret.ExternalKey,
            ExternalVersion: secret.ExternalVersion,
            SecretDeliveryMode: entry.SecretDeliveryMode,
            TargetPath: entry.TargetPath);
}

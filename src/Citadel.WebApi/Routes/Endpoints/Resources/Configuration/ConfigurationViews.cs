using Application.Features.Configuration.Models;
using Domain.Entities.Configuration;

namespace WebApi.Routes.Endpoints.Resources.Configuration;

public sealed record ConfigurationEntryView(
    Guid Id,
    string Name,
    ConfigurationEntryKind Kind,
    ConfigurationScope Scope,
    Guid? ResourceId,
    string? Value,
    Guid? SecretId,
    SecretDeliveryMode? SecretDeliveryMode,
    string? TargetPath,
    bool IsInherited);

public sealed record ConfigurationEntriesView(
    IReadOnlyList<ConfigurationEntryView> Entries,
    IReadOnlyList<ConfigurationEntryView> EffectiveEntries)
{
    internal static ConfigurationEntriesView Map(ConfigurationEntriesResult result)
    {
        var localIds = result.Entries.Select(x => x.Id).ToHashSet();
        return new ConfigurationEntriesView(
            [.. result.Entries.Select(x => Map(x, isInherited: false))],
            [.. result.EffectiveEntries.Select(x => Map(x, isInherited: !localIds.Contains(x.Id)))]);
    }

    private static ConfigurationEntryView Map(ConfigurationEntry entry, bool isInherited) => new(
        Id: entry.Id,
        Name: entry.Name,
        Kind: entry.Kind,
        Scope: entry.Scope,
        ResourceId: entry.ResourceId,
        Value: entry.Kind == ConfigurationEntryKind.Variable ? entry.Value : null,
        SecretId: entry.SecretId,
        SecretDeliveryMode: entry.SecretDeliveryMode,
        TargetPath: entry.TargetPath,
        IsInherited: isInherited);
}

public sealed record ReplaceConfigurationEntriesInput(IReadOnlyList<ConfigurationEntryInput> Entries);

public sealed record SecretDefinitionView(
    Guid Id,
    string Name,
    SecretProviderType ProviderType,
    Guid? ProviderId,
    string? ExternalPath,
    string? ExternalKey,
    int? ExternalVersion,
    DateTime CreatedAt)
{
    internal static SecretDefinitionView Map(SecretDefinition secret) => new(
        secret.Id,
        secret.Name,
        secret.ProviderType,
        secret.ProviderId,
        secret.ExternalPath,
        secret.ExternalKey,
        secret.ExternalVersion,
        secret.CreatedAt);
}

public sealed record SecretDefinitionsView(IReadOnlyList<SecretDefinitionView> Secrets)
{
    internal static SecretDefinitionsView Map(IReadOnlyList<SecretDefinition> secrets)
        => new([.. secrets.Select(SecretDefinitionView.Map)]);
}

public sealed record CreateInternalSecretInput(string Name, string Value);

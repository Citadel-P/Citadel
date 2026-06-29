using Application.Features.Configuration.Commands;
using Application.Features.Configuration.Models;
using Domain.Entities.Configuration;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

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
    IReadOnlyList<ConfigurationEntryView> EffectiveEntries,
    ResourceCapabilities? Capabilities = null)
{
    internal static ConfigurationEntriesView Map(ConfigurationEntriesResult result, PermissionMetadata? permissions = null)
    {
        var localIds = result.Entries.Select(x => x.Id).ToHashSet();
        return new ConfigurationEntriesView(
            [.. result.Entries.Select(x => Map(x, isInherited: false))],
            [.. result.EffectiveEntries.Select(x => Map(x, isInherited: !localIds.Contains(x.Id)))],
            permissions is null ? null : CapabilityMapper.ToResourceCapabilities(permissions.Value));
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

public sealed record SecretDefinitionsView(
    IReadOnlyList<SecretDefinitionView> Secrets,
    ResourceCapabilities Capabilities)
{
    internal static SecretDefinitionsView Map(IReadOnlyList<SecretDefinition> secrets, PermissionMetadata configurationPermissions)
        => new(
            [.. secrets.Select(SecretDefinitionView.Map)],
            CapabilityMapper.ToResourceCapabilities(configurationPermissions));
}

public sealed record CreateInternalSecretInput(string Name, string Value);

public sealed record CreateExternalSecretInput(
    string Name,
    Guid ProviderId,
    string ExternalPath,
    string ExternalKey,
    int? ExternalVersion);

public sealed record TestExternalSecretInput(
    Guid ProviderId,
    string ExternalPath,
    string ExternalKey,
    int? ExternalVersion);

public sealed record ExternalSecretTestResultView(bool Success, string Message)
{
    internal static ExternalSecretTestResultView Map(ExternalSecretTestResult result)
        => new(result.Success, result.Message);
}

public sealed record SecretProviderView(
    Guid Id,
    string Name,
    SecretProviderType ProviderType,
    string Address,
    string MountPath,
    DateTime CreatedAt)
{
    internal static SecretProviderView Map(SecretProvider provider) => new(
        provider.Id,
        provider.Name,
        provider.ProviderType,
        provider.Configuration.Address,
        provider.Configuration.MountPath,
        provider.CreatedAt);
}

public sealed record SecretProvidersView(IReadOnlyList<SecretProviderView> Providers)
{
    internal static SecretProvidersView Map(IReadOnlyList<SecretProvider> providers)
        => new([.. providers.Select(SecretProviderView.Map)]);
}

public sealed record CreateVaultKvV2SecretProviderInput(
    string Name,
    string Address,
    string MountPath,
    string Token);

using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class ConfigurationMappers
{
    internal static ResourceBinding ToDomain(this ResourceBindingDto dto)
        => new(
            Name: dto.Name,
            Kind: Enum.Parse<ResourceBindingKind>(dto.Kind),
            Scope: Enum.Parse<ResourceBindingScope>(dto.Scope),
            ResourceId: dto.ResourceId,
            Value: dto.Value,
            SecretId: dto.SecretId,
            SecretDeliveryMode: string.IsNullOrWhiteSpace(dto.SecretDeliveryMode)
                ? null
                : Enum.Parse<SecretDeliveryMode>(dto.SecretDeliveryMode),
            TargetPath: dto.TargetPath)
        {
            Id = dto.Id,
            CreatedAt = dto.CreatedAt,
            UpdatedAt = dto.UpdatedAt
        };

    internal static SecretDefinition ToDomain(this SecretDefinitionDto dto)
        => new(
            Name: dto.Name,
            ProviderType: Enum.Parse<SecretProviderType>(dto.ProviderType),
            ProviderId: dto.ProviderId,
            ExternalPath: dto.ExternalPath,
            ExternalKey: dto.ExternalKey,
            ExternalVersion: dto.ExternalVersion)
        {
            Id = dto.Id,
            CreatedAt = dto.CreatedAt,
            UpdatedAt = dto.UpdatedAt
        };

    internal static InternalSecretValue ToDomain(this InternalSecretValueDto dto)
        => new(dto.SecretId, dto.EncryptedValue)
        {
            CreatedAt = dto.CreatedAt,
            UpdatedAt = dto.UpdatedAt
        };

    internal static SecretProvider ToDomain(this SecretProviderDto dto)
    {
        var configuration = JsonSerializer.Deserialize(
            dto.Configuration,
            ConfigurationJsonContext.Default.VaultKvV2SecretProviderConfiguration)
            ?? throw new InvalidOperationException("Secret provider configuration is invalid.");

        return new SecretProvider(
            Name: dto.Name,
            ProviderType: Enum.Parse<SecretProviderType>(dto.ProviderType),
            Configuration: configuration)
        {
            Id = dto.Id,
            CreatedAt = dto.CreatedAt,
            UpdatedAt = dto.UpdatedAt
        };
    }

    internal static SecretResolutionMaterial ToDomain(this SecretResolutionMaterialDto dto)
    {
        var definition = new SecretDefinition(
            Name: dto.Name,
            ProviderType: Enum.Parse<SecretProviderType>(dto.ProviderType),
            ProviderId: dto.ProviderId,
            ExternalPath: dto.ExternalPath,
            ExternalKey: dto.ExternalKey,
            ExternalVersion: dto.ExternalVersion)
        {
            Id = dto.Id,
            CreatedAt = dto.CreatedAt,
            UpdatedAt = dto.UpdatedAt
        };

        InternalSecretValue? value = null;
        if (dto.InternalEncryptedValue is not null)
        {
            value = new InternalSecretValue(dto.Id, dto.InternalEncryptedValue)
            {
                CreatedAt = dto.InternalValueCreatedAt ?? dto.CreatedAt,
                UpdatedAt = dto.InternalValueUpdatedAt ?? dto.UpdatedAt
            };
        }

        SecretProvider? provider = null;
        if (dto.ExternalProviderId.HasValue
            && dto.ExternalProviderName is not null
            && dto.ExternalProviderType is not null
            && dto.ExternalProviderConfiguration is not null)
        {
            var configuration = JsonSerializer.Deserialize(
                dto.ExternalProviderConfiguration,
                ConfigurationJsonContext.Default.VaultKvV2SecretProviderConfiguration)
                ?? throw new InvalidOperationException("Secret provider configuration is invalid.");

            provider = new SecretProvider(
                Name: dto.ExternalProviderName,
                ProviderType: Enum.Parse<SecretProviderType>(dto.ExternalProviderType),
                Configuration: configuration)
            {
                Id = dto.ExternalProviderId.Value,
                CreatedAt = dto.ExternalProviderCreatedAt ?? dto.CreatedAt,
                UpdatedAt = dto.ExternalProviderUpdatedAt ?? dto.UpdatedAt
            };
        }

        return new SecretResolutionMaterial(definition, value, provider);
    }
}

namespace Infrastructure.Persistence.Dtos;

internal sealed record ConfigurationEntryDto(
    Guid Id,
    string Name,
    string Kind,
    string Scope,
    Guid? ResourceId,
    string? Value,
    Guid? SecretId,
    string? SecretDeliveryMode,
    string? TargetPath,
    DateTime CreatedAt,
    DateTime UpdatedAt);

internal sealed record SecretDefinitionDto(
    Guid Id,
    string Name,
    string ProviderType,
    Guid? ProviderId,
    string? ExternalPath,
    string? ExternalKey,
    int? ExternalVersion,
    DateTime CreatedAt,
    DateTime UpdatedAt);

internal sealed record SecretProviderDto(
    Guid Id,
    string Name,
    string ProviderType,
    string Configuration,
    DateTime CreatedAt,
    DateTime UpdatedAt);

internal sealed record InternalSecretValueDto(
    Guid SecretId,
    string EncryptedValue,
    DateTime CreatedAt,
    DateTime UpdatedAt);

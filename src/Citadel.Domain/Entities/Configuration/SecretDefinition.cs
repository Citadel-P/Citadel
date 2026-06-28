namespace Domain.Entities.Configuration;

public sealed record SecretDefinition(
    string Name,
    SecretProviderType ProviderType,
    Guid? ProviderId = null,
    string? ExternalPath = null,
    string? ExternalKey = null,
    int? ExternalVersion = null)
{
    public Guid Id { get; init; } = Guid.CreateVersion7();
    public DateTime CreatedAt { get; init; } = DateTime.UtcNow;
    public DateTime UpdatedAt { get; init; } = DateTime.UtcNow;

    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(Name))
            throw new ArgumentException("Secret name is required.", nameof(Name));

        if (ProviderType == SecretProviderType.InternalEncrypted)
        {
            if (ProviderId is not null || ExternalPath is not null || ExternalKey is not null || ExternalVersion is not null)
                throw new ArgumentException("Internal encrypted secrets cannot define external provider metadata.");
        }
    }
}

public sealed record InternalSecretValue(Guid SecretId, string EncryptedValue)
{
    public DateTime CreatedAt { get; init; } = DateTime.UtcNow;
    public DateTime UpdatedAt { get; init; } = DateTime.UtcNow;
}

public enum SecretProviderType
{
    InternalEncrypted,
    VaultCompatibleKvV2
}

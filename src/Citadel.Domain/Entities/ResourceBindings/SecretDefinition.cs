namespace Domain.Entities.ResourceBindings;

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
        else if (ProviderType == SecretProviderType.VaultCompatibleKvV2)
        {
            if (ProviderId is null)
                throw new ArgumentException("External secrets require a provider.");
            if (string.IsNullOrWhiteSpace(ExternalPath))
                throw new ArgumentException("External secrets require a provider path.");
            if (string.IsNullOrWhiteSpace(ExternalKey))
                throw new ArgumentException("External secrets require a provider key.");
            if (ExternalVersion is <= 0)
                throw new ArgumentException("External secret version must be greater than zero.");
        }
    }
}

public sealed record InternalSecretValue(Guid SecretId, string EncryptedValue)
{
    public DateTime CreatedAt { get; init; } = DateTime.UtcNow;
    public DateTime UpdatedAt { get; init; } = DateTime.UtcNow;
}


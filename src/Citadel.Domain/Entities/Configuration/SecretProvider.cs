namespace Domain.Entities.Configuration;

public sealed record SecretProvider(
    string Name,
    SecretProviderType ProviderType,
    VaultKvV2SecretProviderConfiguration Configuration)
{
    public Guid Id { get; init; } = Guid.CreateVersion7();
    public DateTime CreatedAt { get; init; } = DateTime.UtcNow;
    public DateTime UpdatedAt { get; init; } = DateTime.UtcNow;

    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(Name))
            throw new ArgumentException("Secret provider name is required.", nameof(Name));
        if (ProviderType != SecretProviderType.VaultCompatibleKvV2)
            throw new ArgumentException("Only Vault-compatible KV v2 providers are supported.");
        if (string.IsNullOrWhiteSpace(Configuration.Address))
            throw new ArgumentException("Vault address is required.");
        if (!Uri.TryCreate(Configuration.Address, UriKind.Absolute, out var uri) ||
            (uri.Scheme != Uri.UriSchemeHttp && uri.Scheme != Uri.UriSchemeHttps))
            throw new ArgumentException("Vault address must be an absolute HTTP or HTTPS URL.");
        if (string.IsNullOrWhiteSpace(Configuration.MountPath))
            throw new ArgumentException("Vault mount path is required.");
        if (string.IsNullOrWhiteSpace(Configuration.ProtectedToken))
            throw new ArgumentException("Vault token is required.");
    }
}

public sealed record VaultKvV2SecretProviderConfiguration(
    string Address,
    string MountPath,
    string ProtectedToken);

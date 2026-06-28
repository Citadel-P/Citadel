namespace Domain.Entities.Configuration;

public sealed record ConfigurationEntry(
    string Name,
    ConfigurationEntryKind Kind,
    ConfigurationScope Scope,
    Guid? ResourceId,
    string? Value,
    Guid? SecretId,
    SecretDeliveryMode? SecretDeliveryMode = null,
    string? TargetPath = null)
{
    public Guid Id { get; init; } = Guid.CreateVersion7();
    public DateTime CreatedAt { get; init; } = DateTime.UtcNow;
    public DateTime UpdatedAt { get; init; } = DateTime.UtcNow;

    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(Name))
            throw new ArgumentException("Configuration entry name is required.", nameof(Name));

        if (Scope != ConfigurationScope.Global && ResourceId is null)
            throw new ArgumentException("Resource scoped configuration entries require a resource id.", nameof(ResourceId));

        if (Scope == ConfigurationScope.Global && ResourceId is not null)
            throw new ArgumentException("Global configuration entries cannot have a resource id.", nameof(ResourceId));

        if (Kind == ConfigurationEntryKind.Variable)
        {
            if (Value is null)
                throw new ArgumentException("Variable entries require a value.", nameof(Value));

            if (SecretId is not null || SecretDeliveryMode is not null || TargetPath is not null)
                throw new ArgumentException("Variable entries cannot reference secret delivery metadata.");
        }
        else
        {
            if (Value is not null)
                throw new ArgumentException("Secret entries cannot store plaintext values.", nameof(Value));

            if (SecretId is null)
                throw new ArgumentException("Secret entries require a secret id.", nameof(SecretId));

            if (SecretDeliveryMode is null)
                throw new ArgumentException("Secret entries require a delivery mode.", nameof(SecretDeliveryMode));
        }
    }
}

public enum ConfigurationEntryKind
{
    Variable,
    Secret
}

public enum ConfigurationScope
{
    Global,
    Stack,
    Deployment
}

public enum SecretDeliveryMode
{
    EnvironmentVariable,
    MountedFile,
    NativePlatformSecret
}

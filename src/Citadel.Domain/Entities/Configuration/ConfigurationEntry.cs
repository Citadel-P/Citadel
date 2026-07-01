using SecretMode = Domain.SecretDeliveryMode;

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

            if (SecretDeliveryMode == SecretMode.EnvironmentVariable && TargetPath is not null)
                throw new ArgumentException("Environment variable secrets cannot define a target path.", nameof(TargetPath));

            if (SecretDeliveryMode == SecretMode.MountedFile)
                ValidateMountedFileTargetPath(TargetPath);

            if (SecretDeliveryMode == SecretMode.NativePlatformSecret)
                throw new ArgumentException("Native platform secret delivery is not supported yet.", nameof(SecretDeliveryMode));
        }
    }

    private static void ValidateMountedFileTargetPath(string? targetPath)
    {
        if (string.IsNullOrWhiteSpace(targetPath))
            throw new ArgumentException("Mounted file secrets require a target path.", nameof(TargetPath));

        if (!targetPath.StartsWith("/", StringComparison.Ordinal) || targetPath.Contains("\\", StringComparison.Ordinal))
            throw new ArgumentException("Mounted file secret target path must be an absolute Linux container path.", nameof(TargetPath));

        var normalized = targetPath.Trim();
        if (normalized == "/" || normalized.EndsWith("/", StringComparison.Ordinal))
            throw new ArgumentException("Mounted file secret target path must point to a file.", nameof(TargetPath));

        var segments = normalized.Split('/', StringSplitOptions.RemoveEmptyEntries);
        if (segments.Any(segment => segment is "." or ".."))
            throw new ArgumentException("Mounted file secret target path cannot contain relative path segments.", nameof(TargetPath));

        if (normalized.Equals("/etc/passwd", StringComparison.Ordinal)
            || normalized.Equals("/etc/shadow", StringComparison.Ordinal)
            || normalized.StartsWith("/proc/", StringComparison.Ordinal)
            || normalized.StartsWith("/sys/", StringComparison.Ordinal)
            || normalized.StartsWith("/dev/", StringComparison.Ordinal))
        {
            throw new ArgumentException("Mounted file secret target path uses a protected container path.", nameof(TargetPath));
        }
    }
}

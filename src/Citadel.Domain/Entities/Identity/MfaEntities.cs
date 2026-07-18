namespace Domain.Entities.Identity;

public sealed record UserMfaSettings(
    Guid UserId,
    string ProtectedTotpSecret,
    long? LastAcceptedTimeStep,
    DateTime EnabledAt,
    DateTime CreatedAt)
{
    public void Validate()
    {
        if (UserId == Guid.Empty)
            throw new ArgumentException("User id is required.", nameof(UserId));
        if (string.IsNullOrWhiteSpace(ProtectedTotpSecret))
            throw new ArgumentException("Protected TOTP secret is required.", nameof(ProtectedTotpSecret));
    }
}

public sealed record UserMfaRecoveryCode(
    Guid Id,
    Guid UserId,
    string CodeHash,
    DateTime? UsedAt,
    DateTime CreatedAt)
{
    public void Validate()
    {
        if (Id == Guid.Empty)
            throw new ArgumentException("Recovery code id is required.", nameof(Id));
        if (UserId == Guid.Empty)
            throw new ArgumentException("User id is required.", nameof(UserId));
        if (string.IsNullOrWhiteSpace(CodeHash))
            throw new ArgumentException("Recovery code hash is required.", nameof(CodeHash));
    }
}

public sealed record MfaSetupSession(
    Guid Id,
    Guid UserId,
    string ProtectedTotpSecret,
    DateTime ExpiresAt,
    DateTime? ConsumedAt,
    DateTime CreatedAt)
{
    public bool IsActive(DateTime now) => ConsumedAt is null && ExpiresAt > now;

    public void Validate()
    {
        if (Id == Guid.Empty)
            throw new ArgumentException("Setup session id is required.", nameof(Id));
        if (UserId == Guid.Empty)
            throw new ArgumentException("User id is required.", nameof(UserId));
        if (string.IsNullOrWhiteSpace(ProtectedTotpSecret))
            throw new ArgumentException("Protected TOTP secret is required.", nameof(ProtectedTotpSecret));
        if (ExpiresAt <= CreatedAt)
            throw new ArgumentException("Setup session expiry must be after creation.", nameof(ExpiresAt));
    }
}

public sealed record MfaChallenge(
    Guid Id,
    Guid UserId,
    DateTime ExpiresAt,
    int FailedAttempts,
    DateTime? ConsumedAt,
    DateTime CreatedAt)
{
    public bool IsActive(DateTime now, int maxFailedAttempts)
        => ConsumedAt is null && ExpiresAt > now && FailedAttempts < maxFailedAttempts;

    public void Validate()
    {
        if (Id == Guid.Empty)
            throw new ArgumentException("Challenge id is required.", nameof(Id));
        if (UserId == Guid.Empty)
            throw new ArgumentException("User id is required.", nameof(UserId));
        if (ExpiresAt <= CreatedAt)
            throw new ArgumentException("Challenge expiry must be after creation.", nameof(ExpiresAt));
        if (FailedAttempts < 0)
            throw new ArgumentException("Failed attempts cannot be negative.", nameof(FailedAttempts));
    }
}

namespace Domain.Contracts.Resources.Oidc;

public sealed record OidcLoginState(
    Guid Id,
    Guid ProviderId,
    string StateHash,
    string Nonce,
    string CodeVerifier,
    string ReturnUrl,
    DateTime CreatedAt,
    DateTime ExpiresAt);

public sealed record OidcExternalLogin(
    Guid Id,
    Guid ProviderId,
    string Subject,
    Guid UserId,
    string? Email,
    DateTime CreatedAt,
    DateTime UpdatedAt);

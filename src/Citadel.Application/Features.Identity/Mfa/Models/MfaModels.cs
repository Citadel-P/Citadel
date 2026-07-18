using Domain;

namespace Application.Features.Identity.Mfa.Models;

public sealed record TotpSetup(string Secret, string OtpAuthUri, DateTime ExpiresAt);

public sealed record MfaSetupResult(string Secret, string OtpAuthUri, DateTime ExpiresAt);

public sealed record MfaStatusResult(
    bool Enabled,
    int RemainingRecoveryCodes,
    MfaPolicy Policy,
    bool CanDisable);

public sealed record MfaRecoveryCodesResult(bool Enabled, IReadOnlyList<string> RecoveryCodes);

public sealed record MandatoryMfaSetupResult(string Secret, string OtpAuthUri, DateTime ExpiresAt);

public sealed record MandatoryMfaSetupCompleteResult(string AccessToken, IReadOnlyList<string> RecoveryCodes);

public sealed record MfaVerificationResult(string AccessToken);

public sealed record MfaVerificationInput(string? Code = null, string? RecoveryCode = null);

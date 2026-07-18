using Application.Features.Identity.Mfa.Commands;
using Application.Features.Identity.Mfa.Models;
using Domain;

namespace WebApi.Routes.Endpoints.Resources.Identity.Mfa;

public sealed record MfaVerificationInput(string? Code = null, string? RecoveryCode = null)
{
    internal VerifyMfaChallenge ToVerifyCommand() => new(Code, RecoveryCode);
}

public sealed record ConfirmMandatoryMfaSetupInput(string Code)
{
    internal ConfirmMandatoryMfaSetup ToCommand() => new(Code);
}

public sealed record StartProfileMfaSetupInput(string Password)
{
    internal StartProfileMfaSetup ToCommand() => new(Password);
}

public sealed record ConfirmProfileMfaSetupInput(string Code)
{
    internal ConfirmProfileMfaSetup ToCommand() => new(Code);
}

public sealed record DisableProfileMfaInput(string Password, string? Code = null, string? RecoveryCode = null)
{
    internal DisableProfileMfa ToCommand() => new(Password, Code, RecoveryCode);
}

public sealed record RegenerateProfileMfaRecoveryCodesInput(string Password, string Code)
{
    internal RegenerateProfileMfaRecoveryCodes ToCommand() => new(Password, Code);
}

public sealed record MandatoryMfaSetupView(string Secret, string OtpAuthUri, DateTime ExpiresAt)
{
    internal static MandatoryMfaSetupView Map(MandatoryMfaSetupResult result)
        => new(result.Secret, result.OtpAuthUri, result.ExpiresAt);
}

public sealed record MandatoryMfaSetupCompleteView(string AccessToken, IReadOnlyList<string> RecoveryCodes)
{
    internal static MandatoryMfaSetupCompleteView Map(MandatoryMfaSetupCompleteResult result)
        => new(result.AccessToken, result.RecoveryCodes);
}

public sealed record MfaVerificationView(string AccessToken)
{
    internal static MfaVerificationView Map(MfaVerificationResult result)
        => new(result.AccessToken);
}

public sealed record ProfileMfaSetupView(string Secret, string OtpAuthUri, DateTime ExpiresAt)
{
    internal static ProfileMfaSetupView Map(MfaSetupResult result)
        => new(result.Secret, result.OtpAuthUri, result.ExpiresAt);
}

public sealed record ProfileMfaStatusView(bool Enabled, int RemainingRecoveryCodes, MfaPolicy Policy, bool CanDisable)
{
    internal static ProfileMfaStatusView Map(MfaStatusResult result)
        => new(result.Enabled, result.RemainingRecoveryCodes, result.Policy, result.CanDisable);
}

public sealed record ProfileMfaRecoveryCodesView(bool Enabled, IReadOnlyList<string> RecoveryCodes)
{
    internal static ProfileMfaRecoveryCodesView Map(MfaRecoveryCodesResult result)
        => new(result.Enabled, result.RecoveryCodes);
}

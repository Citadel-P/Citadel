using System.Security.Cryptography;
using System.Text;
using Application.Configs;
using Application.Features.Identity.Mfa.Models;
using Application.Services;
using Application.Services.Identity;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using Microsoft.Extensions.Options;
using OtpNet;

namespace Application.Features.Identity.Mfa.Services;

public interface ITotpService
{
    TotpSetup CreateSetup(string issuer, string accountName, DateTime expiresAt);
    bool TryVerify(byte[] secret, string code, out long matchedTimeStep);
    string BuildOtpAuthUri(string issuer, string accountName, string secret);
}

public interface IRecoveryCodeService
{
    IReadOnlyList<string> Generate(int count);
    string Normalize(string code);
    string Hash(string normalizedCode);
    bool Verify(string normalizedCode, string expectedHash);
}

public interface IMfaPolicyService
{
    bool RequiresMfa(UserAuthInfo user);
    bool CanDisable(UserAuthInfo user);
    MfaPolicy CurrentPolicy { get; }
}

public interface IAuthenticationSessionIssuer
{
    Task<string> IssueAsync(UserAuthInfo user, CancellationToken cancellationToken);
}

internal sealed class TotpService : ITotpService
{
    private const int SecretLength = 20;
    private const int Period = 30;
    private const int Digits = 6;

    public TotpSetup CreateSetup(string issuer, string accountName, DateTime expiresAt)
    {
        var secret = Base32Encoding.ToString(KeyGeneration.GenerateRandomKey(SecretLength));
        return new TotpSetup(secret, BuildOtpAuthUri(issuer, accountName, secret), expiresAt);
    }

    public bool TryVerify(byte[] secret, string code, out long matchedTimeStep)
    {
        matchedTimeStep = 0;
        if (string.IsNullOrWhiteSpace(code) || code.Length != Digits || code.Any(ch => !char.IsDigit(ch)))
            return false;

        var totp = new Totp(secret, step: Period, mode: OtpHashMode.Sha1, totpSize: Digits);
        return totp.VerifyTotp(
            code,
            out matchedTimeStep,
            new VerificationWindow(previous: 1, future: 1));
    }

    public string BuildOtpAuthUri(string issuer, string accountName, string secret)
    {
        var escapedIssuer = Uri.EscapeDataString(issuer);
        var escapedAccountName = Uri.EscapeDataString(accountName);
        return $"otpauth://totp/{escapedIssuer}:{escapedAccountName}?secret={secret}&issuer={escapedIssuer}&algorithm=SHA1&digits={Digits}&period={Period}";
    }
}

internal sealed class RecoveryCodeService(IOptions<SecretsConfiguration> secretsOptions) : IRecoveryCodeService
{
    private const string Alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    private const string HmacPurpose = "citadel:mfa:recovery-code:v1";

    public IReadOnlyList<string> Generate(int count)
    {
        var codes = new List<string>(count);
        for (var i = 0; i < count; i++)
        {
            var chars = new char[12];
            var random = new byte[12];
            RandomNumberGenerator.Fill(random);

            for (var j = 0; j < chars.Length; j++)
                chars[j] = Alphabet[random[j] % Alphabet.Length];

            codes.Add($"{new string(chars.AsSpan(0, 4))}-{new string(chars.AsSpan(4, 4))}-{new string(chars.AsSpan(8, 4))}");
        }

        return codes;
    }

    public string Normalize(string code)
        => new(code
            .Trim()
            .Where(ch => ch is not '-' && !char.IsWhiteSpace(ch))
            .Select(char.ToUpperInvariant)
            .ToArray());

    public string Hash(string normalizedCode)
    {
        using var hmac = new HMACSHA256(GetPurposeKey());
        return Convert.ToBase64String(hmac.ComputeHash(Encoding.UTF8.GetBytes(normalizedCode)));
    }

    public bool Verify(string normalizedCode, string expectedHash)
    {
        var actual = Convert.FromBase64String(Hash(normalizedCode));
        var expected = Convert.FromBase64String(expectedHash);
        return CryptographicOperations.FixedTimeEquals(actual, expected);
    }

    private byte[] GetPurposeKey()
    {
        using var hmac = new HMACSHA256(secretsOptions.Value.GetEncryptionKey());
        return hmac.ComputeHash(Encoding.UTF8.GetBytes(HmacPurpose));
    }
}

internal sealed class MfaPolicyService(IOptions<MfaOptions> options) : IMfaPolicyService
{
    private const string AdminRoleName = "Admin";

    public MfaPolicy CurrentPolicy => options.Value.Policy;

    public bool RequiresMfa(UserAuthInfo user)
        => CurrentPolicy switch
        {
            MfaPolicy.RequiredForAllUsers => HasLocalPassword(user),
            MfaPolicy.RequiredForAdministrators => HasLocalPassword(user)
                && user.Roles.Any(role => string.Equals(role, AdminRoleName, StringComparison.OrdinalIgnoreCase)),
            _ => false
        };

    public bool CanDisable(UserAuthInfo user) => !RequiresMfa(user);

    private static bool HasLocalPassword(UserAuthInfo user) => !string.IsNullOrWhiteSpace(user.Password);
}

internal sealed class AuthenticationSessionIssuer(
    IUnitOfWork unitOfWork,
    IJwtService jwtService,
    IRoleCache roleCache,
    IRequestSessionMetadataAccessor requestSessionMetadataAccessor,
    IRefreshTokenCookieService refreshTokenCookieService)
    : IAuthenticationSessionIssuer
{
    public async Task<string> IssueAsync(UserAuthInfo user, CancellationToken cancellationToken)
    {
        var accessToken = jwtService.CreateAccessToken(User.GetJwtClaims(user));
        var (refreshTokenId, refreshToken, refreshTokenExpiresAt) = jwtService.CreateRefreshToken();
        var metadata = requestSessionMetadataAccessor.GetCurrent();

        await unitOfWork.RefreshTokens.AddAsync(
            Domain.Entities.Identity.RefreshToken.Create(
                refreshTokenId,
                user.Id,
                refreshTokenExpiresAt,
                metadata.UserAgent,
                metadata.IpAddress),
            cancellationToken);

        var tokensCount = await unitOfWork.RefreshTokens.CountAsync(user.Id, cancellationToken);
        const int maxTokensPerUser = 10;
        if (tokensCount > maxTokensPerUser)
            await unitOfWork.RefreshTokens.DeleteOldestTokensAsync(user.Id, tokensCount - maxTokensPerUser, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        refreshTokenCookieService.Set(refreshToken, refreshTokenExpiresAt);
        roleCache.SetRoles(user.Id, user.Roles);

        return accessToken;
    }
}
